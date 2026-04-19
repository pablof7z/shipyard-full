<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import type { ApiErrorBody, PublishItem, PublishTrigger } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let items = $state<PublishItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  let activeItems = $derived(
    items.filter((item) => !['PUBLISHED', 'CANCELLED', 'REJECTED'].includes(item.state))
  );

  function eventSummary(item: PublishItem) {
    const event = item.unsigned_event_json ?? item.signed_event_json;
    const content = event?.content;
    return typeof content === 'string' && content.trim() ? content : 'Untitled post';
  }

  function triggerLabel(trigger: PublishTrigger) {
    if (trigger === 'QUEUE') return 'From queue';
    if (trigger === 'SEND_NOW') return 'Publishing now';
    return 'Scheduled';
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

  function canCancel(item: PublishItem) {
    return !['PUBLISHED', 'PUBLISHING', 'CANCELLED'].includes(item.state);
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
    message = '';
  }

  async function loadItems() {
    session = readShipyardSession();
    loading = true;
    error = '';

    try {
      if (!session.token || !session.ownerPubkey) {
        items = [];
        return;
      }

      items = await shipyardApi.publishItems(session.token, session.ownerPubkey);
    } catch (err) {
      setError(err, 'Failed to load scheduled posts.');
    } finally {
      loading = false;
    }
  }

  async function cancelItem(itemId: string) {
    saving = true;
    try {
      await shipyardApi.cancelPublishItem(session.token, itemId);
      message = 'Post cancelled.';
      error = '';
      await loadItems();
    } catch (err) {
      setError(err, "Couldn't cancel — try again.");
    } finally {
      saving = false;
    }
  }

  async function retryItem(itemId: string) {
    saving = true;
    try {
      await shipyardApi.retryPublishItem(session.token, itemId);
      message = 'Retrying publish.';
      error = '';
      await loadItems();
    } catch (err) {
      setError(err, "Couldn't retry — try again.");
    } finally {
      saving = false;
    }
  }

  onMount(loadItems);
</script>

<svelte:head>
  <title>Scheduled - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Publishing</p>
    <h1>Scheduled</h1>
  </div>
  <a class="primary-action" href="/write">Write</a>
</header>

{#if message}
  <section class="notice success">{message}</section>
{/if}
{#if error}
  <section class="notice error">{error}</section>
{/if}
{#if !session.token || !session.ownerPubkey}
  <section class="notice">
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to see your scheduled posts.
  </section>
{/if}

<section class="panel stack">
  <div class="card-form">
    <div class="section-header">
      <h2>Upcoming</h2>
    </div>

    <div class="rows">
      {#if loading}
        <article class="row">
          <p>Loading...</p>
        </article>
      {:else if !activeItems.length}
        <article class="row">
          <p>Nothing scheduled.</p>
        </article>
      {:else}
        {#each activeItems as item}
          <article class="row">
            <p>
              <strong>{eventSummary(item)}</strong>
              <span>
                {triggerLabel(item.trigger)}
                {#if item.failure_message}
                  · {item.failure_message}
                {/if}
              </span>
            </p>
            <StatusBadge state={item.state} />
            <time>{formatDate(item.publish_time)}</time>
            <div class="inline-actions">
              {#if item.state === 'FAILED'}
                <button
                  class="secondary-action"
                  type="button"
                  onclick={() => retryItem(item.id)}
                  disabled={saving}
                >
                  Retry
                </button>
              {/if}
              {#if canCancel(item)}
                <button
                  class="danger-action"
                  type="button"
                  onclick={() => cancelItem(item.id)}
                  disabled={saving}
                >
                  Cancel
                </button>
              {/if}
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </div>
</section>
