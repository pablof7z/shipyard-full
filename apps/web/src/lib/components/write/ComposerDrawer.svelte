<script lang="ts">
  import type { LocalDraftWrapRecord } from '$lib/nostr/local-drafts';
  import BlossomUploadPanel from './BlossomUploadPanel.svelte';
  import type { ComposerDrawerState } from './composer';

  let {
    draftId,
    draftRecords,
    drawer,
    saving,
    onBlankDraft,
    onDraftIdChange,
    onForgetDraft,
    onInsertUrl,
    onLoadDraft,
    onRefreshDrafts
  }: {
    draftId: string;
    draftRecords: LocalDraftWrapRecord[];
    drawer: Exclude<ComposerDrawerState, 'none'>;
    saving: boolean;
    onBlankDraft: (record: LocalDraftWrapRecord) => void;
    onDraftIdChange: (value: string) => void;
    onForgetDraft: (record: LocalDraftWrapRecord) => void;
    onInsertUrl: (url: string) => void;
    onLoadDraft: (record: LocalDraftWrapRecord) => void;
    onRefreshDrafts: () => void;
  } = $props();

  function inputValue(event: Event) {
    return (event.currentTarget as HTMLInputElement).value;
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
  {/if}
</aside>
