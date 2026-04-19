import { shipyardApi } from '$lib/api/client';
import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
import type { ApiErrorBody, Queue, QueueNextSlotResponse } from '$lib/api/types';

type QueuePageState = {
  session: ShipyardSession;
  queues: Queue[];
  name: string;
  description: string;
  cadenceMinutes: number;
  startAt: string;
  editQueueId: string;
  editName: string;
  editDescription: string;
  editCadenceMinutes: number;
  editStartAt: string;
  nextSlot: QueueNextSlotResponse | null;
  loading: boolean;
  saving: boolean;
  message: string;
  error: string;
};

function toLocalInput(date: Date) {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function formatDate(value: string | null) {
  if (!value) return 'Not set';
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(value));
}

function cadenceLabel(seconds: number) {
  if (seconds % 86_400 === 0) {
    const days = seconds / 86_400;
    return days === 1 ? 'Once a day' : `Every ${days} days`;
  }
  if (seconds % 3_600 === 0) {
    const hours = seconds / 3_600;
    return hours === 1 ? 'Every hour' : `Every ${hours} hours`;
  }
  const minutes = Math.max(1, Math.round(seconds / 60));
  return minutes === 1 ? 'Every minute' : `Every ${minutes} minutes`;
}

export function createQueuesPageState() {
  const state = $state<QueuePageState>({
    session: { token: '', ownerPubkey: '' },
    queues: [],
    name: '',
    description: '',
    cadenceMinutes: 1440,
    startAt: toLocalInput(new Date(Date.now() + 60 * 60 * 1000)),
    editQueueId: '',
    editName: '',
    editDescription: '',
    editCadenceMinutes: 1440,
    editStartAt: '',
    nextSlot: null,
    loading: true,
    saving: false,
    message: '',
    error: ''
  });

  function selectedQueue() {
    return state.queues.find((queue) => queue.id === state.editQueueId);
  }

  function setError(err: unknown, fallback: string) {
    state.error = (err as ApiErrorBody).message ?? fallback;
    state.message = '';
  }

  async function loadQueues() {
    state.session = readShipyardSession();
    state.loading = true;
    state.error = '';

    try {
      if (!state.session.token || !state.session.ownerPubkey) {
        state.queues = [];
        return;
      }

      state.queues = await shipyardApi.queues(state.session.token, state.session.ownerPubkey);
    } catch (err) {
      setError(err, "Couldn't load queues.");
    } finally {
      state.loading = false;
    }
  }

  async function createQueue(event: SubmitEvent) {
    event.preventDefault();
    state.saving = true;

    try {
      await shipyardApi.createQueue(state.session.token, state.session.ownerPubkey, {
        name: state.name,
        description: state.description.trim() || null,
        cadence_seconds: Math.max(1, Math.round(state.cadenceMinutes * 60)),
        start_at: new Date(state.startAt).toISOString()
      });
      state.name = '';
      state.description = '';
      state.cadenceMinutes = 1440;
      state.startAt = toLocalInput(new Date(Date.now() + 60 * 60 * 1000));
      state.message = 'Queue created.';

      state.error = '';
      await loadQueues();
    } catch (err) {
      setError(err, "Couldn't create queue. Try again?");
    } finally {
      state.saving = false;
    }
  }

  function selectQueue(queue: Queue) {
    state.editQueueId = queue.id;
    state.editName = queue.name;
    state.editDescription = queue.description ?? '';
    state.editCadenceMinutes = Math.max(1, Math.round(queue.cadence_seconds / 60));
    state.editStartAt = toLocalInput(new Date(queue.start_at));
    state.nextSlot = null;
  }

  async function updateSelectedQueue(event: SubmitEvent) {
    event.preventDefault();
    const queue = selectedQueue();
    if (!queue) return;
    state.saving = true;

    try {
      await shipyardApi.updateQueue(state.session.token, queue.id, {
        name: state.editName,
        description: state.editDescription.trim() || null,
        cadence_seconds: Math.max(1, Math.round(state.editCadenceMinutes * 60)),
        start_at: new Date(state.editStartAt).toISOString()
      });
      state.message = 'Saved.';
      state.error = '';
      await loadQueues();
    } catch (err) {
      setError(err, "Couldn't save changes. Try again?");
    } finally {
      state.saving = false;
    }
  }

  async function loadNextSlot(queueId: string) {
    state.saving = true;
    try {
      state.nextSlot = await shipyardApi.nextQueueSlot(state.session.token, queueId);
      state.error = '';
    } catch (err) {
      setError(err, "Couldn't calculate the next slot.");
    } finally {
      state.saving = false;
    }
  }

  async function archiveQueue(queueId: string) {
    state.saving = true;
    try {
      await shipyardApi.archiveQueue(state.session.token, queueId);
      state.message = 'Queue archived.';
      state.error = '';
      await loadQueues();
      if (state.editQueueId === queueId) {
        state.editQueueId = '';
        state.nextSlot = null;
      }
    } catch (err) {
      setError(err, "Couldn't archive queue.");
    } finally {
      state.saving = false;
    }
  }

  return {
    state,
    get activeQueues() {
      return state.queues.filter((queue) => !queue.archived_at);
    },
    get archivedQueues() {
      return state.queues.filter((queue) => queue.archived_at);
    },
    get selectedQueue() {
      return selectedQueue();
    },
    archiveQueue,
    cadenceLabel,
    createQueue,
    formatDate,
    loadNextSlot,
    loadQueues,
    selectQueue,
    updateSelectedQueue
  };
}
