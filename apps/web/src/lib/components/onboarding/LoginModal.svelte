<script lang="ts">
  import { NDKNip07Signer, NDKPrivateKeySigner, NDKNip46Signer, nip19 } from '@nostr-dev-kit/ndk';
  import { env } from '$env/dynamic/public';
  import { goto } from '$app/navigation';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { shipyardApi, shipyardApiBase } from '$lib/api/client';
  import { writeShipyardSession } from '$lib/api/session';
  import { signNostrEventWithNdk } from '$lib/nostr/signing';
  import type { ApiErrorBody } from '$lib/api/types';
  import { loginModal } from './loginState.svelte';
  import './login-modal.css';

  const authDomain = env.PUBLIC_SHIPYARD_AUTH_DOMAIN ?? 'localhost';
  const authUrl = env.PUBLIC_SHIPYARD_AUTH_URL ?? `${shipyardApiBase}/v1/auth/login`;

  let credentialInput = $state('');
  let error = $state('');
  let isLoading = $state(false);
  let hasExtension = $state(false);

  const credentialType = $derived.by(() => {
    const trimmed = credentialInput.trim();
    if (!trimmed) return null;
    if (trimmed.startsWith('nsec1')) return 'nsec';
    if (trimmed.startsWith('bunker://')) return 'bunker';
    return 'unknown';
  });

  $effect(() => {
    if (typeof window !== 'undefined') {
      hasExtension = Boolean(window.nostr?.getPublicKey && window.nostr?.signEvent);
    }
  });

  async function completeShipyardLogin(pubkey: string) {
    const signedEvent = await signNostrEventWithNdk({
      pubkey,
      created_at: Math.floor(Date.now() / 1000),
      kind: 27235,
      tags: [
        ['domain', authDomain],
        ['method', 'POST'],
        ['u', authUrl]
      ],
      content: 'Sign in to Shipyard.'
    });
    const response = await shipyardApi.login(signedEvent);
    writeShipyardSession({ token: response.session_token, ownerPubkey: response.user_pubkey });
    credentialInput = '';
    loginModal.close();
    await goto('/dashboard');
  }

  function describeError(err: unknown, fallback: string) {
    if (err instanceof Error) return err.message || fallback;
    const api = err as ApiErrorBody;
    return api?.message ?? fallback;
  }

  async function loginWithExtension() {
    if (!hasExtension) {
      error = 'No browser extension detected.';
      return;
    }
    isLoading = true;
    error = '';
    try {
      const signer = new NDKNip07Signer(1_000, ndk);
      ndk.signer = signer;
      const user = await signer.user();
      await completeShipyardLogin(user.pubkey);
    } catch (err) {
      error = describeError(err, "Couldn't sign you in. Try again?");
    } finally {
      isLoading = false;
    }
  }

  async function loginWithCredential() {
    const trimmed = credentialInput.trim();
    if (!trimmed) return;

    if (credentialType === 'unknown') {
      error = "That doesn't look right — check the key or link and try again.";
      return;
    }

    isLoading = true;
    error = '';
    try {
      if (credentialType === 'nsec') {
        const decoded = nip19.decode(trimmed);
        if (decoded.type !== 'nsec') throw new Error('Invalid nsec.');
        const signer = new NDKPrivateKeySigner(decoded.data as string);
        ndk.signer = signer;
        const user = await signer.user();
        await completeShipyardLogin(user.pubkey);
      } else if (credentialType === 'bunker') {
        await ensureClientNdk();
        const signer = new NDKNip46Signer(ndk, trimmed);
        await signer.blockUntilReady();
        ndk.signer = signer;
        const user = await signer.user();
        await completeShipyardLogin(user.pubkey);
      }
    } catch (err) {
      error = describeError(err, "Couldn't sign you in. Try again?");
    } finally {
      isLoading = false;
    }
  }

  function onOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget && !isLoading) {
      loginModal.close();
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape' && loginModal.open && !isLoading) {
      loginModal.close();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

{#if loginModal.open}
  <div
    class="overlay"
    role="presentation"
    onclick={onOverlayClick}
  >
    <div class="card" role="dialog" aria-modal="true" aria-label="Sign in to Shipyard">
      <button
        type="button"
        class="close"
        aria-label="Close"
        onclick={() => loginModal.close()}
        disabled={isLoading}
      >
        ×
      </button>

      <h2>Sign in to Shipyard</h2>
      <p class="lede">Connect your account. We never see your keys.</p>

      {#if hasExtension}
        <button
          type="button"
          class="primary"
          onclick={loginWithExtension}
          disabled={isLoading}
        >
          {isLoading ? 'Connecting...' : 'Use browser extension'}
        </button>
        <p class="hint">We found a browser extension to sign you in. Fastest option.</p>
      {/if}

      <div class="divider" class:with-or={hasExtension}>
        <span>{hasExtension ? 'or' : 'Paste credentials'}</span>
      </div>

      <label class="field">
        <input
          type="text"
          placeholder="Paste your key or remote signer link"
          bind:value={credentialInput}
          disabled={isLoading}
          autocomplete="off"
          spellcheck="false"
          onkeydown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              loginWithCredential();
            }
          }}
        />
      </label>

      <button
        type="button"
        class={hasExtension ? 'secondary' : 'primary'}
        onclick={loginWithCredential}
        disabled={isLoading || !credentialInput.trim()}
      >
        {isLoading ? 'Signing in...' : 'Sign in'}
      </button>

      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>
  </div>
{/if}
