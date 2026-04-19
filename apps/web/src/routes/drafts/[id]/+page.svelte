<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { getNdk, connectNdk } from '$lib/ndk/client';
  import { loadDraft, saveDraft, deleteDraft } from '$lib/ndk/drafts';
  import {
    currentDraft,
    openDraft,
    closeDraft,
    markDirty,
    isDirty,
    isLoading,
    setLoading,
    draftError,
    setDraftError
  } from '$lib/features/drafts/state';
  import { readShipyardSession } from '$lib/api/session';

  const draftId = $derived(page.params.id ?? '');

  let session = $state(readShipyardSession());
  let content = $state('');
  let tagsText = $state('[]');
  let targetKind = $state(1);
  let message = $state('');

  function loadContentFromDraft() {
    if (!currentDraft?.inner) return;
    try {
      const ev = currentDraft.inner;
      content = ev.content ?? '';
      tagsText = JSON.stringify(ev.tags ?? []);
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
      await connectNdk();
      const ndk = getNdk();
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
      let tags: string[][] = [];
      try {
        tags = JSON.parse(tagsText) as string[][];
      } catch {
        tags = [];
      }

      const ndk = getNdk();
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
      const ndk = getNdk();
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
    <h1>Edit Draft</h1>
    <small class="muted">{draftId}</small>
  </div>
  <div class="header-actions">
    <a class="secondary-action" href="/drafts">← All Drafts</a>
    <button
      class="danger-action"
      type="button"
      onclick={handleDelete}
      disabled={isLoading}
    >
      Delete
    </button>
  </div>
</header>

{#if draftError}
  <section class="notice error">{draftError}</section>
{/if}
{#if message}
  <section class="notice success">{message}</section>
{/if}

{#if !session.ownerPubkey}
  <section class="notice"><a href="/settings#login">Sign in</a> to manage drafts.</section>
{:else}
  <section class="panel">
    <form class="card-form" onsubmit={handleSave}>
      <div class="section-header">
        <h2>Draft Content</h2>
        <button
          class="primary-action"
          type="submit"
          disabled={isLoading}
        >
          {isDirty ? 'Save *' : 'Save'}
        </button>
      </div>

      <label class="field">
        <span>Target kind</span>
        <input
          type="number"
          bind:value={targetKind}
          min="1"
          onchange={markDirty}
        />
      </label>

      <label class="field">
        <span>Content</span>
        <textarea bind:value={content} rows="12" oninput={markDirty}></textarea>
      </label>

      <label class="field">
        <span>Tags JSON</span>
        <textarea bind:value={tagsText} rows="4" spellcheck="false" oninput={markDirty}></textarea>
      </label>
    </form>
  </section>
{/if}
