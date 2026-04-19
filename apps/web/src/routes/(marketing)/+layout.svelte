<script lang="ts">
  import LoginModal from '$lib/components/onboarding/LoginModal.svelte';

  let { children }: { children: import('svelte').Snippet } = $props();
</script>

{@render children()}
<LoginModal />
