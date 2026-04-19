<script lang="ts">
  import { NDKNip07Signer, NDKPrivateKeySigner, NDKNip46Signer, nip19 } from '@nostr-dev-kit/ndk';
  import { env } from '$env/dynamic/public';
  import { goto } from '$app/navigation';
  import { connectNdk, getNdk } from '$lib/ndk/client';
  import { shipyardApi, shipyardApiBase } from '$lib/api/client';
  import { writeShipyardSession } from '$lib/api/session';
  import { signNostrEventWithNdk } from '$lib/nostr/signing';
  import type { ApiErrorBody } from '$lib/api/types';
  import { loginModal } from './loginState.svelte';

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
      error = 'No browser signer detected.';
      return;
    }
    isLoading = true;
    error = '';
    try {
      const ndk = getNdk();
      await connectNdk();
      const signer = new NDKNip07Signer(1_000, ndk);
      ndk.signer = signer;
      const user = await signer.user();
      await completeShipyardLogin(user.pubkey);
    } catch (err) {
      error = describeError(err, 'Browser signer login failed.');
    } finally {
      isLoading = false;
    }
  }

  async function loginWithCredential() {
    const trimmed = credentialInput.trim();
    if (!trimmed) return;

    if (credentialType === 'unknown') {
      error = 'Unrecognized format. Paste an nsec1... or bunker://... URI.';
      return;
    }

    isLoading = true;
    error = '';
    try {
      const ndk = getNdk();
      await connectNdk();

      if (credentialType === 'nsec') {
        const decoded = nip19.decode(trimmed);
        if (decoded.type !== 'nsec') throw new Error('Invalid nsec.');
        const signer = new NDKPrivateKeySigner(decoded.data as string);
        ndk.signer = signer;
        const user = await signer.user();
        await completeShipyardLogin(user.pubkey);
      } else if (credentialType === 'bunker') {
        const signer = new NDKNip46Signer(ndk, trimmed);
        await signer.blockUntilReady();
        ndk.signer = signer;
        const user = await signer.user();
        await completeShipyardLogin(user.pubkey);
      }
    } catch (err) {
      error = describeError(err, 'Login failed.');
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
      <p class="lede">Connect your Nostr identity. We never see your keys.</p>

      {#if hasExtension}
        <button
          type="button"
          class="primary"
          onclick={loginWithExtension}
          disabled={isLoading}
        >
          {isLoading ? 'Connecting...' : 'Use browser extension'}
        </button>
        <p class="hint">We detected a Nostr signer extension. Fastest option.</p>
      {/if}

      <div class="divider" class:with-or={hasExtension}>
        <span>{hasExtension ? 'or' : 'Paste credentials'}</span>
      </div>

      <label class="field">
        <input
          type="text"
          placeholder="nsec1... or bunker://..."
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

      <p class="hint">
        Supports <code>nsec1...</code> (local private key) and <code>bunker://</code> (NIP-46 remote signer).
      </p>

      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: overlay-fade 0.15s ease;
  }

  :global([data-theme='light']) .overlay {
    background: rgba(0, 0, 0, 0.35);
  }

  @keyframes overlay-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .card {
    position: relative;
    width: 100%;
    max-width: 420px;
    background: var(--bg-primary, #0c0c0c);
    color: var(--text-primary, #ededec);
    border: 1px solid var(--border-default, #262626);
    border-radius: 14px;
    padding: 28px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.45);
    animation: card-rise 0.18s ease;
  }

  :global([data-theme='light']) .card {
    background: #ffffff;
    color: #1a1a19;
    border-color: #e0e0df;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.12);
  }

  @keyframes card-rise {
    from { opacity: 0; transform: translateY(10px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .close {
    position: absolute;
    top: 10px;
    right: 12px;
    background: none;
    border: none;
    color: var(--text-tertiary, #636362);
    font-size: 22px;
    line-height: 1;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
  }

  .close:hover {
    background: var(--bg-tertiary, #161616);
    color: var(--text-primary, #ededec);
  }

  h2 {
    margin: 0 0 6px;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .lede {
    margin: 0 0 20px;
    font-size: 13px;
    color: var(--text-secondary, #a1a1a0);
    line-height: 1.5;
  }

  .primary,
  .secondary {
    width: 100%;
    border-radius: 8px;
    padding: 12px 16px;
    font-family: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    border: 1px solid transparent;
  }

  .primary {
    background: var(--accent, #f06449);
    color: #ffffff;
  }

  .primary:hover:not(:disabled) {
    background: var(--accent-hover, #f4806a);
  }

  .secondary {
    background: transparent;
    color: var(--text-primary, #ededec);
    border-color: var(--border-default, #262626);
  }

  .secondary:hover:not(:disabled) {
    background: var(--bg-tertiary, #161616);
  }

  :global([data-theme='light']) .secondary {
    border-color: #e0e0df;
  }

  :global([data-theme='light']) .secondary:hover:not(:disabled) {
    background: #f5f5f4;
  }

  .primary:disabled,
  .secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted, #4a4a49);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-subtle, #1e1e1e);
  }

  :global([data-theme='light']) .divider::before,
  :global([data-theme='light']) .divider::after {
    background: #e8e8e7;
  }

  .field {
    display: block;
    margin-bottom: 10px;
  }

  .field input {
    width: 100%;
    border-radius: 8px;
    padding: 12px 14px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 13px;
    background: var(--bg-tertiary, #161616);
    color: var(--text-primary, #ededec);
    border: 1px solid var(--border-default, #262626);
    outline: none;
    transition: border-color 0.15s;
  }

  :global([data-theme='light']) .field input {
    background: #fafafa;
    border-color: #e0e0df;
    color: #1a1a19;
  }

  .field input:focus {
    border-color: var(--accent, #f06449);
  }

  .hint {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--text-tertiary, #636362);
    line-height: 1.5;
  }

  .hint code {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 11.5px;
    padding: 1px 4px;
    background: var(--bg-tertiary, #161616);
    border-radius: 4px;
  }

  :global([data-theme='light']) .hint code {
    background: #f5f5f4;
  }

  .error {
    margin-top: 14px;
    padding: 10px 12px;
    background: rgba(240, 100, 73, 0.1);
    border: 1px solid rgba(240, 100, 73, 0.3);
    border-radius: 8px;
    font-size: 13px;
    color: var(--accent, #f06449);
  }
</style>
