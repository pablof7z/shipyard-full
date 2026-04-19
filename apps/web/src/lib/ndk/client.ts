import { browser } from '$app/environment';
import { createNDK } from '@nostr-dev-kit/svelte';
import { LocalStorage } from '@nostr-dev-kit/sessions';
import { APP_NAME, DEFAULT_RELAYS } from '$lib/ndk/config';

export const ndk = createNDK({
  explicitRelayUrls: DEFAULT_RELAYS,
  clientName: APP_NAME,
  enableOutboxModel: false,
  session: {
    storage: new LocalStorage('shipyard:sessions'),
    autoSave: true,
    fetches: {
      follows: false,
      mutes: false,
      relayList: true,
      wallet: false
    }
  }
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
