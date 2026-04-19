import { browser } from '$app/environment';
import NDK from '@nostr-dev-kit/ndk';
import { APP_NAME, DEFAULT_RELAYS } from '$lib/ndk/config';

export const ndk = new NDK({
  explicitRelayUrls: DEFAULT_RELAYS,
  clientName: APP_NAME,
  enableOutboxModel: false
});

let connectPromise: Promise<void> | null = null;

export function ensureClientNdk(): Promise<void> {
  if (!browser) return Promise.resolve();
  if (!connectPromise) {
    connectPromise = ndk
      .connect()
      .then(() => undefined)
      .catch((error) => {
        connectPromise = null;
        throw error;
      });
  }

  return connectPromise;
}
