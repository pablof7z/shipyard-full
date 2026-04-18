import type NDK from '@nostr-dev-kit/ndk';
import { NDKBlossomList, type NDKKind } from '@nostr-dev-kit/ndk';
import { defaultBlossomServer } from '../client';

/**
 * Resolves the Blossom server list for a user from kind 10063.
 * Falls back to defaultBlossomServer if no list is found.
 */
export async function resolveBlossomServers(ndk: NDK, pubkey: string): Promise<string[]> {
  try {
    const raw = await ndk.fetchEvent({
      kinds: [10063 as NDKKind],
      authors: [pubkey],
      limit: 1
    });

    if (raw) {
      const list = NDKBlossomList.from(raw);
      const servers = list.servers;
      if (servers.length > 0) {
        return servers;
      }
    }
  } catch {
    // fall through to default
  }
  return [defaultBlossomServer];
}

/**
 * Returns the primary Blossom server (first in user's list, or default).
 */
export async function primaryBlossomServer(ndk: NDK, pubkey: string): Promise<string> {
  const servers = await resolveBlossomServers(ndk, pubkey);
  return servers[0];
}
