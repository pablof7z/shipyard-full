import { shipyardApi } from '$lib/api/client';
import type { PublishTrigger } from '$lib/api/types';
import { DEFAULT_RELAYS } from '$lib/ndk/config';
import type { UnsignedNostrEvent } from '$lib/nostr/drafts';
import { signNostrEventWithNdk } from '$lib/nostr/signing';
import {
  parseTagsText,
  publishTimeFor,
  publishTimeFromSignedEvent,
  queueFor
} from './composer-actions';

type CompositionInput = {
  token: string;
  ownerPubkey: string;
  trigger: PublishTrigger;
  publishAt: string;
  queueId: string;
  relayUrls: string[];
  content: string;
  tagsText: string;
};

export async function submitComposition(
  input: CompositionInput
): Promise<'signed' | 'proposal' | 'published'> {
  const unsigned = draftEvent(input);
  const signed = await signAsOwner(input.ownerPubkey, unsigned);

  if (input.trigger === 'SEND_NOW') {
    if (!signed) {
      throw new Error('The active browser signer must match the active owner to send now.');
    }
    await publishSignedEventToRelays(signed, input.relayUrls);
    return 'published';
  }

  if (signed) {
    await shipyardApi.scheduleSignedEvent(input.token, input.ownerPubkey, {
      signed_event: signed,
      trigger: input.trigger,
      publish_time: publishTimeFromSignedEvent(input.trigger, signed),
      queue_id: queueFor(input.trigger, input.queueId)
    });
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

export async function scheduleSignedJson(input: CompositionInput & { signedEvent: Record<string, unknown> }) {
  const { signedEvent } = input;
  if (input.trigger === 'SEND_NOW') {
    if (signedEvent.pubkey !== input.ownerPubkey) {
      throw new Error('Signed event pubkey must match the active owner to send now.');
    }
    await publishSignedEventToRelays(signedEvent, input.relayUrls);
    return;
  }

  await shipyardApi.scheduleSignedEvent(input.token, input.ownerPubkey, {
    signed_event: signedEvent,
    trigger: input.trigger,
    publish_time: publishTimeFromSignedEvent(input.trigger, signedEvent),
    queue_id: queueFor(input.trigger, input.queueId)
  });
}

function draftEvent(input: CompositionInput): UnsignedNostrEvent {
  const createdAt =
    input.trigger === 'SEND_NOW'
      ? Math.floor(Date.now() / 1000)
      : input.trigger === 'TIME' && input.publishAt
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
  return pubkey === ownerPubkey ? signNostrEventWithNdk(event) : null;
}

async function publishSignedEventToRelays(
  signedEvent: Record<string, unknown>,
  relayUrls: string[]
) {
  const eventId = signedEvent.id;
  if (typeof eventId !== 'string' || !eventId) {
    throw new Error('Signed event JSON must include id.');
  }

  const targets = relayUrls.length ? relayUrls : DEFAULT_RELAYS;
  const results = await Promise.allSettled(
    targets.map((relayUrl) => publishToRelay(relayUrl, signedEvent, eventId))
  );
  const accepted = results.filter((result) => result.status === 'fulfilled');
  if (accepted.length) return;

  const errors = results
    .map((result, index) =>
      result.status === 'rejected' ? `${targets[index]}: ${String(result.reason)}` : null
    )
    .filter(Boolean)
    .join('; ');
  throw new Error(errors || 'No relays accepted the event.');
}

function publishToRelay(
  relayUrl: string,
  signedEvent: Record<string, unknown>,
  eventId: string
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!relayUrl.startsWith('wss://') && !relayUrl.startsWith('ws://')) {
      reject(new Error('invalid relay URL'));
      return;
    }

    const socket = new WebSocket(relayUrl);
    const timeout = window.setTimeout(() => {
      socket.close();
      reject(new Error('relay OK timed out'));
    }, 10_000);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(['EVENT', signedEvent]));
    });

    socket.addEventListener('error', () => {
      window.clearTimeout(timeout);
      reject(new Error('relay connection failed'));
    });

    socket.addEventListener('message', (message) => {
      let value: unknown;
      try {
        value = parseRelayMessage(message.data);
      } catch (err) {
        window.clearTimeout(timeout);
        socket.close();
        reject(err);
        return;
      }
      if (!Array.isArray(value) || value[0] !== 'OK' || value[1] !== eventId) return;

      window.clearTimeout(timeout);
      socket.close();
      if (value[2] === true) {
        resolve();
      } else {
        reject(new Error(typeof value[3] === 'string' ? value[3] : 'relay rejected event'));
      }
    });
  });
}

function parseRelayMessage(data: unknown): unknown {
  if (typeof data === 'string') return JSON.parse(data);
  throw new Error('relay sent unsupported message data');
}
