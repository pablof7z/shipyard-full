<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { readShipyardSession, sessionUpdatedEvent } from '$lib/api/session';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import MarketingBands from '$lib/components/marketing/MarketingBands.svelte';
  import MarketingCli from '$lib/components/marketing/MarketingCli.svelte';
  import MarketingClosing from '$lib/components/marketing/MarketingClosing.svelte';
  import MarketingDemo from '$lib/components/marketing/MarketingDemo.svelte';
  import MarketingHero from '$lib/components/marketing/MarketingHero.svelte';
  import MarketingNav from '$lib/components/marketing/MarketingNav.svelte';
  import MarketingPlatforms from '$lib/components/marketing/MarketingPlatforms.svelte';

  let authed = $state(false);

  function refreshSession() {
    const session = readShipyardSession();
    authed = Boolean(session.token && session.ownerPubkey);
  }

  function handlePrimaryCta(event: MouseEvent) {
    event.preventDefault();
    if (authed) {
      goto('/write');
    } else {
      loginModal.show();
    }
  }

  function toggleTheme() {
    const root = document.documentElement;
    const next = root.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    root.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
  }

  onMount(() => {
    refreshSession();
    window.addEventListener(sessionUpdatedEvent, refreshSession);

    const saved = localStorage.getItem('theme');
    if (saved) {
      document.documentElement.setAttribute('data-theme', saved);
    } else if (!document.documentElement.getAttribute('data-theme')) {
      const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
      document.documentElement.setAttribute('data-theme', prefersLight ? 'light' : 'dark');
    }

    return () => {
      window.removeEventListener(sessionUpdatedEvent, refreshSession);
    };
  });

  const ctaLabel = $derived(authed ? 'Open app' : 'Sign in with Nostr');
  const ctaHref = $derived(authed ? '/write' : '#');
</script>

<svelte:head>
  <title>Shipyard - Schedule your Nostr posts</title>
  <meta
    name="description"
    content="Schedule Nostr posts, run content queues, boost older notes for new timezones, and let teammates draft while you approve. Web, mobile, and CLI."
  />
</svelte:head>

<div class="marketing">
  <MarketingNav {ctaLabel} {ctaHref} onPrimaryCta={handlePrimaryCta} onToggleTheme={toggleTheme} />
  <MarketingHero {ctaLabel} {ctaHref} onPrimaryCta={handlePrimaryCta} />
  <MarketingDemo {ctaLabel} {ctaHref} onPrimaryCta={handlePrimaryCta} />
  <MarketingBands />
  <MarketingPlatforms />
  <MarketingCli />
  <MarketingClosing {ctaLabel} {ctaHref} onPrimaryCta={handlePrimaryCta} />
</div>

<style>
  :global(html) {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  .marketing {
    --accent: #f06449;
    --accent-hover: #f4806a;
    --accent-text-on: #ffffff;
    --bg-primary: #0c0c0c;
    --bg-secondary: #111111;
    --bg-tertiary: #161616;
    --border-subtle: #1e1e1e;
    --border-default: #262626;
    --border-strong: #333333;
    --text-primary: #ededec;
    --text-secondary: #a1a1a0;
    --text-tertiary: #636362;
    --text-muted: #4a4a49;
    --text-ghost: #1e1e1e;
    background: var(--bg-primary);
    color: var(--text-primary);
    min-height: 100vh;
    overflow-x: hidden;
  }

  :global([data-theme='light']) .marketing {
    --bg-primary: #ffffff;
    --bg-secondary: #fafafa;
    --bg-tertiary: #f5f5f4;
    --border-subtle: #e8e8e7;
    --border-default: #e0e0df;
    --border-strong: #d0d0cf;
    --text-primary: #1a1a19;
    --text-secondary: #5c5c5b;
    --text-tertiary: #8a8a89;
    --text-muted: #b0b0af;
    --text-ghost: #f0f0ef;
  }
</style>
