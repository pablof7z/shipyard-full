<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import type { ApiErrorBody, PublishItem } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let items = $state<PublishItem[]>([]);
  let loading = $state(true);
  let error = $state('');

  let publishedItems = $derived(items.filter((item) => item.state === 'PUBLISHED'));

  function eventSummary(item: PublishItem) {
    const event = item.signed_event_json ?? item.unsigned_event_json;
    const content = event?.content;
    return typeof content === 'string' && content.trim() ? content : 'Untitled post';
  }

  function formatDate(value: string | null) {
    if (!value) return 'Not published';
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
  }

  async function loadPublished() {
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
      setError(err, 'Failed to load published posts.');
    } finally {
      loading = false;
    }
  }

  onMount(loadPublished);
</script>

<svelte:head>
  <title>Published - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Archive</p>
    <h1>Published</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadPublished} disabled={loading}>
    Refresh
  </button>
</header>

{#if error}
  <section class="notice error">{error}</section>
{/if}
{#if !session.token || !session.ownerPubkey}
  <section class="notice">
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to see your published posts.
  </section>
{/if}

<section class="panel stack">
  <div class="card-form">
    <div class="section-header">
      <h2>Published posts</h2>
    </div>

    <div class="rows">
      {#if loading}
        <article class="row">
          <p>Loading...</p>
        </article>
      {:else if !publishedItems.length}
        <article class="row">
          <p>Nothing published yet.</p>
        </article>
      {:else}
        {#each publishedItems as item}
          <article class="row">
            <p>
              <strong>{eventSummary(item)}</strong>
            </p>
            <StatusBadge state={item.state} />
            <time>{formatDate(item.published_at)}</time>
          </article>
        {/each}
      {/if}
    </div>
  </div>
</section>
