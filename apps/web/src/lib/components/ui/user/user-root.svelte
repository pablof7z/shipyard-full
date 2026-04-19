<script lang="ts">
  import { setContext } from 'svelte';
  import type { Snippet } from 'svelte';
  import type NDK from '@nostr-dev-kit/ndk';
  import type { NDKUser, NDKUserProfile } from '@nostr-dev-kit/ndk';
  import { createProfileFetcher } from '$lib/components/builders/profile';
  import { cn } from '$lib/utils/cn';
  import { USER_CONTEXT_KEY } from './user.context';

  interface Props {
    ndk: NDK;
    user?: NDKUser;
    npub?: string;
    pubkey?: string;
    profile?: NDKUserProfile;
    onclick?: (event: MouseEvent) => void;
    class?: string;
    children: Snippet;
  }

  let {
    ndk,
    user,
    pubkey,
    npub,
    profile: propProfile,
    onclick,
    class: className = '',
    children
  }: Props = $props();

  const ndkUser = $derived.by(() => {
    if (user) return user;
    if (npub) {
      try {
        return ndk.getUser({ npub });
      } catch {
        return null;
      }
    }
    if (pubkey) {
      try {
        return ndk.getUser({ pubkey });
      } catch {
        return null;
      }
    }
    return null;
  });

  const profileFetcher = createProfileFetcher(
    () => ({ user: propProfile === undefined ? ndkUser : null }),
    () => ndk
  );
  const profile = $derived(propProfile !== undefined ? propProfile : profileFetcher.profile);

  const context = {
    get ndk() {
      return ndk;
    },
    get user() {
      return user;
    },
    get ndkUser() {
      return ndkUser;
    },
    get profile() {
      return profile;
    },
    get onclick() {
      return onclick;
    }
  };

  setContext(USER_CONTEXT_KEY, context);
</script>

{#if onclick}
  <button type="button" data-user-root="" class={cn(className)} {onclick}>
    {@render children()}
  </button>
{:else}
  <div data-user-root="" class={cn(className)}>
    {@render children()}
  </div>
{/if}
