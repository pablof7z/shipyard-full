<script lang="ts">
  import { onMount } from 'svelte';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { loadDrafts, deleteDraft } from '$lib/ndk/drafts';
  import {
    draftsState,
    setDraftList,
    removeDraftFromList,
    setLoading,
    setDraftError
  } from '$lib/features/drafts/state.svelte';
  import { readShipyardSession } from '$lib/api/session';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';

  let session = $state(readShipyardSession());

  function formatDate(unixTs: number): string {
    if (!unixTs) return '—';
    return new Date(unixTs * 1000).toLocaleString();
  }

  function draftTitle(entry: (typeof draftsState.draftList)[number]): string {
    if (!entry.inner) return `Draft ${entry.id.slice(0, 8)}…`;
    try {
      const parsed = JSON.parse(entry.inner.content ?? '{}') as Record<string, unknown>;
      const text = (parsed['content'] as string) ?? entry.inner.content ?? '';
      return text.slice(0, 60) || `Draft ${entry.id.slice(0, 8)}…`;
    } catch {
      return `Draft ${entry.id.slice(0, 8)}…`;
    }
  }

  async function load() {
    if (!session.ownerPubkey) return;
    setLoading(true);
    setDraftError('');
    try {
      await ensureClientNdk();
      const entries = await loadDrafts(ndk, session.ownerPubkey);
      setDraftList(entries);
    } catch (err) {
      setDraftError(`Failed to load drafts: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this draft?')) return;
    setLoading(true);
    setDraftError('');
    try {
      await deleteDraft(ndk, id);
      removeDraftFromList(id);
    } catch (err) {
      setDraftError(`Failed to delete draft: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  onMount(load);
</script>

<svelte:head>
  <title>Drafts - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Compose</p>
    <h1>Drafts</h1>
  </div>
  <button class="secondary-action" type="button" onclick={load} disabled={draftsState.isLoading}>
    Refresh
  </button>
</header>

{#if draftsState.draftError}
  <section class="notice error">{draftsState.draftError}</section>
{/if}

{#if !session.ownerPubkey}
  <section class="notice">
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to manage drafts.
  </section>
{:else if draftsState.isLoading}
  <section class="notice">Loading drafts…</section>
{:else if draftsState.draftList.length === 0}
  <section class="notice">No drafts found. Start writing and save a draft.</section>
{:else}
  <section class="panel">
    <table class="data-table">
      <thead>
        <tr>
          <th>Draft</th>
          <th>Last saved</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each draftsState.draftList as entry (entry.id)}
          <tr>
            <td>
              <a href="/drafts/{entry.id}">{draftTitle(entry)}</a>
            </td>
            <td>{formatDate(entry.updatedAt)}</td>
            <td>
              <button
                class="danger-action"
                type="button"
                onclick={() => handleDelete(entry.id)}
                disabled={draftsState.isLoading}
              >
                Delete
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
{/if}
