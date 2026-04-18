import type NDK from '@nostr-dev-kit/ndk';
import { NDKDraft, NDKSubscriptionCacheUsage } from '@nostr-dev-kit/ndk';
import type { NDKSigner, NDKEvent } from '@nostr-dev-kit/ndk';
import { privateRelaySet } from './relays';

export type DraftEntry = {
  id: string;
  draft: NDKDraft;
  /** Decoded inner event (null if deleted / empty content). */
  inner: NDKEvent | null;
  updatedAt: number;
};

/**
 * Fetches all draft events (kind 31234) for the given pubkey.
 * Decrypts each draft's inner event using the provided signer.
 */
export async function loadDrafts(
  ndk: NDK,
  pubkey: string,
  signer?: NDKSigner
): Promise<DraftEntry[]> {
  const activeSigner = signer ?? ndk.signer;
  const relaySet = await privateRelaySet(ndk, pubkey);

  const rawEvents = await ndk.fetchEvents(
    {
      kinds: [31234],
      authors: [pubkey]
    },
    {
      cacheUsage: NDKSubscriptionCacheUsage.ONLY_RELAY,
      relaySet
    }
  );

  const entries: DraftEntry[] = [];

  for (const raw of rawEvents) {
    const draft = NDKDraft.from(raw);
    const dTag = raw.tagValue('d') ?? raw.id ?? '';

    let inner: NDKEvent | null = null;
    try {
      inner = (await draft.getEvent(activeSigner)) ?? null;
    } catch {
      // encrypted draft we can't decrypt yet — still include with null inner
    }

    entries.push({
      id: dTag,
      draft,
      inner,
      updatedAt: raw.created_at ?? 0
    });
  }

  // Sort newest first
  entries.sort((a, b) => b.updatedAt - a.updatedAt);
  return entries;
}

/**
 * Loads a single draft by its d-tag identifier.
 */
export async function loadDraft(
  ndk: NDK,
  pubkey: string,
  draftId: string,
  signer?: NDKSigner
): Promise<DraftEntry | null> {
  const activeSigner = signer ?? ndk.signer;
  const relaySet = await privateRelaySet(ndk, pubkey);

  const raw = await ndk.fetchEvent(
    {
      kinds: [31234],
      authors: [pubkey],
      '#d': [draftId]
    },
    undefined,
    relaySet
  );

  if (!raw) return null;

  const draft = NDKDraft.from(raw);
  let inner: NDKEvent | null = null;
  try {
    inner = (await draft.getEvent(activeSigner)) ?? null;
  } catch {
    // can't decrypt
  }

  return {
    id: draftId,
    draft,
    inner,
    updatedAt: raw.created_at ?? 0
  };
}
