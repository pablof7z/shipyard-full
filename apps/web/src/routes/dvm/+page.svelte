<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { compactPubkey, readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import type { ApiErrorBody, DvmRequest } from '$lib/api/types';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let requests = $state<DvmRequest[]>([]);
  let loading = $state(true);
  let error = $state('');

  function formatDate(value: string) {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function requestSummary(request: DvmRequest) {
    const content = request.raw_request_event.content;
    if (typeof content === 'string' && content.trim()) {
      return request.encrypted ? 'Encrypted request' : content;
    }

    return request.request_event_id;
  }

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
  }

  async function loadRequests() {
    session = readShipyardSession();
    loading = true;
    error = '';

    try {
      if (!session.token || !session.ownerPubkey) {
        requests = [];
        return;
      }

      requests = await shipyardApi.dvmRequests(session.token, session.ownerPubkey);
    } catch (err) {
      setError(err, 'Failed to load DVM requests.');
    } finally {
      loading = false;
    }
  }

  onMount(loadRequests);
</script>

<svelte:head>
  <title>DVM Requests - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Kind 5905</p>
    <h1>DVM Requests</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadRequests} disabled={loading}>
    Refresh
  </button>
</header>

{#if error}
  <section class="notice error">{error}</section>
{/if}
{#if !session.token || !session.ownerPubkey}
  <section class="notice"><a href="/settings#login">Sign in</a> before viewing DVM requests.</section>
{/if}

<section class="panel stack">
  <div class="card-form">
    <div class="section-header">
      <h2>Requests</h2>
      <span class="muted-text">{compactPubkey(session.ownerPubkey)}</span>
    </div>

    <div class="rows">
      {#if loading}
        <article class="row">
          <p>Loading DVM requests...</p>
        </article>
      {:else if !requests.length}
        <article class="row">
          <p>No DVM requests stored for this owner.</p>
        </article>
      {:else}
        {#each requests as request}
          <article class="row">
            <p>
              <strong>{requestSummary(request)}</strong>
              <span>{request.encrypted ? 'Encrypted' : 'Clear'} from {compactPubkey(request.request_pubkey)}</span>
            </p>
            <span class="muted-text">{request.status}</span>
            <time>{formatDate(request.created_at)}</time>
            <span class="muted-text">{request.error ?? request.request_event_id}</span>
          </article>
        {/each}
      {/if}
    </div>
  </div>
</section>
