<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import {
    compactPubkey,
    readShipyardSession,
    sessionUpdatedEvent,
    type ShipyardSession
  } from '$lib/api/session';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { User } from '$lib/components/ui/user';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';

  let { children }: { children: import('svelte').Snippet } = $props();

  const navItems = [
    ['Dashboard', '/dashboard'],
    ['Write', '/write'],
    ['Drafts', '/drafts'],
    ['Scheduled', '/scheduled'],
    ['Queues', '/queues'],
    ['Review', '/proposals'],
    ['Published', '/published'],
    ['Settings', '/settings']
  ];

  let session = $state<ShipyardSession>({ token: '', ownerPubkey: '' });
  const isComposer = $derived(page.url.pathname === '/write');

  function refreshSession() {
    session = readShipyardSession();
  }

  function isActive(href: string) {
    return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
  }

  onMount(() => {
    void ensureClientNdk().catch(() => {
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
        {#if session.token && session.ownerPubkey}
          <User.Root {ndk} pubkey={session.ownerPubkey} class="account-profile">
            <User.Avatar class="avatar" alt="Active account avatar" />
            <span class="account-copy">
              <strong>
                <User.Name fallback={compactPubkey(session.ownerPubkey)} />
              </strong>
            </span>
          </User.Root>
        {:else}
          <button
            class="account-signin primary-action"
            type="button"
            onclick={() => loginModal.show()}
          >
            Sign in
          </button>
        {/if}
      </div>
    </aside>
  {/if}

  <main class="main">
    {@render children()}
  </main>
</div>
