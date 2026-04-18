import NDK from '@nostr-dev-kit/ndk';

export const defaultRelays = ['wss://relay.damus.io', 'wss://relay.primal.net'];

export const defaultBlossomServer = 'https://blossom.primal.net';

export type NdkSessionState = {
  userPubkey: string | null;
  activeOwnerPubkey: string | null;
  relays: string[];
};

export function createInitialSession(): NdkSessionState {
  return {
    userPubkey: null,
    activeOwnerPubkey: null,
    relays: defaultRelays
  };
}

// Lazy singleton — created once on first access, only in the browser.
let _ndk: NDK | null = null;

/**
 * Returns the shared NDK instance, creating it on first call.
 * Only call from browser context (inside onMount or event handlers).
 */
export function getNdk(): NDK {
  if (!_ndk) {
    _ndk = new NDK({
      explicitRelayUrls: defaultRelays,
      autoConnectUserRelays: false,
      enableOutboxModel: false
    });
  }
  return _ndk;
}

/**
 * Connect the NDK instance to its configured relays.
 * Safe to call multiple times — no-ops after first connection.
 */
export async function connectNdk(): Promise<void> {
  const ndk = getNdk();
  await ndk.connect();
}

export type { default as NDK } from '@nostr-dev-kit/ndk';
