<script lang="ts">
  import type { LocalDraftWrapRecord } from '$lib/nostr/local-drafts';
  import BlossomUploadPanel from './BlossomUploadPanel.svelte';
  import type { ComposerDrawerState } from './composer';

  let {
    draftId,
    draftRecords,
    drawer,
    saving,
    signedEventText,
    tagsText,
    onBlankDraft,
    onDraftIdChange,
    onForgetDraft,
    onInsertUrl,
    onLoadDraft,
    onRefreshDrafts,
    onScheduleSignedJson,
    onSignedEventTextChange,
    onTagsTextChange
  }: {
    draftId: string;
    draftRecords: LocalDraftWrapRecord[];
    drawer: ComposerDrawerState;
    saving: boolean;
    signedEventText: string;
    tagsText: string;
    onBlankDraft: (record: LocalDraftWrapRecord) => void;
    onDraftIdChange: (value: string) => void;
    onForgetDraft: (record: LocalDraftWrapRecord) => void;
    onInsertUrl: (url: string) => void;
    onLoadDraft: (record: LocalDraftWrapRecord) => void;
    onRefreshDrafts: () => void;
    onScheduleSignedJson: (event: SubmitEvent) => void;
    onSignedEventTextChange: (value: string) => void;
    onTagsTextChange: (value: string) => void;
  } = $props();

  function inputValue(event: Event) {
    return (event.currentTarget as HTMLInputElement | HTMLTextAreaElement).value;
  }
</script>

<aside class="composer-drawer" aria-label="Composer drawer">
  {#if drawer === 'media'}
    <BlossomUploadPanel onInsertUrl={onInsertUrl} />
  {:else if drawer === 'drafts'}
    <div class="composer-drawer-panel">
      <div class="section-header">
        <h2>Drafts</h2>
        <button class="secondary-action" type="button" onclick={onRefreshDrafts}>Refresh</button>
      </div>
      <label class="field">
        <span>Draft identifier</span>
        <input value={draftId} autocomplete="off" oninput={(event) => onDraftIdChange(inputValue(event))} />
      </label>
      <div class="rows compact">
        {#if !draftRecords.length}
          <article class="row"><p>No local draft wraps.</p></article>
        {:else}
          {#each draftRecords as record}
            <article class="row draft-row">
              <p>
                <strong>{record.draftId}</strong>
                <span>{new Date(record.updatedAt).toLocaleString()}</span>
              </p>
              <div class="inline-actions">
                <button class="secondary-action" type="button" onclick={() => onLoadDraft(record)} disabled={record.deleted || saving}>
                  Load
                </button>
                <button class="danger-action" type="button" onclick={() => onBlankDraft(record)} disabled={record.deleted || saving}>
                  Blank
                </button>
                <button class="secondary-action" type="button" onclick={() => onForgetDraft(record)}>
                  Forget
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  {:else}
    <div class="composer-drawer-panel">
      <label class="field">
        <span>Tags JSON</span>
        <textarea value={tagsText} rows="4" spellcheck="false" oninput={(event) => onTagsTextChange(inputValue(event))}></textarea>
      </label>
      <form class="inner-form" onsubmit={onScheduleSignedJson}>
        <label class="field">
          <span>Signed event JSON</span>
          <textarea
            value={signedEventText}
            rows="8"
            spellcheck="false"
            oninput={(event) => onSignedEventTextChange(inputValue(event))}
          ></textarea>
        </label>
        <div class="inline-actions">
          <button class="secondary-action" type="submit" disabled={saving || !signedEventText.trim()}>
            Schedule signed event
          </button>
        </div>
      </form>
    </div>
  {/if}
</aside>
