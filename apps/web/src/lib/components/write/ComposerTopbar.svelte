<script lang="ts">
  import { compactPubkey } from '$lib/api/session';

  let {
    activeCharCount,
    canSaveDraft,
    error,
    message,
    notesCount,
    ownerPubkey,
    saving,
    onSaveDraft
  }: {
    activeCharCount: number;
    canSaveDraft: boolean;
    error: string;
    message: string;
    notesCount: number;
    ownerPubkey: string;
    saving: boolean;
    onSaveDraft: () => void;
  } = $props();
</script>

<header class="composer-topbar">
  <div class="composer-topbar-left">
    <a class="composer-icon-btn" href="/dashboard" aria-label="Back to dashboard">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M9.5 3l-5 5 5 5" /></svg>
    </a>
    <div class="composer-account-pill">
      <span class="dot" aria-hidden="true"></span>
      <span>{ownerPubkey ? compactPubkey(ownerPubkey) : 'No account'}</span>
    </div>
    {#if notesCount > 1}
      <div class="thread-counter">Thread · {notesCount} notes</div>
    {/if}
  </div>

  <div class="composer-topbar-right">
    {#if message || error}
      <span class:error-text={Boolean(error)} class:success-text={Boolean(message)}>
        {error || message}
      </span>
    {/if}
    <button class="secondary-action" type="button" onclick={onSaveDraft} disabled={saving || !canSaveDraft}>
      Save Draft
    </button>
    <div class="char-count">{activeCharCount} / 280</div>
  </div>
</header>
