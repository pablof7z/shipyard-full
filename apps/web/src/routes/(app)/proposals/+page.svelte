<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { compactPubkey, readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import type {
    ApiErrorBody,
    BatchSignProposalItem,
    PublishItem,
    PublishTrigger,
    Queue
  } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let proposals = $state<PublishItem[]>([]);
  let queues = $state<Queue[]>([]);
  let noteContent = $state('');
  let unsignedEventText = $state('');
  let trigger = $state<PublishTrigger>('TIME');
  let publishAt = $state(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)));
  let queueId = $state('');
  let selectedProposalId = $state('');
  let signedEventText = $state('');
  let batchSignText = $state('');
  let rejectReason = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');
  const batchSignPlaceholder = '[{"proposal_id":"...","signed_event":{}}]';

  let selectedProposal = $derived(proposals.find((proposal) => proposal.id === selectedProposalId));
  let activeQueues = $derived(queues.filter((queue) => !queue.archived_at));

  function toLocalInput(date: Date) {
    const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 16);
  }

  function formatDate(value: string | null) {
    if (!value) return 'Unscheduled';
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function eventSummary(item: PublishItem) {
    const event = item.unsigned_event_json ?? item.signed_event_json;
    const content = event?.content;
    return typeof content === 'string' && content.trim() ? content : item.id;
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
    message = '';
  }

  function buildUnsignedTemplate() {
    const createdAt =
      trigger === 'TIME' && publishAt
        ? Math.floor(new Date(publishAt).getTime() / 1000)
        : Math.floor(Date.now() / 1000);

    unsignedEventText = JSON.stringify(
      {
        id: null,
        pubkey: session.ownerPubkey,
        created_at: createdAt,
        kind: 1,
        tags: [],
        content: noteContent,
        sig: null
      },
      null,
      2
    );
  }

  async function loadProposals() {
    session = readShipyardSession();
    loading = true;
    error = '';

    try {
      if (!session.token || !session.ownerPubkey) {
        proposals = [];
        queues = [];
        return;
      }

      const [proposalResponse, queueResponse] = await Promise.all([
        shipyardApi.proposals(session.token, session.ownerPubkey),
        shipyardApi.queues(session.token, session.ownerPubkey)
      ]);
      proposals = proposalResponse;
      queues = queueResponse;

      if (!queueId && activeQueues[0]) {
        queueId = activeQueues[0].id;
      }
    } catch (err) {
      setError(err, 'Failed to load proposals.');
    } finally {
      loading = false;
    }
  }

  async function createProposal(event: SubmitEvent) {
    event.preventDefault();
    saving = true;

    try {
      const unsignedEvent = JSON.parse(unsignedEventText) as Record<string, unknown>;
      await shipyardApi.createProposal(session.token, {
        owner_pubkey: session.ownerPubkey,
        unsigned_event: unsignedEvent,
        trigger,
        publish_time: trigger === 'TIME' ? new Date(publishAt).toISOString() : null,
        queue_id: trigger === 'QUEUE' ? queueId : null
      });
      message = 'Proposal created.';
      error = '';
      noteContent = '';
      unsignedEventText = '';
      await loadProposals();
    } catch (err) {
      setError(err, 'Failed to create proposal.');
    } finally {
      saving = false;
    }
  }

  async function signSelectedProposal(event: SubmitEvent) {
    event.preventDefault();
    if (!selectedProposal) return;

    saving = true;
    try {
      const signedEvent = JSON.parse(signedEventText) as Record<string, unknown>;
      await shipyardApi.signProposal(session.token, selectedProposal.id, signedEvent);
      message = 'Proposal signed and scheduled.';
      error = '';
      signedEventText = '';
      selectedProposalId = '';
      await loadProposals();
    } catch (err) {
      setError(err, 'Failed to sign proposal.');
    } finally {
      saving = false;
    }
  }

  async function batchSign(event: SubmitEvent) {
    event.preventDefault();
    saving = true;

    try {
      const parsed = JSON.parse(batchSignText) as
        | BatchSignProposalItem[]
        | { items: BatchSignProposalItem[] };
      const items = Array.isArray(parsed) ? parsed : parsed.items;
      const response = await shipyardApi.batchSignProposals(session.token, items);
      const successes = response.results.filter((result) => result.item).length;
      const failures = response.results.length - successes;
      message = `Batch signed ${successes} proposal${successes === 1 ? '' : 's'} with ${failures} failure${failures === 1 ? '' : 's'}.`;
      error = '';
      batchSignText = '';
      await loadProposals();
    } catch (err) {
      setError(err, 'Failed to batch sign proposals.');
    } finally {
      saving = false;
    }
  }

  async function rejectSelectedProposal() {
    if (!selectedProposal) return;

    saving = true;
    try {
      await shipyardApi.rejectProposal(session.token, selectedProposal.id, rejectReason);
      message = 'Proposal rejected.';
      error = '';
      rejectReason = '';
      selectedProposalId = '';
      await loadProposals();
    } catch (err) {
      setError(err, 'Failed to reject proposal.');
    } finally {
      saving = false;
    }
  }

  async function cancelProposal(proposalId: string) {
    saving = true;
    try {
      await shipyardApi.deleteProposal(session.token, proposalId);
      message = 'Proposal cancelled.';
      error = '';
      await loadProposals();
    } catch (err) {
      setError(err, 'Failed to cancel proposal.');
    } finally {
      saving = false;
    }
  }

  onMount(loadProposals);
</script>

<svelte:head>
  <title>Proposals - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Review</p>
    <h1>Proposals</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadProposals} disabled={loading}>
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
  <section class="notice"><a href="/settings#login">Sign in</a> before reviewing proposals.</section>
{/if}

<section class="panel stack">
  <div class="two-column">
    <form class="card-form" onsubmit={createProposal}>
      <div class="section-header">
        <h2>Create Proposal</h2>
        <button class="primary-action" type="submit" disabled={saving || !unsignedEventText.trim()}>
          Submit
        </button>
      </div>

      <label class="field">
        <span>Note content</span>
        <textarea bind:value={noteContent} rows="4" placeholder="Draft note content"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Trigger</span>
          <select bind:value={trigger}>
            <option value="TIME">Time</option>
            <option value="QUEUE">Queue</option>
          </select>
        </label>

        {#if trigger === 'TIME'}
          <label class="field">
            <span>Publish time</span>
            <input bind:value={publishAt} type="datetime-local" />
          </label>
        {:else if trigger === 'QUEUE'}
          <label class="field">
            <span>Queue</span>
            <select bind:value={queueId}>
              {#each activeQueues as queue}
                <option value={queue.id}>{queue.name}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>

      <button class="secondary-action" type="button" onclick={buildUnsignedTemplate}>
        Build Unsigned JSON
      </button>

      <label class="field">
        <span>Unsigned event JSON</span>
        <textarea bind:value={unsignedEventText} rows="12" spellcheck="false"></textarea>
      </label>
    </form>

    <div class="card-form">
      <div class="section-header">
        <h2>Owner Action</h2>
      </div>

      <form class="inner-form" onsubmit={signSelectedProposal}>
        <div class="section-header flush">
          <h2>Single Sign</h2>
          <button
            class="primary-action"
            type="submit"
            disabled={saving || !selectedProposal || !signedEventText.trim()}
          >
            Sign
          </button>
        </div>

        <label class="field">
          <span>Selected proposal</span>
          <select bind:value={selectedProposalId}>
            <option value="">Select proposal</option>
            {#each proposals as proposal}
              <option value={proposal.id}>{eventSummary(proposal)}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>Signed event JSON</span>
          <textarea bind:value={signedEventText} rows="8" spellcheck="false"></textarea>
        </label>

        {#if selectedProposal?.unsigned_event_json}
          <label class="field">
            <span>Unsigned JSON</span>
            <textarea
              readonly
              rows="8"
              spellcheck="false"
              value={JSON.stringify(selectedProposal.unsigned_event_json, null, 2)}
            ></textarea>
          </label>
        {/if}
      </form>

      <form class="inner-form" onsubmit={batchSign}>
        <div class="section-header flush">
          <h2>Batch Sign</h2>
          <button class="primary-action" type="submit" disabled={saving || !batchSignText.trim()}>
            Sign Batch
          </button>
        </div>

        <label class="field">
          <span>Batch JSON</span>
          <textarea
            bind:value={batchSignText}
            rows="7"
            spellcheck="false"
            placeholder={batchSignPlaceholder}
          ></textarea>
        </label>
      </form>

      <label class="field">
        <span>Reject reason</span>
        <input bind:value={rejectReason} placeholder="Optional" />
      </label>

      <div class="inline-actions">
        <button
          class="danger-action"
          type="button"
          onclick={rejectSelectedProposal}
          disabled={saving || !selectedProposal}
        >
          Reject
        </button>
      </div>
    </div>
  </div>

  <div class="card-form">
    <div class="section-header">
      <h2>Pending Proposals</h2>
      <span class="muted-text">{compactPubkey(session.ownerPubkey)}</span>
    </div>

    <div class="rows">
      {#if loading}
        <article class="row">
          <p>Loading proposals...</p>
        </article>
      {:else if !proposals.length}
        <article class="row">
          <p>No proposals waiting for review.</p>
        </article>
      {:else}
        {#each proposals as proposal}
          <article class="row">
            <p>
              <strong>{eventSummary(proposal)}</strong>
              <span>Created by {compactPubkey(proposal.created_by_pubkey)}</span>
            </p>
            <StatusBadge state={proposal.state} />
            <time>{formatDate(proposal.publish_time ?? proposal.created_at)}</time>
            <div class="inline-actions">
              <button
                class="secondary-action"
                type="button"
                onclick={() => (selectedProposalId = proposal.id)}
              >
                Select
              </button>
              <button
                class="danger-action"
                type="button"
                onclick={() => cancelProposal(proposal.id)}
                disabled={saving}
              >
                Cancel
              </button>
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </div>
</section>
