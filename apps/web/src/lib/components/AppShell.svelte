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

  type IconKey =
    | 'dashboard'
    | 'write'
    | 'drafts'
    | 'scheduled'
    | 'queues'
    | 'review'
    | 'published'
    | 'settings';

  const navItems: Array<{ label: string; href: string; icon: IconKey }> = [
    { label: 'Dashboard', href: '/dashboard', icon: 'dashboard' },
    { label: 'Write', href: '/write', icon: 'write' },
    { label: 'Drafts', href: '/drafts', icon: 'drafts' },
    { label: 'Scheduled', href: '/scheduled', icon: 'scheduled' },
    { label: 'Queues', href: '/queues', icon: 'queues' },
    { label: 'Review', href: '/proposals', icon: 'review' },
    { label: 'Published', href: '/published', icon: 'published' },
    { label: 'Settings', href: '/settings', icon: 'settings' }
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
        {#each navItems as item}
          <a class="nav-item" class:active={isActive(item.href)} href={item.href}>
            <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
              {#if item.icon === 'dashboard'}
                <rect x="3" y="3" width="7" height="9" rx="1" />
                <rect x="14" y="3" width="7" height="5" rx="1" />
                <rect x="14" y="12" width="7" height="9" rx="1" />
                <rect x="3" y="16" width="7" height="5" rx="1" />
              {:else if item.icon === 'write'}
                <path d="M12 20h9" />
                <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5z" />
              {:else if item.icon === 'drafts'}
                <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
                <path d="M14 3v6h6" />
                <path d="M8 13h8M8 17h6" />
              {:else if item.icon === 'scheduled'}
                <circle cx="12" cy="12" r="9" />
                <path d="M12 7v5l3 2" />
              {:else if item.icon === 'queues'}
                <path d="M21 7.5L12 3 3 7.5l9 4.5 9-4.5z" />
                <path d="M3 12l9 4.5 9-4.5" />
                <path d="M3 16.5l9 4.5 9-4.5" />
              {:else if item.icon === 'review'}
                <path d="M20 6L9 17l-5-5" />
                <path d="M22 12l-2 2" />
              {:else if item.icon === 'published'}
                <path d="M22 2L11 13" />
                <path d="M22 2l-7 20-4-9-9-4 20-7z" />
              {:else if item.icon === 'settings'}
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 0 1-4 0v-.09a1.7 1.7 0 0 0-1.11-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 0 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.11 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34H9a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 0 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87V9a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 0 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03z" />
              {/if}
            </svg>
            <span>{item.label}</span>
          </a>
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
