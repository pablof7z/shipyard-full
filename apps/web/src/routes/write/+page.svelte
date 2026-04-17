<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { compactPubkey, readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import type { ApiErrorBody, PublishTrigger, Queue } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let queues = $state<Queue[]>([]);
  let content = $state('');
  let proposalTrigger = $state<PublishTrigger>('TIME');
  let signedTrigger = $state<PublishTrigger>('TIME');
  let publishAt = $state(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)));
  let signedPublishAt = $state(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)));
  let proposalQueueId = $state('');
  let signedQueueId = $state('');
  let tagsText = $state('[]');
  let signedEventText = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  let activeQueues = $derived(queues.filter((queue) => !queue.archived_at));

  function toLocalInput(date: Date) {
    const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 16);
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
    message = '';
  }

  function publishTimeFor(trigger: PublishTrigger, value: string) {
    return trigger === 'TIME' ? new Date(value).toISOString() : null;
  }

  function queueFor(trigger: PublishTrigger, queueId: string) {
    return trigger === 'QUEUE' ? queueId : null;
  }

  function unsignedEvent() {
    const createdAt =
      proposalTrigger === 'TIME' && publishAt
        ? Math.floor(new Date(publishAt).getTime() / 1000)
        : Math.floor(Date.now() / 1000);
    const tags = JSON.parse(tagsText) as string[][];

    return {
      id: null,
      pubkey: session.ownerPubkey,
      created_at: createdAt,
      kind: 1,
      tags,
      content,
      sig: null
    };
  }

  async function loadWriteContext() {
    session = readShipyardSession();
    loading = true;

    try {
      if (!session.token || !session.ownerPubkey) {
        queues = [];
        return;
      }

      queues = await shipyardApi.queues(session.token, session.ownerPubkey);
      const firstQueue = queues.find((queue) => !queue.archived_at);
      proposalQueueId = proposalQueueId || firstQueue?.id || '';
      signedQueueId = signedQueueId || firstQueue?.id || '';
    } catch (err) {
      setError(err, 'Failed to load write context.');
    } finally {
      loading = false;
    }
  }

  async function createProposal(event: SubmitEvent) {
    event.preventDefault();
    saving = true;

    try {
      await shipyardApi.createProposal(session.token, {
        owner_pubkey: session.ownerPubkey,
        unsigned_event: unsignedEvent(),
        trigger: proposalTrigger,
        publish_time: publishTimeFor(proposalTrigger, publishAt),
        queue_id: queueFor(proposalTrigger, proposalQueueId)
      });
      content = '';
      tagsText = '[]';
      message = 'Proposal submitted.';
      error = '';
    } catch (err) {
      setError(err, 'Failed to submit proposal.');
    } finally {
      saving = false;
    }
  }

  async function scheduleSigned(event: SubmitEvent) {
    event.preventDefault();
    saving = true;

    try {
      const signedEvent = JSON.parse(signedEventText) as Record<string, unknown>;
      if (signedTrigger === 'SEND_NOW') {
        await shipyardApi.sendNow(session.token, session.ownerPubkey, signedEvent);
      } else {
        await shipyardApi.scheduleSignedEvent(session.token, session.ownerPubkey, {
          signed_event: signedEvent,
          trigger: signedTrigger,
          publish_time: publishTimeFor(signedTrigger, signedPublishAt),
          queue_id: queueFor(signedTrigger, signedQueueId)
        });
      }
      signedEventText = '';
      message = 'Signed event scheduled.';
      error = '';
    } catch (err) {
      setError(err, 'Failed to schedule signed event.');
    } finally {
      saving = false;
    }
  }

  onMount(loadWriteContext);
</script>

<svelte:head>
  <title>Write - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Compose</p>
    <h1>Write</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadWriteContext} disabled={loading}>
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
  <section class="notice">Configure a session and active owner in Settings before writing.</section>
{/if}

<section class="panel stack">
  <div class="two-column">
    <form class="card-form" onsubmit={createProposal}>
      <div class="section-header">
        <h2>New Proposal</h2>
        <button class="primary-action" type="submit" disabled={saving || !content.trim()}>
          Submit
        </button>
      </div>

      <label class="field">
        <span>Owner</span>
        <input value={compactPubkey(session.ownerPubkey)} readonly />
      </label>

      <label class="field">
        <span>Content</span>
        <textarea bind:value={content} rows="8"></textarea>
      </label>

      <label class="field">
        <span>Tags JSON</span>
        <textarea bind:value={tagsText} rows="4" spellcheck="false"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Trigger</span>
          <select bind:value={proposalTrigger}>
            <option value="TIME">Time</option>
            <option value="QUEUE">Queue</option>
            <option value="SEND_NOW">Send now</option>
          </select>
        </label>

        {#if proposalTrigger === 'TIME'}
          <label class="field">
            <span>Publish time</span>
            <input bind:value={publishAt} type="datetime-local" />
          </label>
        {:else if proposalTrigger === 'QUEUE'}
          <label class="field">
            <span>Queue</span>
            <select bind:value={proposalQueueId}>
              {#each activeQueues as queue}
                <option value={queue.id}>{queue.name}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>
    </form>

    <form class="card-form" onsubmit={scheduleSigned}>
      <div class="section-header">
        <h2>Signed Event</h2>
        <button class="primary-action" type="submit" disabled={saving || !signedEventText.trim()}>
          Schedule
        </button>
      </div>

      <label class="field">
        <span>Signed event JSON</span>
        <textarea bind:value={signedEventText} rows="12" spellcheck="false"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Trigger</span>
          <select bind:value={signedTrigger}>
            <option value="TIME">Time</option>
            <option value="QUEUE">Queue</option>
            <option value="SEND_NOW">Send now</option>
          </select>
        </label>

        {#if signedTrigger === 'TIME'}
          <label class="field">
            <span>Publish time</span>
            <input bind:value={signedPublishAt} type="datetime-local" />
          </label>
        {:else if signedTrigger === 'QUEUE'}
          <label class="field">
            <span>Queue</span>
            <select bind:value={signedQueueId}>
              {#each activeQueues as queue}
                <option value={queue.id}>{queue.name}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>
    </form>
  </div>
</section>
