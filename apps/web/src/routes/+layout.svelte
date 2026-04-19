<script lang="ts">
  import { onMount, setContext } from 'svelte';
  import '$lib/styles.css';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { NDK_CONTEXT_KEY } from '$lib/ndk/config';

  let { children }: { children: import('svelte').Snippet } = $props();

  setContext(NDK_CONTEXT_KEY, ndk);

  onMount(() => {
    void ensureClientNdk().catch((error) => {
      console.error('Failed to connect client NDK', error);
    });
  });
</script>

{@render children()}
