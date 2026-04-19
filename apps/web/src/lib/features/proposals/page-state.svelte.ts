import { shipyardApi } from '$lib/api/client';
import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
import { ensureClientNdk } from '$lib/ndk/client';
import { signNostrEventWithNdk } from '$lib/nostr/signing';
import type {
  ApiErrorBody,
  BatchSignProposalItem,
  PublishItem,
  Queue
} from '$lib/api/types';

type ReviewPageState = {
  session: ShipyardSession;
  proposals: PublishItem[];
  queues: Queue[];
  selected: Set<string>;
  rejectingId: string | null;
  rejectReason: string;
  loading: boolean;
  saving: boolean;
  message: string;
  error: string;
};

function formatDate(value: string | null) {
  if (!value) return 'Unscheduled';
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(value));
}

function postContent(item: PublishItem) {
  const event = item.unsigned_event_json ?? item.signed_event_json;
  const content = event?.content;
  return typeof content === 'string' && content.trim() ? content : 'Untitled post';
}

export function createProposalsPageState() {
  const state = $state<ReviewPageState>({
    session: { token: '', ownerPubkey: '' },
    proposals: [],
    queues: [],
    selected: new Set(),
    rejectingId: null,
    rejectReason: '',
    loading: true,
    saving: false,
    message: '',
    error: ''
  });

  function queueName(queueId: string | null) {
    if (!queueId) return null;
    return state.queues.find((queue) => queue.id === queueId)?.name ?? null;
  }

  function whenLabel(item: PublishItem) {
    if (item.queue_id) {
      const name = queueName(item.queue_id);
      return name ? `From queue: ${name}` : 'From queue';
    }
    if (item.publish_time) {
      return `Scheduled for ${formatDate(item.publish_time)}`;
    }
    return 'Ready to publish';
  }

  function setError(err: unknown, fallback: string) {
    state.error = (err as ApiErrorBody).message ?? fallback;
    state.message = '';
  }

  function setMessage(value: string) {
    state.message = value;
    state.error = '';
  }

  function toggle(proposalId: string) {
    const next = new Set(state.selected);
    if (next.has(proposalId)) {
      next.delete(proposalId);
    } else {
      next.add(proposalId);
    }
    state.selected = next;
  }

  function clearSelection() {
    state.selected = new Set();
  }

  function startReject(proposalId: string) {
    state.rejectingId = proposalId;
    state.rejectReason = '';
  }

  function cancelReject() {
    state.rejectingId = null;
    state.rejectReason = '';
  }

  async function load() {
    state.session = readShipyardSession();
    state.loading = true;

    try {
      if (!state.session.token || !state.session.ownerPubkey) {
        state.proposals = [];
        state.queues = [];
        return;
      }

      const [proposalResponse, queueResponse] = await Promise.all([
        shipyardApi.proposals(state.session.token, state.session.ownerPubkey),
        shipyardApi.queues(state.session.token, state.session.ownerPubkey)
      ]);
      state.proposals = proposalResponse;
      state.queues = queueResponse;
      state.selected = new Set(
        [...state.selected].filter((id) => state.proposals.some((p) => p.id === id))
      );
    } catch (err) {
      setError(err, "Couldn't load posts for review.");
    } finally {
      state.loading = false;
    }
  }

  async function approve(proposal: PublishItem) {
    if (!proposal.unsigned_event_json) {
      setError(null, "This post doesn't have content to approve.");
      return;
    }

    state.saving = true;
    try {
      await ensureClientNdk();
      const signed = await signNostrEventWithNdk(
        proposal.unsigned_event_json as Parameters<typeof signNostrEventWithNdk>[0]
      );
      await shipyardApi.signProposal(state.session.token, proposal.id, signed);
      setMessage('Approved.');
      const next = new Set(state.selected);
      next.delete(proposal.id);
      state.selected = next;
      await load();
    } catch (err) {
      setError(err, "Couldn't approve. Try again?");
    } finally {
      state.saving = false;
    }
  }

  async function approveSelected() {
    const candidates = state.proposals.filter((proposal) => state.selected.has(proposal.id));
    const items: BatchSignProposalItem[] = [];

    state.saving = true;
    try {
      await ensureClientNdk();
      for (const proposal of candidates) {
        if (!proposal.unsigned_event_json) continue;
        const signed = await signNostrEventWithNdk(
          proposal.unsigned_event_json as Parameters<typeof signNostrEventWithNdk>[0]
        );
        items.push({ proposal_id: proposal.id, signed_event: signed });
      }
      if (!items.length) {
        setError(null, 'Nothing to approve.');
        return;
      }
      const response = await shipyardApi.batchSignProposals(state.session.token, items);
      const successes = response.results.filter((result) => result.item).length;
      const failures = response.results.length - successes;
      setMessage(
        failures
          ? `Approved ${successes}, ${failures} failed.`
          : `Approved ${successes} post${successes === 1 ? '' : 's'}.`
      );
      state.selected = new Set();
      await load();
    } catch (err) {
      setError(err, "Couldn't approve posts. Try again?");
    } finally {
      state.saving = false;
    }
  }

  async function rejectWithReason(proposalId: string) {
    state.saving = true;
    try {
      await shipyardApi.rejectProposal(state.session.token, proposalId, state.rejectReason);
      setMessage('Rejected.');
      state.rejectingId = null;
      state.rejectReason = '';
      await load();
    } catch (err) {
      setError(err, "Couldn't reject. Try again?");
    } finally {
      state.saving = false;
    }
  }

  async function remove(proposalId: string) {
    state.saving = true;
    try {
      await shipyardApi.deleteProposal(state.session.token, proposalId);
      setMessage('Removed.');
      await load();
    } catch (err) {
      setError(err, "Couldn't remove. Try again?");
    } finally {
      state.saving = false;
    }
  }

  return {
    state,
    approve,
    approveSelected,
    cancelReject,
    clearSelection,
    load,
    postContent,
    rejectWithReason,
    remove,
    startReject,
    toggle,
    whenLabel
  };
}
