import { shipyardApi } from '$lib/api/client';
import type { PublishTrigger } from '$lib/api/types';
import type { UnsignedNostrEvent } from '$lib/nostr/drafts';
import { parseTagsText, publishTimeFor, queueFor } from './composer-actions';

type CompositionInput = {
  token: string;
  ownerPubkey: string;
  trigger: PublishTrigger;
  publishAt: string;
  queueId: string;
  content: string;
  tagsText: string;
};

export async function submitComposition(input: CompositionInput): Promise<'signed' | 'proposal'> {
  const unsigned = draftEvent(input);
  const signed = await signAsOwner(input.ownerPubkey, unsigned);

  if (signed) {
    if (input.trigger === 'SEND_NOW') {
      await shipyardApi.sendNow(input.token, input.ownerPubkey, signed);
    } else {
      await shipyardApi.scheduleSignedEvent(input.token, input.ownerPubkey, {
        signed_event: signed,
        trigger: input.trigger,
        publish_time: publishTimeFor(input.trigger, input.publishAt),
        queue_id: queueFor(input.trigger, input.queueId)
      });
    }
    return 'signed';
  }

  await shipyardApi.createProposal(input.token, {
    owner_pubkey: input.ownerPubkey,
    unsigned_event: { ...unsigned, id: null, sig: null },
    trigger: input.trigger,
    publish_time: publishTimeFor(input.trigger, input.publishAt),
    queue_id: queueFor(input.trigger, input.queueId)
  });
  return 'proposal';
}

export async function scheduleSignedJson(input: CompositionInput & { signedEventText: string }) {
  const signedEvent = JSON.parse(input.signedEventText) as Record<string, unknown>;
  if (input.trigger === 'SEND_NOW') {
    await shipyardApi.sendNow(input.token, input.ownerPubkey, signedEvent);
    return;
  }

  await shipyardApi.scheduleSignedEvent(input.token, input.ownerPubkey, {
    signed_event: signedEvent,
    trigger: input.trigger,
    publish_time: publishTimeFor(input.trigger, input.publishAt),
    queue_id: queueFor(input.trigger, input.queueId)
  });
}

function draftEvent(input: CompositionInput): UnsignedNostrEvent {
  const createdAt =
    input.trigger === 'TIME' && input.publishAt
      ? Math.floor(new Date(input.publishAt).getTime() / 1000)
      : Math.floor(Date.now() / 1000);

  return {
    pubkey: input.ownerPubkey,
    created_at: createdAt,
    kind: 1,
    tags: parseTagsText(input.tagsText),
    content: input.content
  };
}

async function signAsOwner(ownerPubkey: string, event: UnsignedNostrEvent) {
  if (!window.nostr?.getPublicKey || !window.nostr.signEvent) return null;
  const pubkey = await window.nostr.getPublicKey().catch(() => null);
  return pubkey === ownerPubkey ? window.nostr.signEvent(event) : null;
}
