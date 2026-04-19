<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { compactPubkey, readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import WelcomeModal from '$lib/components/onboarding/WelcomeModal.svelte';
  import type { ApiErrorBody, PublishItem } from '$lib/api/types';

  type Stat = {
    label: string;
    value: string;
    sub: string;
    attention?: boolean;
  };

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let items = $state<PublishItem[]>([]);
  let proposals = $state<PublishItem[]>([]);
  let status = $state<Record<string, unknown> | null>(null);
  let loading = $state(true);
  let error = $state('');

  let scheduledCount = $derived(items.filter((item) => item.state === 'SCHEDULED').length);
  let publishedTodayCount = $derived(
    items.filter((item) => {
      if (!item.published_at) return false;
      return new Date(item.published_at).toDateString() === new Date().toDateString();
    }).length
  );
  let failedCount = $derived(items.filter((item) => item.state === 'FAILED').length);
  let upcoming = $derived(
    items
      .filter((item) => ['SCHEDULED', 'PUBLISHING', 'FAILED'].includes(item.state))
      .sort((left, right) => {
        const leftTime = left.publish_time ? Date.parse(left.publish_time) : Number.MAX_SAFE_INTEGER;
        const rightTime = right.publish_time ? Date.parse(right.publish_time) : Number.MAX_SAFE_INTEGER;
        return leftTime - rightTime;
      })
      .slice(0, 6)
  );
  let stats = $derived<Stat[]>([
    {
      label: 'Scheduled',
      value: String(scheduledCount),
      sub: nextScheduledLabel(items)
    },
    {
      label: 'Pending Review',
      value: String(proposals.length),
      sub: proposalSourceLabel(proposals)
    },
    {
      label: 'Published Today',
      value: String(publishedTodayCount),
      sub: items.length ? 'From active publish history' : 'No publish history loaded'
    },
    {
      label: 'Needs Attention',
      value: String(failedCount),
      sub: failedCount === 1 ? 'One failed publish' : `${failedCount} failed publishes`,
      attention: failedCount > 0
    }
  ]);

  function proposalSourceLabel(currentProposals: PublishItem[]) {
    const delegateCount = new Set(
      currentProposals
        .filter((proposal) => proposal.created_by_pubkey !== proposal.owner_pubkey)
        .map((proposal) => proposal.created_by_pubkey)
    ).size;

    if (!currentProposals.length) {
      return 'Nothing waiting for review';
    }

    return delegateCount ? `From ${delegateCount} delegate${delegateCount === 1 ? '' : 's'}` : 'Owner drafts only';
  }

  function nextScheduledLabel(currentItems: PublishItem[]) {
    const next = currentItems
      .filter((item) => item.state === 'SCHEDULED' && item.publish_time)
      .sort((left, right) => Date.parse(left.publish_time ?? '') - Date.parse(right.publish_time ?? ''))[0];

    if (!next?.publish_time) {
      return 'No scheduled publish time';
    }

    return `Next ${formatDate(next.publish_time)}`;
  }

  function eventSummary(item: PublishItem) {
    const event = item.unsigned_event_json ?? item.signed_event_json;
    const content = event?.content;
    return typeof content === 'string' && content.trim() ? content : item.event_id ?? item.id;
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

  async function loadDashboard() {
    loading = true;
    error = '';
    session = readShipyardSession();

    try {
      status = await shipyardApi.status();

      if (!session.token || !session.ownerPubkey) {
        items = [];
        proposals = [];
        return;
      }

      const [loadedItems, loadedProposals] = await Promise.all([
        shipyardApi.publishItems(session.token, session.ownerPubkey),
        shipyardApi.proposals(session.token, session.ownerPubkey)
      ]);
      items = loadedItems;
      proposals = loadedProposals;
    } catch (err) {
      error = (err as ApiErrorBody).message ?? 'Failed to load dashboard.';
    } finally {
      loading = false;
    }
  }

  onMount(loadDashboard);
</script>

<svelte:head>
  <title>Shipyard</title>
  <meta
    name="description"
    content="Shipyard publishing cockpit for Nostr queues, drafts, proposals, and scheduled posts."
  />
</svelte:head>

<WelcomeModal />

<header class="page-header">
  <div>
    <p class="eyebrow">Publishing</p>
    <h1>Dashboard</h1>
  </div>
  <a class="primary-action" href="/write">Write</a>
</header>

{#if error}
  <section class="notice error">{error}</section>
{:else if !session.token || !session.ownerPubkey}
  <section class="notice">
    <a href="/settings#login">Sign in with a browser signer</a> to load account-specific publishing data.
  </section>
{:else if status}
  <section class="notice muted">
    API connected for {compactPubkey(session.ownerPubkey)}.
  </section>
{/if}

<section class="stats-grid" aria-label="Publishing stats">
  {#each stats as stat}
    <article class="stat" class:attention={stat.attention}>
      <span>{stat.label}</span>
      <strong>{stat.value}</strong>
      <small>{stat.sub}</small>
    </article>
  {/each}
</section>

<section class="panel" aria-labelledby="upcoming-title">
  <div class="section-header">
    <h2 id="upcoming-title">Upcoming</h2>
    <a href="/scheduled">View all</a>
  </div>

  <div class="rows">
    {#if loading}
      <article class="row">
        <p>Loading publishing data...</p>
      </article>
    {:else if !upcoming.length}
      <article class="row">
        <p>No upcoming publish items.</p>
      </article>
    {:else}
      {#each upcoming as item}
        <article class="row">
          <p>{eventSummary(item)}</p>
          <StatusBadge state={item.state} />
          <time>{formatDate(item.publish_time)}</time>
        </article>
      {/each}
    {/if}
  </div>
</section>

<section class="panel" aria-labelledby="review-title">
  <div class="section-header">
    <h2 id="review-title">Pending Review</h2>
    <a href="/proposals">Review</a>
  </div>

  <div class="rows">
    {#if loading}
      <article class="row">
        <p>Loading proposals...</p>
      </article>
    {:else if !proposals.length}
      <article class="row">
        <p>No proposals waiting for owner action.</p>
      </article>
    {:else}
      {#each proposals.slice(0, 5) as proposal}
        <article class="row">
          <p>{eventSummary(proposal)}</p>
          <StatusBadge state={proposal.state} />
          <time>{formatDate(proposal.created_at)}</time>
        </article>
      {/each}
    {/if}
  </div>
</section>
