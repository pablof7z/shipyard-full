<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import {
    compactPubkey,
    readShipyardSession,
    sessionUpdatedEvent,
    type ShipyardSession
  } from '$lib/api/session';

  let { children }: { children: import('svelte').Snippet } = $props();

  const navItems = [
    ['Dashboard', '/'],
    ['Write', '/write'],
    ['Drafts', '/drafts'],
    ['Scheduled', '/scheduled'],
    ['Queues', '/queues'],
    ['Proposals', '/proposals'],
    ['DVM', '/dvm'],
    ['Published', '/published'],
    ['Settings', '/settings']
  ];

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  const isComposer = $derived(page.url.pathname === '/write');

  function refreshSession() {
    session = readShipyardSession();
  }

  function isActive(href: string) {
    if (href === '/') {
      return page.url.pathname === '/';
    }

    return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
  }

  onMount(() => {
    refreshSession();
    window.addEventListener(sessionUpdatedEvent, refreshSession);

    return () => {
      window.removeEventListener(sessionUpdatedEvent, refreshSession);
    };
  });
</script>

<div class="app-shell" class:composer-mode={isComposer}>
  {#if !isComposer}
    <aside class="sidebar">
      <a class="brand" href="/" aria-label="Shipyard dashboard">
        <span class="brand-mark" aria-hidden="true"></span>
        <span>Shipyard</span>
      </a>

      <nav class="nav" aria-label="Main navigation">
        {#each navItems as [label, href]}
          <a class="nav-item" class:active={isActive(href)} {href}>{label}</a>
        {/each}
      </nav>

      <div class="account-pill" aria-label="Active account">
        <span class="avatar">{session.ownerPubkey ? session.ownerPubkey.slice(0, 1).toUpperCase() : '-'}</span>
        <span class="account-copy">
          <strong>{compactPubkey(session.ownerPubkey)}</strong>
          <small>{session.token ? 'Session configured' : 'No session'}</small>
          {#if !session.token}
            <a class="account-signin" href="/settings#login">Sign in</a>
          {/if}
        </span>
      </div>
    </aside>
  {/if}

  <main class="main">
    {@render children()}
  </main>
</div>
