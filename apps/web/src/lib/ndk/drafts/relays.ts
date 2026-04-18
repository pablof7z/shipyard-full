import type NDK from '@nostr-dev-kit/ndk';
import { type NDKKind, NDKRelaySet } from '@nostr-dev-kit/ndk';
import { defaultRelays } from '../client';

/**
 * Resolves private content relay URLs from kind 10013.
 * Falls back to defaultRelays if none are configured.
 */
export async function resolvePrivateRelays(ndk: NDK, pubkey: string): Promise<string[]> {
  try {
    const raw = await ndk.fetchEvent({
      kinds: [10013 as NDKKind],
      authors: [pubkey],
      limit: 1
    });

    if (raw) {
      const relayUrls = raw.tags
        .filter((t) => t[0] === 'relay' && t[1])
        .map((t) => t[1]);

      if (relayUrls.length > 0) {
        return relayUrls;
      }
    }
  } catch {
    // fall through to default
  }

  return [...defaultRelays];
}

/**
 * Builds an NDKRelaySet from the user's private content relays (kind 10013).
 */
export async function privateRelaySet(ndk: NDK, pubkey: string): Promise<NDKRelaySet> {
  const urls = await resolvePrivateRelays(ndk, pubkey);
  return NDKRelaySet.fromRelayUrls(urls, ndk);
}
