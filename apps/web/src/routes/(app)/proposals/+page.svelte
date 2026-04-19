<script lang="ts">
  import { onMount } from 'svelte';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { User } from '$lib/components/ui/user';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import { createProposalsPageState } from '$lib/features/proposals/page-state.svelte';

  const model = createProposalsPageState();
  let selectedCount = $derived(model.state.selected.size);

  onMount(() => {
    void ensureClientNdk().catch(() => {});
    model.load();
  });
</script>

<svelte:head>
  <title>Review - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Team</p>
    <h1>Review</h1>
  </div>
  <button
    class="secondary-action"
    type="button"
    onclick={model.load}
    disabled={model.state.loading}
  >
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
    to review posts from your teammates.
  </section>
{/if}

{#if selectedCount > 0}
  <section class="bulk-bar">
    <span>{selectedCount} selected</span>
    <div class="bulk-actions">
      <button
        class="secondary-action"
        type="button"
        onclick={model.clearSelection}
        disabled={model.state.saving}
      >
        Clear
      </button>
      <button
        class="primary-action"
        type="button"
        onclick={model.approveSelected}
        disabled={model.state.saving}
      >
        Approve {selectedCount}
      </button>
    </div>
  </section>
{/if}

<section class="panel stack">
  <div class="review-list">
    {#if model.state.loading}
      <article class="row">
        <p>Loading...</p>
      </article>
    {:else if !model.state.proposals.length}
      <article class="row empty">
        <p>No posts waiting for your review.</p>
      </article>
    {:else}
      {#each model.state.proposals as proposal}
        <article class="review-card" class:selected={model.state.selected.has(proposal.id)}>
          <label class="select-box">
            <input
              type="checkbox"
              checked={model.state.selected.has(proposal.id)}
              onchange={() => model.toggle(proposal.id)}
            />
          </label>

          <User.Root {ndk} pubkey={proposal.created_by_pubkey} class="author">
            <User.Avatar class="avatar" alt="Author" />
            <span class="author-copy">
              <strong><User.Name /></strong>
              <small>{model.whenLabel(proposal)}</small>
            </span>
          </User.Root>

          {#if model.state.editingId === proposal.id}
            <textarea
              class="edit-box"
              bind:value={model.state.editedContent}
              rows="6"
              placeholder="Edit the post"
            ></textarea>
          {:else}
            <p class="post-content">
              {model.postContent(proposal)}
              {#if model.state.editedIds.has(proposal.id)}
                <span class="edited-tag">edited</span>
              {/if}
            </p>
          {/if}

          {#if model.state.rejectingId === proposal.id}
            <div class="reject-box">
              <input
                bind:value={model.state.rejectReason}
                placeholder="Optional note for your teammate"
                autocomplete="off"
              />
              <div class="inline-actions">
                <button
                  class="secondary-action"
                  type="button"
                  onclick={model.cancelReject}
                  disabled={model.state.saving}
                >
                  Cancel
                </button>
                <button
                  class="danger-action"
                  type="button"
                  onclick={() => model.rejectWithReason(proposal.id)}
                  disabled={model.state.saving}
                >
                  Reject
                </button>
              </div>
            </div>
          {:else if model.state.editingId === proposal.id}
            <div class="actions">
              <button
                class="secondary-action"
                type="button"
                onclick={model.cancelEdit}
                disabled={model.state.saving}
              >
                Cancel
              </button>
              <button
                class="primary-action"
                type="button"
                onclick={() => model.saveEdit(proposal.id)}
                disabled={model.state.saving}
              >
                Save edits
              </button>
            </div>
          {:else}
            <div class="actions">
              <button
                class="primary-action"
                type="button"
                onclick={() => model.approve(proposal)}
                disabled={model.state.saving}
              >
                Approve
              </button>
              <button
                class="secondary-action"
                type="button"
                onclick={() => model.startEdit(proposal)}
                disabled={model.state.saving || !proposal.unsigned_event_json}
              >
                Edit
              </button>
              <button
                class="secondary-action"
                type="button"
                onclick={() => model.startReject(proposal.id)}
                disabled={model.state.saving}
              >
                Reject
              </button>
              <button
                class="danger-action"
                type="button"
                onclick={() => model.remove(proposal.id)}
                disabled={model.state.saving}
              >
                Remove
              </button>
            </div>
          {/if}
        </article>
      {/each}
    {/if}
  </div>
</section>

<style>
  .bulk-bar {
    position: sticky;
    top: 0;
    z-index: 5;
    margin: 12px 24px 0;
    padding: 10px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-tertiary);
    border: 1px solid var(--accent);
    border-radius: 10px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    font-size: 13px;
  }

  .bulk-actions {
    display: flex;
    gap: 8px;
  }

  .review-list {
    display: grid;
    gap: 12px;
  }

  .review-card {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto auto;
    grid-template-areas:
      'select author'
      'select content'
      'select actions';
    gap: 10px 14px;
    padding: 16px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    transition: border-color 0.15s;
  }

  .review-card.selected {
    border-color: var(--accent);
  }

  .select-box {
    grid-area: select;
    display: flex;
    align-items: flex-start;
    padding-top: 2px;
  }

  .select-box input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  :global(.review-card .author) {
    grid-area: author;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }

  .author-copy {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .author-copy strong {
    font-size: 14px;
    color: var(--text-primary);
  }

  .author-copy small {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .post-content {
    grid-area: content;
    margin: 0;
    color: var(--text-primary);
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .actions,
  .reject-box {
    grid-area: actions;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .reject-box {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }

  .reject-box input {
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-primary);
    color: var(--text-primary);
    font: inherit;
  }

  .edit-box {
    grid-area: content;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--accent);
    background: var(--bg-primary);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    line-height: 1.5;
    resize: vertical;
    min-height: 100px;
  }

  .edited-tag {
    display: inline-block;
    margin-left: 8px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border-radius: 999px;
    vertical-align: middle;
  }

  .row.empty {
    padding: 24px;
    text-align: center;
    color: var(--text-tertiary);
  }
</style>
