<script lang="ts">
  import { onMount } from 'svelte';
  import { compactPubkey } from '$lib/api/session';
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
        <h2>Create Queue</h2>
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
        <textarea bind:value={model.state.description} rows="4" placeholder="Optional queue purpose"></textarea>
      </label>

      <div class="form-grid">
        <label class="field">
          <span>Cadence minutes</span>
          <input bind:value={model.state.cadenceMinutes} min="1" type="number" />
        </label>

        <label class="field">
          <span>First slot</span>
          <input bind:value={model.state.startAt} type="datetime-local" />
        </label>
      </div>
    </form>

    <form class="card-form" onsubmit={model.updateSelectedQueue}>
      <div class="section-header">
        <h2>Update Queue</h2>
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
            <span>Cadence minutes</span>
            <input bind:value={model.state.editCadenceMinutes} min="1" type="number" />
          </label>

          <label class="field">
            <span>First slot</span>
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
            Next Slot
          </button>
        </div>

        {#if model.state.nextSlot?.queue_id === model.selectedQueue.id}
          <p class="meta-line">
            Next slot {model.formatDate(model.state.nextSlot.next_slot)}. Latest assigned slot {model.formatDate(model.state.nextSlot.latest_queue_slot)}.
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
        <span class="muted-text">{compactPubkey(model.state.session.ownerPubkey)}</span>
      </div>

      <div class="rows compact">
        {#if model.state.loading}
          <article class="row">
            <p>Loading queues...</p>
          </article>
        {:else if !model.activeQueues.length}
          <article class="row">
            <p>No active queues.</p>
          </article>
        {:else}
          {#each model.activeQueues as queue}
            <article class="row">
              <p>
                <strong>{queue.name}</strong>
                <span>{queue.description ?? 'No description'}</span>
              </p>
              <span class="muted-text">{model.cadenceLabel(queue.cadence_seconds)}</span>
              <time>{model.formatDate(queue.start_at)}</time>
              <div class="inline-actions">
                <button class="secondary-action" type="button" onclick={() => model.selectQueue(queue)}>
                  Select
                </button>
                <button
                  class="secondary-action"
                  type="button"
                  onclick={() => model.loadNextSlot(queue.id)}
                  disabled={model.state.saving}
                >
                  Next
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
