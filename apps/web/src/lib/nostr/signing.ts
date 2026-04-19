import { NDKEvent, NDKNip07Signer, type NDKSigner } from '@nostr-dev-kit/ndk';
import { ndk } from '$lib/ndk/client';
import type { SignedNostrEvent, UnsignedNostrEvent } from './drafts';

function ndkSigner(): NDKSigner {
  return ndk.signer ?? new NDKNip07Signer(1_000, ndk);
}

export async function signNostrEventWithNdk(
  event: UnsignedNostrEvent
): Promise<SignedNostrEvent> {
  const ndkEvent = new NDKEvent(ndk, event);
  await ndkEvent.sign(ndkSigner());

  const signed = ndkEvent.rawEvent();
  if (!signed.id || !signed.sig) {
    throw new Error('NDK did not return a signed event.');
  }
  if (signed.pubkey !== event.pubkey) {
    throw new Error('NDK signer pubkey does not match the event pubkey.');
  }

  return {
    pubkey: signed.pubkey,
    created_at: signed.created_at,
    kind: signed.kind,
    tags: signed.tags,
    content: signed.content,
    id: signed.id,
    sig: signed.sig
  };
}
