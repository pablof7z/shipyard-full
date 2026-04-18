import type NDK from '@nostr-dev-kit/ndk';
import { NDKDraft, NDKEvent, NDKRelaySet } from '@nostr-dev-kit/ndk';
import type { NDKSigner } from '@nostr-dev-kit/ndk';
import { privateRelaySet } from './relays';

export type DraftPayload = {
  /** Draft identifier (d-tag). Auto-generated if omitted. */
  id?: string;
  /** Target event kind (k-tag). */
  targetKind: number;
  /** Draft content as a plain Nostr event object. */
  event: object;
};

/**
 * Creates and publishes a NIP-37 draft wrap (kind 31234).
 * Content is the JSON-stringified draft event, NIP-44 encrypted to self.
 */
export async function saveDraft(
  ndk: NDK,
  payload: DraftPayload,
  signer?: NDKSigner
): Promise<NDKDraft> {
  const activeSigner = signer ?? ndk.signer;
  if (!activeSigner) {
    throw new Error('No NDK signer available — connect a signer before saving drafts');
  }

  const user = await activeSigner.user();

  // Wrap the draft content in a temporary NDKEvent so NDKDraft can encrypt it
  const inner = new NDKEvent(ndk, {
    kind: payload.targetKind,
    content: JSON.stringify(payload.event),
    tags: [],
    pubkey: user.pubkey,
    created_at: Math.floor(Date.now() / 1000)
  });

  const draft = new NDKDraft(ndk);
  if (payload.id) {
    draft.identifier = payload.id;
  }
  draft.event = inner;

  const relaySet = await privateRelaySet(ndk, user.pubkey);
  await draft.save({ signer: activeSigner, publish: true, relaySet });

  return draft;
}

/**
 * Deletes a draft by publishing a NIP-37 blank-content deletion form.
 */
export async function deleteDraft(
  ndk: NDK,
  draftId: string,
  signer?: NDKSigner
): Promise<void> {
  const activeSigner = signer ?? ndk.signer;
  if (!activeSigner) {
    throw new Error('No NDK signer available');
  }

  const user = await activeSigner.user();

  // NIP-37: empty content = deletion signal
  const deletion = new NDKEvent(ndk, {
    kind: 31234,
    content: '',
    tags: [
      ['d', draftId],
      ['k', '1'] // placeholder kind tag required by spec
    ],
    pubkey: user.pubkey,
    created_at: Math.floor(Date.now() / 1000)
  });

  await deletion.sign(activeSigner);
  const relaySet = await privateRelaySet(ndk, user.pubkey);
  await deletion.publish(relaySet);
}
