<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { loadDraft, saveDraft, deleteDraft } from '$lib/ndk/drafts';
  import {
    draftsState,
    openDraft,
    closeDraft,
    markDirty,
    setLoading,
    setDraftError
  } from '$lib/features/drafts/state.svelte';
  import { readShipyardSession } from '$lib/api/session';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';

  const draftId = $derived(page.params.id ?? '');

  let session = $state(readShipyardSession());
  let content = $state('');
  let tags = $state<string[][]>([]);
  let targetKind = $state(1);
  let message = $state('');

  function loadContentFromDraft() {
    if (!draftsState.currentDraft?.inner) return;
    try {
      const ev = draftsState.currentDraft.inner;
      content = ev.content ?? '';
      tags = ev.tags ?? [];
      targetKind = ev.kind ?? 1;
    } catch {
      content = '';
    }
  }

  async function load() {
    if (!session.ownerPubkey) return;
    setLoading(true);
    setDraftError('');
    try {
      await ensureClientNdk();
      const entry = await loadDraft(ndk, session.ownerPubkey, draftId);
      if (entry) {
        openDraft(entry);
        loadContentFromDraft();
      } else {
        setDraftError('Draft not found.');
      }
    } catch (err) {
      setDraftError(`Failed to load draft: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave(event: SubmitEvent) {
    event.preventDefault();
    if (!session.ownerPubkey) return;
    setLoading(true);
    setDraftError('');
    message = '';
    try {
      const entry = await saveDraft(ndk, {
        id: draftId,
        targetKind,
        event: {
          kind: targetKind,
          content,
          tags,
          created_at: Math.floor(Date.now() / 1000)
        }
      });

      openDraft({
        id: draftId,
        draft: entry,
        inner: null,
        updatedAt: Math.floor(Date.now() / 1000)
      });
      message = 'Draft saved.';
    } catch (err) {
      setDraftError(`Failed to save draft: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete() {
    if (!confirm('Delete this draft permanently?')) return;
    setLoading(true);
    setDraftError('');
    try {
      await deleteDraft(ndk, draftId);
      closeDraft();
      await goto('/drafts');
    } catch (err) {
      setDraftError(`Failed to delete draft: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  onMount(load);
</script>

<svelte:head>
  <title>Edit Draft - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Drafts</p>
    <h1>Edit draft</h1>
  </div>
  <div class="header-actions">
    <a class="secondary-action" href="/drafts">← All drafts</a>
    <button
      class="danger-action"
      type="button"
      onclick={handleDelete}
      disabled={draftsState.isLoading}
    >
      Delete
    </button>
  </div>
</header>

{#if draftsState.draftError}
  <section class="notice error">{draftsState.draftError}</section>
{/if}
{#if message}
  <section class="notice success">{message}</section>
{/if}

{#if !session.ownerPubkey}
  <section class="notice">
    <button class="link-button" type="button" onclick={() => loginModal.show()}>Sign in</button>
    to manage drafts.
  </section>
{:else}
  <section class="panel">
    <form class="card-form" onsubmit={handleSave}>
      <div class="section-header">
        <h2>Draft</h2>
        <button
          class="primary-action"
          type="submit"
          disabled={draftsState.isLoading}
        >
          {draftsState.isDirty ? 'Save *' : 'Save'}
        </button>
      </div>

      <label class="field">
        <span>Content</span>
        <textarea bind:value={content} rows="12" oninput={markDirty}></textarea>
      </label>
    </form>
  </section>
{/if}
