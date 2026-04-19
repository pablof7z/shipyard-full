<script lang="ts">
  import { onMount } from 'svelte';
  import {
    createDraftId,
    createSignedBlankDraftWrap,
    createSignedDraftWrap,
    decryptDraftWrap,
    type DraftSourceEvent
  } from '$lib/nostr/drafts';
  import {
    readLocalDraftWraps,
    removeLocalDraftWrap,
    upsertLocalDraftWrap,
    type LocalDraftWrapRecord
  } from '$lib/nostr/local-drafts';

  let {
    content,
    tagsText,
    ownerPubkey,
    onLoadDraft
  }: {
    content: string;
    tagsText: string;
    ownerPubkey: string;
    onLoadDraft: (draft: DraftSourceEvent, draftId: string) => void;
  } = $props();

  let draftId = $state(createDraftId());
  let records = $state<LocalDraftWrapRecord[]>([]);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  function refreshDrafts() {
    records = readLocalDraftWraps(ownerPubkey);
  }

  function setMessage(value: string) {
    message = value;
    error = '';
  }

  function setError(err: unknown, fallback: string) {
    error = err instanceof Error ? err.message : fallback;
    message = '';
  }

  function parseTags() {
    const tags = JSON.parse(tagsText) as string[][];
    if (!Array.isArray(tags) || tags.some((tag) => !Array.isArray(tag))) {
      throw new Error('Tags JSON must be an array of tag arrays.');
    }

    return tags;
  }

  async function signerPubkey() {
    const pubkey = await window.nostr?.getPublicKey?.();
    if (!pubkey) {
      throw new Error('You need a browser signer to save drafts.');
    }

    if (ownerPubkey && pubkey !== ownerPubkey) {
      throw new Error("Sign in with the account you're posting from.");
    }

    return pubkey;
  }

  async function saveDraftWrap() {
    saving = true;
    try {
      const pubkey = await signerPubkey();
      const activeDraftId = draftId.trim() || createDraftId();
      const signed = await createSignedDraftWrap({
        signer: window.nostr,
        pubkey,
        draftId: activeDraftId,
        draft: {
          kind: 1,
          content,
          tags: parseTags()
        }
      });
      draftId = activeDraftId;
      upsertLocalDraftWrap(signed);
      refreshDrafts();
      setMessage('Draft saved.');
    } catch (err) {
      setError(err, "Couldn't save draft.");
    } finally {
      saving = false;
    }
  }

  async function loadDraft(record: LocalDraftWrapRecord) {
    saving = true;
    try {
      const draft = await decryptDraftWrap(window.nostr, record.event);
      draftId = record.draftId;
      onLoadDraft(draft, record.draftId);
      setMessage('Draft loaded.');
    } catch (err) {
      setError(err, "Couldn't load draft.");
    } finally {
      saving = false;
    }
  }

  async function blankDelete(record: LocalDraftWrapRecord) {
    saving = true;
    try {
      const pubkey = await signerPubkey();
      if (pubkey !== record.ownerPubkey) {
        throw new Error("Sign in with the account you're posting from.");
      }

      const signed = await createSignedBlankDraftWrap({
        signer: window.nostr,
        pubkey,
        draftId: record.draftId,
        draftKind: record.targetKind
      });
      upsertLocalDraftWrap(signed);
      refreshDrafts();
      setMessage('Draft cleared.');
    } catch (err) {
      setError(err, "Couldn't clear draft.");
    } finally {
      saving = false;
    }
  }

  function forgetDraft(record: LocalDraftWrapRecord) {
    removeLocalDraftWrap(record.ownerPubkey, record.draftId);
    refreshDrafts();
    setMessage('Draft removed from this device.');
  }

  $effect(() => {
    ownerPubkey;
    refreshDrafts();
  });

  onMount(() => {
    refreshDrafts();
    window.addEventListener('shipyard-local-drafts-updated', refreshDrafts);

    return () => {
      window.removeEventListener('shipyard-local-drafts-updated', refreshDrafts);
    };
  });
</script>

<div class="card-form">
  <div class="section-header">
    <h2>Drafts</h2>
    <button class="primary-action" type="button" onclick={saveDraftWrap} disabled={saving}>
      Save draft
    </button>
  </div>

  {#if message}
    <p class="meta-line success-text">{message}</p>
  {/if}
  {#if error}
    <p class="meta-line error-text">{error}</p>
  {/if}

  <div class="rows compact">
    {#if !records.length}
      <article class="row">
        <p>No drafts yet.</p>
      </article>
    {:else}
      {#each records as record}
        <article class="row draft-row">
          <p>
            <strong>Draft</strong>
            <span>{new Date(record.updatedAt).toLocaleString()}</span>
          </p>
          {#if record.deleted}
            <span class="muted-text">Cleared</span>
          {/if}
          <div class="inline-actions">
            <button
              class="secondary-action"
              type="button"
              onclick={() => loadDraft(record)}
              disabled={saving || record.deleted}
            >
              Load
            </button>
            <button
              class="danger-action"
              type="button"
              onclick={() => blankDelete(record)}
              disabled={saving || record.deleted}
            >
              Clear
            </button>
            <button class="secondary-action" type="button" onclick={() => forgetDraft(record)}>
              Remove from this device
            </button>
          </div>
        </article>
      {/each}
    {/if}
  </div>
</div>
