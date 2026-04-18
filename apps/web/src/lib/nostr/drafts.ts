export const draftWrapKind = 31234;
export const defaultDraftExpirationDays = 90;

export type NostrTag = string[];

export type UnsignedNostrEvent = {
  pubkey: string;
  created_at: number;
  kind: number;
  tags: NostrTag[];
  content: string;
};

export type SignedNostrEvent = UnsignedNostrEvent & {
  id: string;
  sig: string;
};

export type BrowserNostrSigner = {
  getPublicKey?: () => Promise<string>;
  signEvent?: (event: UnsignedNostrEvent) => Promise<SignedNostrEvent>;
  nip44?: {
    encrypt?: (pubkey: string, plaintext: string) => Promise<string>;
    decrypt?: (pubkey: string, ciphertext: string) => Promise<string>;
  };
};

export type DraftSourceEvent = {
  kind: number;
  content: string;
  tags: NostrTag[];
  created_at?: number;
  pubkey?: string;
};

export type DraftWrapPayloadInput = {
  pubkey: string;
  draftId: string;
  draftKind: number;
  encryptedContent: string;
  anchorTags?: NostrTag[];
  createdAt?: number;
  expirationDays?: number;
};

export function createDraftId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }

  return `draft-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function createDraftEvent(input: DraftSourceEvent, pubkey: string): DraftSourceEvent {
  return {
    ...input,
    pubkey,
    created_at: input.created_at ?? unixNow(),
    tags: input.tags ?? []
  };
}

export function createDraftWrapPayload(input: DraftWrapPayloadInput): UnsignedNostrEvent {
  const createdAt = input.createdAt ?? unixNow();
  const tags: NostrTag[] = [
    ['d', input.draftId],
    ['k', String(input.draftKind)]
  ];

  const expirationDays = input.expirationDays ?? defaultDraftExpirationDays;
  if (expirationDays > 0) {
    tags.push(['expiration', String(createdAt + expirationDays * 24 * 60 * 60)]);
  }

  for (const tag of input.anchorTags ?? []) {
    if (tag[0] === 'e' || tag[0] === 'a') {
      tags.push(tag);
    }
  }

  return {
    pubkey: input.pubkey,
    created_at: createdAt,
    kind: draftWrapKind,
    tags,
    content: input.encryptedContent
  };
}

export function createBlankDraftWrapPayload(input: {
  pubkey: string;
  draftId: string;
  draftKind: number;
  createdAt?: number;
}): UnsignedNostrEvent {
  return {
    pubkey: input.pubkey,
    created_at: input.createdAt ?? unixNow(),
    kind: draftWrapKind,
    tags: [
      ['d', input.draftId],
      ['k', String(input.draftKind)]
    ],
    content: ''
  };
}

export async function createSignedDraftWrap(input: {
  signer: BrowserNostrSigner | undefined;
  pubkey: string;
  draftId: string;
  draft: DraftSourceEvent;
  expirationDays?: number;
}): Promise<SignedNostrEvent> {
  assertDraftSigner(input.signer);
  const draft = createDraftEvent(input.draft, input.pubkey);
  const encryptedContent = await input.signer.nip44.encrypt(
    input.pubkey,
    JSON.stringify(draft)
  );

  return input.signer.signEvent(
    createDraftWrapPayload({
      pubkey: input.pubkey,
      draftId: input.draftId,
      draftKind: draft.kind,
      encryptedContent,
      anchorTags: draft.tags,
      expirationDays: input.expirationDays
    })
  );
}

export async function createSignedBlankDraftWrap(input: {
  signer: BrowserNostrSigner | undefined;
  pubkey: string;
  draftId: string;
  draftKind: number;
}): Promise<SignedNostrEvent> {
  assertSigningSigner(input.signer);

  return input.signer.signEvent(
    createBlankDraftWrapPayload({
      pubkey: input.pubkey,
      draftId: input.draftId,
      draftKind: input.draftKind
    })
  );
}

export async function decryptDraftWrap(
  signer: BrowserNostrSigner | undefined,
  event: SignedNostrEvent
): Promise<DraftSourceEvent> {
  if (!signer?.nip44?.decrypt) {
    throw new Error('Browser signer does not support NIP-44 decryption.');
  }

  if (!event.content) {
    throw new Error('Draft wrap has blank content and cannot be loaded.');
  }

  const plaintext = await signer.nip44.decrypt(event.pubkey, event.content);
  const draft = JSON.parse(plaintext) as DraftSourceEvent;

  if (typeof draft.kind !== 'number' || typeof draft.content !== 'string') {
    throw new Error('Draft wrap content is not a Nostr draft event.');
  }

  return {
    ...draft,
    tags: Array.isArray(draft.tags) ? draft.tags : []
  };
}

export function draftIdFromEvent(event: SignedNostrEvent): string {
  return event.tags.find((tag) => tag[0] === 'd')?.[1] ?? event.id;
}

export function draftKindFromEvent(event: SignedNostrEvent): number {
  const kind = Number(event.tags.find((tag) => tag[0] === 'k')?.[1]);
  return Number.isFinite(kind) ? kind : 1;
}

export function isDeletedDraftWrap(event: SignedNostrEvent): boolean {
  return event.kind === draftWrapKind && event.content === '';
}

function unixNow(): number {
  return Math.floor(Date.now() / 1000);
}

function assertDraftSigner(
  signer: BrowserNostrSigner | undefined
): asserts signer is Required<Pick<BrowserNostrSigner, 'signEvent'>> &
  BrowserNostrSigner & {
    nip44: { encrypt: (pubkey: string, plaintext: string) => Promise<string> };
  } {
  assertSigningSigner(signer);

  if (!signer.nip44?.encrypt) {
    throw new Error('Browser signer does not support NIP-44 encryption.');
  }
}

function assertSigningSigner(
  signer: BrowserNostrSigner | undefined
): asserts signer is Required<Pick<BrowserNostrSigner, 'signEvent'>> & BrowserNostrSigner {
  if (!signer?.signEvent) {
    throw new Error('No browser Nostr signer is available.');
  }
}
