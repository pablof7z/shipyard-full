<script lang="ts">
  import { onMount } from 'svelte';
  import { shipyardApi } from '$lib/api/client';
  import { readShipyardSession, type ShipyardSession } from '$lib/api/session';
  import type { PublishTrigger, Queue } from '$lib/api/types';
  import { createDraftId, type DraftSourceEvent } from '$lib/nostr/drafts';
  import {
    readLocalDraftWraps,
    removeLocalDraftWrap,
    type LocalDraftWrapRecord
  } from '$lib/nostr/local-drafts';
  import ComposerDrawer from './ComposerDrawer.svelte';
  import ComposerNoteList from './ComposerNoteList.svelte';
  import ComposerScheduleBar from './ComposerScheduleBar.svelte';
  import ComposerTopbar from './ComposerTopbar.svelte';
  import { apiErrorMessage, toLocalInput } from './composer-actions';
  import { submitComposition as submitCompositionRequest } from './composer-submit';
  import { blankDraftWrap, loadDraftWrap, saveDraftWrap } from './composer-drafts';
  import {
    contentFromNotes,
    createComposerNote,
    notesFromContent,
    type ComposerDrawerState,
    type ComposerNote
  } from './composer';

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let queues = $state<Queue[]>([]);
  let relayUrls = $state<string[]>([]);
  let notes = $state<ComposerNote[]>([createComposerNote()]);
  let activeNoteIndex = $state(0);
  let trigger = $state<PublishTrigger>('TIME');
  let publishAt = $state(toLocalInput(new Date(Date.now() + 60 * 60 * 1000)));
  let queueId = $state('');
  let tagsText = $state('[]');
  let draftId = $state(createDraftId());
  let draftRecords = $state<LocalDraftWrapRecord[]>([]);
  let drawer = $state<ComposerDrawerState>('none');
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');

  const activeQueues = $derived(queues.filter((queue) => !queue.archived_at));
  const content = $derived(contentFromNotes(notes));
  const activeCharCount = $derived(notes[activeNoteIndex]?.content.length ?? 0);
  const submitLabel = $derived(
    trigger === 'SEND_NOW' ? 'Send now' : trigger === 'QUEUE' ? 'Add to queue' : 'Schedule'
  );

  function setMessage(value: string) {
    message = value;
    error = '';
  }

  function setError(err: unknown, fallback: string) {
    error = apiErrorMessage(err, fallback);
    message = '';
  }

  async function submitComposition() {
    saving = true;
    try {
      if (trigger !== 'SEND_NOW' && (!session.token || !session.ownerPubkey)) {
        throw new Error('Sign in before you can schedule.');
      }
      if (!content.trim()) return;
      const ownerPubkey = session.ownerPubkey || (await window.nostr?.getPublicKey?.()) || '';
      if (!ownerPubkey) {
        throw new Error('Sign in to send posts.');
      }

      const result = await submitCompositionRequest({
        token: session.token,
        ownerPubkey,
        trigger,
        publishAt,
        queueId,
        relayUrls,
        content,
        tagsText
      });
      setMessage(
        result === 'published'
          ? 'Published.'
          : result === 'signed'
            ? trigger === 'QUEUE'
              ? 'Added to queue.'
              : 'Scheduled.'
            : 'Sent for review.'
      );
      notes = [createComposerNote()];
      tagsText = '[]';
      activeNoteIndex = 0;
    } catch (err) {
      setError(err, "Couldn't publish. Try again?");
    } finally {
      saving = false;
    }
  }

  async function saveDraft() {
    saving = true;
    try {
      draftId = await saveDraftWrap({
        ownerPubkey: session.ownerPubkey,
        draftId,
        content,
        tagsText
      });
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
      const draft = await loadDraftWrap(record);
      draftId = record.draftId;
      loadDraftIntoComposer(draft);
      setMessage('Draft loaded.');
    } catch (err) {
      setError(err, "Couldn't load draft.");
    } finally {
      saving = false;
    }
  }

  async function blankDraft(record: LocalDraftWrapRecord) {
    saving = true;
    try {
      await blankDraftWrap(session.ownerPubkey, record);
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
  }

  function loadDraftIntoComposer(draft: DraftSourceEvent) {
    notes = notesFromContent(draft.content);
    tagsText = JSON.stringify(draft.tags ?? [], null, 2);
    activeNoteIndex = 0;
    drawer = 'none';
  }

  function insertBlossomUrl(url: string) {
    const index = Math.min(activeNoteIndex, notes.length - 1);
    const current = notes[index]?.content ?? '';
    updateNote(index, `${current}${current.trim() ? '\n' : ''}${url}`);
  }

  function updateNote(index: number, value: string) {
    notes = notes.map((note, noteIndex) => (noteIndex === index ? { ...note, content: value } : note));
  }

  function addNote() {
    notes = [...notes, createComposerNote()];
    activeNoteIndex = notes.length - 1;
  }

  function removeNote(index: number) {
    notes = notes.filter((_, noteIndex) => noteIndex !== index);
    if (!notes.length) notes = [createComposerNote()];
    activeNoteIndex = Math.min(activeNoteIndex, notes.length - 1);
  }

  function toggleDrawer(next: Exclude<ComposerDrawerState, 'none'>) {
    drawer = drawer === next ? 'none' : next;
  }

  async function loadWriteContext() {
    session = readShipyardSession();
    loading = true;
    try {
      if (session.token && session.ownerPubkey) {
        const [queueResponse, relayResponse] = await Promise.all([
          shipyardApi.queues(session.token, session.ownerPubkey),
          shipyardApi.relays(session.token, session.ownerPubkey)
        ]);
        queues = queueResponse;
        relayUrls = relayResponse.relay_urls;
      } else {
        queues = [];
        relayUrls = [];
      }
      queueId = queueId || activeQueues[0]?.id || '';
      refreshDrafts();
    } catch (err) {
      setError(err, "Couldn't load your workspace.");
    } finally {
      loading = false;
    }
  }

  function refreshDrafts() {
    draftRecords = readLocalDraftWraps(session.ownerPubkey);
  }

  $effect(() => {
    if (trigger === 'QUEUE' && !queueId) queueId = activeQueues[0]?.id ?? '';
  });

  onMount(() => {
    loadWriteContext();
    window.addEventListener('shipyard-local-drafts-updated', refreshDrafts);
    return () => window.removeEventListener('shipyard-local-drafts-updated', refreshDrafts);
  });
</script>

<div class="composer-page">
  <ComposerTopbar
    {activeCharCount}
    canSaveDraft={Boolean(content.trim())}
    {error}
    {message}
    notesCount={notes.length}
    ownerPubkey={session.ownerPubkey}
    {saving}
    onSaveDraft={saveDraft}
  />

  {#if trigger !== 'SEND_NOW' && (!session.token || !session.ownerPubkey)}
    <section class="composer-notice">Sign in to schedule posts.</section>
  {/if}

  <ComposerNoteList
    {notes}
    activeIndex={activeNoteIndex}
    onAddNote={addNote}
    onFocusNote={(index) => (activeNoteIndex = index)}
    onRemoveNote={removeNote}
    onUpdateNote={updateNote}
  />

  {#if drawer !== 'none'}
    <ComposerDrawer
      {draftRecords}
      drawer={drawer}
      {saving}
      onBlankDraft={blankDraft}
      onForgetDraft={forgetDraft}
      onInsertUrl={insertBlossomUrl}
      onLoadDraft={loadDraft}
      onRefreshDrafts={refreshDrafts}
    />
  {/if}

  <ComposerScheduleBar
    activeDrawer={drawer}
    {activeQueues}
    disabled={loading || !content.trim() || (trigger === 'QUEUE' && !queueId)}
    {publishAt}
    {queueId}
    {saving}
    {submitLabel}
    {trigger}
    onPublishAtChange={(value) => (publishAt = value)}
    onQueueChange={(value) => (queueId = value)}
    onSubmit={submitComposition}
    onToggleDrawer={toggleDrawer}
    onTriggerChange={(value) => (trigger = value)}
  />
</div>
