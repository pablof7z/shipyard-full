<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import WelcomeModal from '$lib/components/onboarding/WelcomeModal.svelte';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
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
      label: 'Pending review',
      value: String(proposals.length),
      sub: proposalSourceLabel(proposals)
    },
    {
      label: 'Published today',
      value: String(publishedTodayCount),
      sub: publishedTodayCount ? 'Nice work' : 'Nothing out yet today'
    },
    {
      label: 'Needs attention',
      value: String(failedCount),
      sub: failedCount === 1 ? '1 post failed' : failedCount ? `${failedCount} posts failed` : 'All good',
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
      return 'Nothing waiting for you';
    }

    return delegateCount ? `From ${delegateCount} teammate${delegateCount === 1 ? '' : 's'}` : 'Your drafts';
  }

  function nextScheduledLabel(currentItems: PublishItem[]) {
    const next = currentItems
      .filter((item) => item.state === 'SCHEDULED' && item.publish_time)
      .sort((left, right) => Date.parse(left.publish_time ?? '') - Date.parse(right.publish_time ?? ''))[0];

    if (!next?.publish_time) {
      return 'Nothing queued';
    }

    return `Next ${formatDate(next.publish_time)}`;
  }

  function eventSummary(item: PublishItem) {
    const event = item.unsigned_event_json ?? item.signed_event_json;
    const content = event?.content;
    return typeof content === 'string' && content.trim() ? content : 'Untitled post';
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
    content="Shipyard — schedule posts, manage drafts, and review teammate posts."
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
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to load your publishing data.
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
        <p>Loading...</p>
      </article>
    {:else if !upcoming.length}
      <article class="row">
        <p>Nothing scheduled yet.</p>
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
    <h2 id="review-title">Pending review</h2>
    <a href="/proposals">Review</a>
  </div>

  <div class="rows">
    {#if loading}
      <article class="row">
        <p>Loading...</p>
      </article>
    {:else if !proposals.length}
      <article class="row">
        <p>Nothing waiting for your review.</p>
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
