<script lang="ts">
  import '$lib/styles.css';
  import AppShell from '$lib/components/AppShell.svelte';

  let { children }: { children: import('svelte').Snippet } = $props();
</script>

<AppShell>
  {@render children()}
</AppShell>
