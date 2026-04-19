<script lang="ts">
  import type { LocalDraftWrapRecord } from '$lib/nostr/local-drafts';
  import BlossomUploadPanel from './BlossomUploadPanel.svelte';
  import type { ComposerDrawerState } from './composer';

  let {
    draftRecords,
    drawer,
    saving,
    onBlankDraft,
    onForgetDraft,
    onInsertUrl,
    onLoadDraft,
    onRefreshDrafts
  }: {
    draftRecords: LocalDraftWrapRecord[];
    drawer: Exclude<ComposerDrawerState, 'none'>;
    saving: boolean;
    onBlankDraft: (record: LocalDraftWrapRecord) => void;
    onForgetDraft: (record: LocalDraftWrapRecord) => void;
    onInsertUrl: (url: string) => void;
    onLoadDraft: (record: LocalDraftWrapRecord) => void;
    onRefreshDrafts: () => void;
  } = $props();

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
      <div class="rows compact">
        {#if !draftRecords.length}
          <article class="row"><p>No drafts yet.</p></article>
        {:else}
          {#each draftRecords as record}
            <article class="row draft-row">
              <p>
                <strong>Draft</strong>
                <span>{new Date(record.updatedAt).toLocaleString()}</span>
              </p>
              <div class="inline-actions">
                <button class="secondary-action" type="button" onclick={() => onLoadDraft(record)} disabled={record.deleted || saving}>
                  Load
                </button>
                <button class="danger-action" type="button" onclick={() => onBlankDraft(record)} disabled={record.deleted || saving}>
                  Clear
                </button>
                <button class="secondary-action" type="button" onclick={() => onForgetDraft(record)}>
                  Remove from device
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</aside>
