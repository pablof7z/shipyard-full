<script lang="ts">
  import { ndk } from '$lib/ndk/client';
  import { User } from '$lib/components/ui/user';

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
    {#if ownerPubkey}
      <User.Root {ndk} pubkey={ownerPubkey} class="composer-account-pill">
        <User.Avatar class="composer-account-avatar" alt="" />
        <User.Name fallback="You" />
      </User.Root>
    {:else}
      <div class="composer-account-pill signed-out">Not signed in</div>
    {/if}
    {#if notesCount > 1}
      <div class="thread-counter">Thread · {notesCount} posts</div>
    {/if}
  </div>

  <div class="composer-topbar-right">
    {#if message || error}
      <span class:error-text={Boolean(error)} class:success-text={Boolean(message)}>
        {error || message}
      </span>
    {/if}
    <button class="secondary-action" type="button" onclick={onSaveDraft} disabled={saving || !canSaveDraft}>
      Save draft
    </button>
    <div class="char-count">{activeCharCount} / 280</div>
  </div>
</header>

<style>
  :global(.composer-account-avatar) {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--bg-tertiary);
  }
</style>
