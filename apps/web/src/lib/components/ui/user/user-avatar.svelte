<script lang="ts">
  import { getContext } from 'svelte';
  import type { Snippet } from 'svelte';
  import { deterministicPubkeyGradient } from '$lib/utils/deterministic-gradient';
  import { cn } from '$lib/utils/cn';
  import { USER_CONTEXT_KEY, type UserContext } from './user.context';

  interface Props {
    class?: string;
    fallback?: string;
    alt?: string;
    customFallback?: Snippet;
  }

  let { class: className = '', fallback, alt, customFallback }: Props = $props();

  const context = getContext<UserContext>(USER_CONTEXT_KEY);
  if (!context) {
    throw new Error('User.Avatar must be used within User.Root');
  }

  const imageUrl = $derived(context.profile?.picture || context.profile?.image || fallback);
  const avatarGradient = $derived(
    context.ndkUser?.pubkey ? deterministicPubkeyGradient(context.ndkUser.pubkey) : 'var(--accent)'
  );
  const fallbackText = $derived(context.ndkUser?.pubkey?.slice(0, 2).toUpperCase() ?? '--');

  let imageLoaded = $state(false);
  let imageError = $state(false);

  function handleImageLoad() {
    imageLoaded = true;
    imageError = false;
  }

  function handleImageError() {
    imageLoaded = false;
    imageError = true;
  }

  $effect(() => {
    imageUrl;
    imageLoaded = false;
    imageError = false;
  });
</script>

<div data-user-avatar="" class={cn(className)}>
  {#if !imageLoaded || !imageUrl || imageError}
    {#if customFallback}
      {@render customFallback()}
    {:else}
      <div data-user-avatar-fallback="" style:background={avatarGradient}>
        {fallbackText}
      </div>
    {/if}
  {/if}

  {#if imageUrl}
    <img
      data-user-avatar-img=""
      src={imageUrl}
      {alt}
      class:image-loaded={imageLoaded}
      onload={handleImageLoad}
      onerror={handleImageError}
    />
  {/if}
</div>
