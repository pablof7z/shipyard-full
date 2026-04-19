<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import {
    compactPubkey,
    readShipyardSession,
    sessionUpdatedEvent,
    type ShipyardSession
  } from '$lib/api/session';
  import { connectNdk, getNdk } from '$lib/ndk/client';
  import { User } from '$lib/components/ui/user';
  import type NDK from '@nostr-dev-kit/ndk';

  let { children }: { children: import('svelte').Snippet } = $props();

  const navItems = [
    ['Dashboard', '/dashboard'],
    ['Write', '/write'],
    ['Drafts', '/drafts'],
    ['Scheduled', '/scheduled'],
    ['Queues', '/queues'],
    ['Proposals', '/proposals'],
    ['Published', '/published'],
    ['Settings', '/settings']
  ];

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  let ndk = $state<NDK | null>(null);
  const isComposer = $derived(page.url.pathname === '/write');

  function refreshSession() {
    session = readShipyardSession();
  }

  function isActive(href: string) {
    return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
  }

  onMount(() => {
    ndk = getNdk();
    connectNdk().catch(() => {
      // Profile metadata is a progressive enhancement; keep the shell usable offline.
    });
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
      <a class="brand" href="/dashboard" aria-label="Shipyard dashboard">
        <span class="brand-mark" aria-hidden="true"></span>
        <span>Shipyard</span>
      </a>

      <nav class="nav" aria-label="Main navigation">
        {#each navItems as [label, href]}
          <a class="nav-item" class:active={isActive(href)} {href}>{label}</a>
        {/each}
      </nav>

      <div class="account-pill" aria-label="Active account">
        {#if ndk && session.ownerPubkey}
          <User.Root {ndk} pubkey={session.ownerPubkey} class="account-profile">
            <User.Avatar class="avatar" alt="Active account avatar" />
            <span class="account-copy">
              <strong>
                <User.Name fallback={compactPubkey(session.ownerPubkey)} />
              </strong>
              <small>{session.token ? 'Session configured' : 'No session'}</small>
              {#if !session.token}
                <a class="account-signin" href="/settings#login">Sign in</a>
              {/if}
            </span>
          </User.Root>
        {:else}
          <span class="avatar" aria-hidden="true">-</span>
          <span class="account-copy">
            <strong>
              {compactPubkey(session.ownerPubkey)}
            </strong>
            <small>{session.token ? 'Session configured' : 'No session'}</small>
            {#if !session.token}
              <a class="account-signin" href="/settings#login">Sign in</a>
            {/if}
          </span>
        {/if}
      </div>
    </aside>
  {/if}

  <main class="main">
    {@render children()}
  </main>
</div>
