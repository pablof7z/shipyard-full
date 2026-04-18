import {
  createDraftId,
  createSignedBlankDraftWrap,
  createSignedDraftWrap,
  decryptDraftWrap,
  type DraftSourceEvent
} from '$lib/nostr/drafts';
import { upsertLocalDraftWrap, type LocalDraftWrapRecord } from '$lib/nostr/local-drafts';
import { parseTagsText } from './composer-actions';

export async function saveDraftWrap(input: {
  ownerPubkey: string;
  draftId: string;
  content: string;
  tagsText: string;
}): Promise<string> {
  const pubkey = await signerPubkey(input.ownerPubkey);
  const activeDraftId = input.draftId.trim() || createDraftId();
  const signed = await createSignedDraftWrap({
    signer: window.nostr,
    pubkey,
    draftId: activeDraftId,
    draft: {
      kind: 1,
      content: input.content,
      tags: parseTagsText(input.tagsText)
    }
  });
  upsertLocalDraftWrap(signed);
  return activeDraftId;
}

export async function loadDraftWrap(record: LocalDraftWrapRecord): Promise<DraftSourceEvent> {
  return decryptDraftWrap(window.nostr, record.event);
}

export async function blankDraftWrap(ownerPubkey: string, record: LocalDraftWrapRecord) {
  const pubkey = await signerPubkey(ownerPubkey);
  const signed = await createSignedBlankDraftWrap({
    signer: window.nostr,
    pubkey,
    draftId: record.draftId,
    draftKind: record.targetKind
  });
  upsertLocalDraftWrap(signed);
}

async function signerPubkey(ownerPubkey: string): Promise<string> {
  const pubkey = await window.nostr?.getPublicKey?.();
  if (!pubkey) throw new Error('No browser Nostr signer is available.');
  if (ownerPubkey && pubkey !== ownerPubkey) {
    throw new Error('Browser signer pubkey does not match the active owner.');
  }
  return pubkey;
}
