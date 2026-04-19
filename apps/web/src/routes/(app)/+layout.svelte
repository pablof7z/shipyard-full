<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import AppShell from '$lib/components/AppShell.svelte';
  import LoginModal from '$lib/components/onboarding/LoginModal.svelte';
  import { readShipyardSession, sessionUpdatedEvent } from '$lib/api/session';

  let { children }: { children: import('svelte').Snippet } = $props();

  let ready = $state(false);

  function guard() {
    const { token, ownerPubkey } = readShipyardSession();
    if (!token || !ownerPubkey) {
      goto('/', { replaceState: true });
      return false;
    }
    return true;
  }

  onMount(() => {
    ready = guard();
    const onSessionChange = () => {
      ready = guard();
    };
    window.addEventListener(sessionUpdatedEvent, onSessionChange);
    return () => window.removeEventListener(sessionUpdatedEvent, onSessionChange);
  });
</script>

{#if ready}
  <AppShell>
    {@render children()}
  </AppShell>
{/if}

<LoginModal />
