<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { compactPubkey, readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import type { ApiErrorBody, Queue, QueueNextSlotResponse } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let queues = $state<Queue[]>([]);
  let name = $state('');
  let description = $state('');
  let cadenceMinutes = $state(1440);
  let startAt = $state(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)));
  let editQueueId = $state('');
  let editName = $state('');
  let editDescription = $state('');
  let editCadenceMinutes = $state(1440);
  let editStartAt = $state('');
  let nextSlot = $state<QueueNextSlotResponse | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  let activeQueues = $derived(queues.filter((queue) => !queue.archived_at));
  let archivedQueues = $derived(queues.filter((queue) => queue.archived_at));
  let selectedQueue = $derived(queues.find((queue) => queue.id === editQueueId));

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
      return `${seconds / 86_400}d`;
    }
    if (seconds % 3_600 === 0) {
      return `${seconds / 3_600}h`;
    }
    return `${Math.round(seconds / 60)}m`;
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
    message = '';
  }

  async function loadQueues() {
    session = readShipyardSession();
    loading = true;
    error = '';

    try {
      if (!session.token || !session.ownerPubkey) {
        queues = [];
        return;
      }

      queues = await shipyardApi.queues(session.token, session.ownerPubkey);
    } catch (err) {
      setError(err, 'Failed to load queues.');
    } finally {
      loading = false;
    }
  }

  async function createQueue(event: SubmitEvent) {
    event.preventDefault();
    saving = true;

    try {
      await shipyardApi.createQueue(session.token, session.ownerPubkey, {
        name,
        description: description.trim() || null,
        cadence_seconds: Math.max(1, Math.round(cadenceMinutes * 60)),
        start_at: new Date(startAt).toISOString()
      });
      name = '';
      description = '';
      cadenceMinutes = 1440;
      startAt = toLocalInput(new Date(Date.now() + 60 * 60 * 1000));
      message = 'Queue created.';
      error = '';
      await loadQueues();
    } catch (err) {
      setError(err, 'Failed to create queue.');
    } finally {
      saving = false;
    }
  }

  function selectQueue(queue: Queue) {
    editQueueId = queue.id;
    editName = queue.name;
    editDescription = queue.description ?? '';
    editCadenceMinutes = Math.max(1, Math.round(queue.cadence_seconds / 60));
    editStartAt = toLocalInput(new Date(queue.start_at));
    nextSlot = null;
  }

  async function updateSelectedQueue(event: SubmitEvent) {
    event.preventDefault();
    if (!selectedQueue) return;
    saving = true;

    try {
      await shipyardApi.updateQueue(session.token, selectedQueue.id, {
        name: editName,
        description: editDescription.trim() || null,
        cadence_seconds: Math.max(1, Math.round(editCadenceMinutes * 60)),
        start_at: new Date(editStartAt).toISOString()
      });
      message = 'Queue updated.';
      error = '';
      await loadQueues();
    } catch (err) {
      setError(err, 'Failed to update queue.');
    } finally {
      saving = false;
    }
  }

  async function loadNextSlot(queueId: string) {
    saving = true;
    try {
      nextSlot = await shipyardApi.nextQueueSlot(session.token, queueId);
      message = 'Next queue slot calculated.';
      error = '';
    } catch (err) {
      setError(err, 'Failed to calculate the next queue slot.');
    } finally {
      saving = false;
    }
  }

  async function archiveQueue(queueId: string) {
    saving = true;
    try {
      await shipyardApi.archiveQueue(session.token, queueId);
      message = 'Queue archived.';
      error = '';
      await loadQueues();
      if (editQueueId === queueId) {
        editQueueId = '';
        nextSlot = null;
      }
    } catch (err) {
      setError(err, 'Failed to archive queue.');
    } finally {
      saving = false;
    }
  }

  onMount(loadQueues);
</script>

<svelte:head>
  <title>Queues - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Scheduling</p>
    <h1>Queues</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadQueues} disabled={loading}>
    Refresh
  </button>
</header>

{#if message}
  <section class="notice success">{message}</section>
{/if}
{#if error}
  <section class="notice error">{error}</section>
{/if}
{#if !session.token || !session.ownerPubkey}
  <section class="notice"><a href="/settings#login">Sign in</a> before managing queues.</section>
{/if}

<section class="panel stack">
  <div class="two-column">
    <form class="card-form" onsubmit={createQueue}>
      <div class="section-header">
        <h2>Create Queue</h2>
        <button class="primary-action" type="submit" disabled={saving || !name.trim()}>
          Create
        </button>
      </div>

      <label class="field">
        <span>Name</span>
        <input bind:value={name} placeholder="Weekday posts" />
      </label>

      <label class="field">
        <span>Description</span>
        <textarea bind:value={description} rows="4" placeholder="Optional queue purpose"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Cadence minutes</span>
          <input bind:value={cadenceMinutes} min="1" type="number" />
        </label>

        <label class="field">
          <span>First slot</span>
          <input bind:value={startAt} type="datetime-local" />
        </label>
      </div>
    </form>

    <form class="card-form" onsubmit={updateSelectedQueue}>
      <div class="section-header">
        <h2>Update Queue</h2>
        <button class="primary-action" type="submit" disabled={saving || !selectedQueue || !editName.trim()}>
          Save
        </button>
      </div>

      {#if selectedQueue}
        <label class="field">
          <span>Name</span>
          <input bind:value={editName} />
        </label>

        <label class="field">
          <span>Description</span>
          <textarea bind:value={editDescription} rows="4"></textarea>
        </label>

        <div class="form-grid">
          <label class="field">
            <span>Cadence minutes</span>
            <input bind:value={editCadenceMinutes} min="1" type="number" />
          </label>

          <label class="field">
            <span>First slot</span>
            <input bind:value={editStartAt} type="datetime-local" />
          </label>
        </div>

        <div class="inline-actions">
          <button class="secondary-action" type="button" onclick={() => loadNextSlot(selectedQueue.id)} disabled={saving}>
            Next Slot
          </button>
        </div>

        {#if nextSlot?.queue_id === selectedQueue.id}
          <p class="meta-line">
            Next slot {formatDate(nextSlot.next_slot)}. Latest assigned slot {formatDate(nextSlot.latest_queue_slot)}.
          </p>
        {/if}
      {:else}
        <p class="meta-line">Select an active queue to edit cadence, start time, or calculate the next slot.</p>
      {/if}
    </form>
  </div>

  <div class="two-column">
    <div class="card-form">
      <div class="section-header">
        <h2>Active Queues</h2>
        <span class="muted-text">{compactPubkey(session.ownerPubkey)}</span>
      </div>

      <div class="rows compact">
        {#if loading}
          <article class="row">
            <p>Loading queues...</p>
          </article>
        {:else if !activeQueues.length}
          <article class="row">
            <p>No active queues.</p>
          </article>
        {:else}
          {#each activeQueues as queue}
            <article class="row">
              <p>
                <strong>{queue.name}</strong>
                <span>{queue.description ?? 'No description'}</span>
              </p>
              <span class="muted-text">{cadenceLabel(queue.cadence_seconds)}</span>
              <time>{formatDate(queue.start_at)}</time>
              <div class="inline-actions">
                <button class="secondary-action" type="button" onclick={() => selectQueue(queue)}>
                  Select
                </button>
                <button
                  class="secondary-action"
                  type="button"
                  onclick={() => loadNextSlot(queue.id)}
                  disabled={saving}
                >
                  Next
                </button>
                <button
                  class="danger-action"
                  type="button"
                  onclick={() => archiveQueue(queue.id)}
                  disabled={saving}
                >
                  Archive
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>
    </div>

    <div class="card-form">
      <div class="section-header">
        <h2>Archived</h2>
        <span class="muted-text">{archivedQueues.length} total</span>
      </div>

      <div class="rows compact">
        {#if !archivedQueues.length}
          <article class="row">
            <p>No archived queues.</p>
          </article>
        {:else}
          {#each archivedQueues as queue}
            <article class="row">
              <p>{queue.name}</p>
              <span class="muted-text">{cadenceLabel(queue.cadence_seconds)}</span>
              <time>{formatDate(queue.archived_at)}</time>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</section>
