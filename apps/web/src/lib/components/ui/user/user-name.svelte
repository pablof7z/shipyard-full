<script lang="ts">
  import { getContext } from 'svelte';
  import { cn } from '$lib/utils/cn';
  import { compactPubkey } from '$lib/api/session';
  import { USER_CONTEXT_KEY, type UserContext } from './user.context';

  interface Props {
    field?: 'displayName' | 'name' | 'both';
    fallback?: string;
    class?: string;
  }

  let { field = 'displayName', fallback, class: className = '' }: Props = $props();

  const context = getContext<UserContext>(USER_CONTEXT_KEY);
  if (!context) {
    throw new Error('User.Name must be used within User.Root');
  }

  const userPubkey = $derived(context.ndkUser?.pubkey);
  const fallbackName = $derived(fallback ?? (userPubkey ? compactPubkey(userPubkey) : 'Unknown'));

  const displayText = $derived.by(() => {
    const profile = context.profile;
    if (!profile) return fallbackName;

    if (field === 'name') {
      return profile.name || profile.displayName || profile.nip05 || fallbackName;
    }

    if (field === 'both') {
      const primary = profile.displayName || profile.name;
      const secondary =
        profile.displayName && profile.name && profile.name !== profile.displayName ? profile.name : null;
      return secondary ? `${primary} (@${secondary})` : primary || profile.nip05 || fallbackName;
    }

    return profile.displayName || profile.name || profile.nip05 || fallbackName;
  });
</script>

<span data-user-name="" class={cn(className)}>
  {displayText}
</span>
