<script lang="ts">
  import { onMount } from 'svelte';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import { createQueuesPageState } from '$lib/features/queues/page-state.svelte';

  const model = createQueuesPageState();

  onMount(model.loadQueues);
</script>

<svelte:head>
  <title>Queues - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Scheduling</p>
    <h1>Queues</h1>
  </div>
  <button class="secondary-action" type="button" onclick={model.loadQueues} disabled={model.state.loading}>
    Refresh
  </button>
</header>

{#if model.state.message}
  <section class="notice success">{model.state.message}</section>
{/if}
{#if model.state.error}
  <section class="notice error">{model.state.error}</section>
{/if}
{#if !model.state.session.token || !model.state.session.ownerPubkey}
  <section class="notice">
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to manage queues.
  </section>
{/if}

<section class="panel stack">
  <div class="two-column">
    <form class="card-form" onsubmit={model.createQueue}>
      <div class="section-header">
        <h2>New queue</h2>
        <button class="primary-action" type="submit" disabled={model.state.saving || !model.state.name.trim()}>
          Create
        </button>
      </div>

      <label class="field">
        <span>Name</span>
        <input bind:value={model.state.name} placeholder="Weekday posts" />
      </label>

      <label class="field">
        <span>Description</span>
        <textarea bind:value={model.state.description} rows="4" placeholder="What's this queue for? (optional)"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Post every (minutes)</span>
          <input bind:value={model.state.cadenceMinutes} min="1" type="number" />
        </label>

        <label class="field">
          <span>Start at</span>
          <input bind:value={model.state.startAt} type="datetime-local" />
        </label>
      </div>
    </form>

    <form class="card-form" onsubmit={model.updateSelectedQueue}>
      <div class="section-header">
        <h2>Edit queue</h2>
        <button
          class="primary-action"
          type="submit"
          disabled={model.state.saving || !model.selectedQueue || !model.state.editName.trim()}
        >
          Save
        </button>
      </div>

      {#if model.selectedQueue}
        <label class="field">
          <span>Name</span>
          <input bind:value={model.state.editName} />
        </label>

        <label class="field">
          <span>Description</span>
          <textarea bind:value={model.state.editDescription} rows="4"></textarea>
        </label>

        <div class="form-grid">
          <label class="field">
            <span>Post every (minutes)</span>
            <input bind:value={model.state.editCadenceMinutes} min="1" type="number" />
          </label>

          <label class="field">
            <span>Start at</span>
            <input bind:value={model.state.editStartAt} type="datetime-local" />
          </label>
        </div>

        <div class="inline-actions">
          <button
            class="secondary-action"
            type="button"
            onclick={() => model.loadNextSlot(model.selectedQueue?.id ?? '')}
            disabled={model.state.saving}
          >
            When's the next slot?
          </button>
        </div>

        {#if model.state.nextSlot?.queue_id === model.selectedQueue.id}
          <p class="meta-line">
            Next slot: {model.formatDate(model.state.nextSlot.next_slot)}.
            {#if model.state.nextSlot.latest_queue_slot}
              Last post went out {model.formatDate(model.state.nextSlot.latest_queue_slot)}.
            {/if}
          </p>
        {/if}
      {:else}
        <p class="meta-line">Pick a queue on the right to edit its name, timing, or schedule.</p>
      {/if}
    </form>
  </div>

  <div class="two-column">
    <div class="card-form">
      <div class="section-header">
        <h2>Your queues</h2>
      </div>

      <div class="rows compact">
        {#if model.state.loading}
          <article class="row">
            <p>Loading...</p>
          </article>
        {:else if !model.activeQueues.length}
          <article class="row">
            <p>No queues yet. Create your first on the left.</p>
          </article>
        {:else}
          {#each model.activeQueues as queue}
            <article class="row">
              <p>
                <strong>{queue.name}</strong>
                {#if queue.description}
                  <span>{queue.description}</span>
                {/if}
              </p>
              <span class="muted-text">{model.cadenceLabel(queue.cadence_seconds)}</span>
              <time>{model.formatDate(queue.start_at)}</time>
              <div class="inline-actions">
                <button class="secondary-action" type="button" onclick={() => model.selectQueue(queue)}>
                  Edit
                </button>
                <button
                  class="secondary-action"
                  type="button"
                  onclick={() => model.loadNextSlot(queue.id)}
                  disabled={model.state.saving}
                >
                  Next slot
                </button>
                <button
                  class="danger-action"
                  type="button"
                  onclick={() => model.archiveQueue(queue.id)}
                  disabled={model.state.saving}
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
        <span class="muted-text">{model.archivedQueues.length} total</span>
      </div>

      <div class="rows compact">
        {#if !model.archivedQueues.length}
          <article class="row">
            <p>No archived queues.</p>
          </article>
        {:else}
          {#each model.archivedQueues as queue}
            <article class="row">
              <p>{queue.name}</p>
              <span class="muted-text">{model.cadenceLabel(queue.cadence_seconds)}</span>
              <time>{model.formatDate(queue.archived_at)}</time>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</section>
