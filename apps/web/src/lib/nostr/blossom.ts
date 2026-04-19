import { DEFAULT_BLOSSOM_SERVER } from '$lib/ndk/config';
import type { BrowserNostrSigner, SignedNostrEvent, UnsignedNostrEvent } from './drafts';
import { signNostrEventWithNdk } from './signing';

export const blossomServerListKind = 10063;
export const blossomAuthKind = 24242;

export type BlossomServerListEvent = {
  kind: number;
  tags: string[][];
};

export type BlossomDescriptor = {
  url: string;
  sha256: string;
  size: number;
  type: string;
  uploaded: number;
};

export type BlossomUploadErrorKind = 'signer' | 'server_selection' | 'upload' | 'response';

export class BlossomUploadError extends Error {
  constructor(
    readonly kind: BlossomUploadErrorKind,
    message: string
  ) {
    super(message);
    this.name = 'BlossomUploadError';
  }
}

export type BlossomUploadInput = {
  blob: Blob;
  signer: BrowserNostrSigner | undefined;
  serverListEvents?: BlossomServerListEvent[];
  servers?: string[];
  fallbackServer?: string;
};

export type BlossomUploadResult = {
  server: string;
  descriptor: BlossomDescriptor;
};

export function resolveBlossomServers(
  events: BlossomServerListEvent[] = [],
  fallbackServer = DEFAULT_BLOSSOM_SERVER
): string[] {
  const servers = events
    .filter((event) => event.kind === blossomServerListKind)
    .flatMap((event) => event.tags)
    .filter((tag) => tag[0] === 'server')
    .map((tag) => normalizeServerUrl(tag[1]))
    .filter((server): server is string => Boolean(server));

  const uniqueServers = [...new Set(servers)];
  return uniqueServers.length ? uniqueServers : [fallbackServer];
}

export function parseServerListJson(value: string): BlossomServerListEvent[] {
  if (!value.trim()) {
    return [];
  }

  const parsed = JSON.parse(value) as BlossomServerListEvent | BlossomServerListEvent[];
  return (Array.isArray(parsed) ? parsed : [parsed]).filter(isServerListEvent);
}

export async function uploadBlobToBlossom(
  input: BlossomUploadInput
): Promise<BlossomUploadResult> {
  const signer = input.signer;
  if (!signer?.getPublicKey || !signer.signEvent) {
    throw new BlossomUploadError('signer', 'No browser Nostr signer is available.');
  }

  const selectedServers = selectServers(input);
  const server = selectedServers[0];
  if (!server) {
    throw new BlossomUploadError('server_selection', 'No valid Blossom server is available.');
  }

  const pubkey = await signer.getPublicKey().catch(() => {
    throw new BlossomUploadError('signer', 'Browser signer did not return a pubkey.');
  });
  const sha256 = await sha256Hex(input.blob);
  const authEvent = await signUploadAuthorization({ signer, pubkey, server, sha256 });
  const response = await putBlob(server, input.blob, sha256, authEvent);

  return {
    server,
    descriptor: await parseDescriptorResponse(response)
  };
}

export async function signUploadAuthorization(input: {
  signer: BrowserNostrSigner;
  pubkey: string;
  server: string;
  sha256: string;
  expirationSeconds?: number;
}): Promise<SignedNostrEvent> {
  if (!input.signer.signEvent) {
    throw new BlossomUploadError('signer', 'No browser Nostr signer is available.');
  }

  const serverHost = hostnameForServer(input.server);
  const createdAt = Math.floor(Date.now() / 1000);
  const event: UnsignedNostrEvent = {
    pubkey: input.pubkey,
    created_at: createdAt,
    kind: blossomAuthKind,
    tags: [
      ['t', 'upload'],
      ['expiration', String(createdAt + (input.expirationSeconds ?? 15 * 60))],
      ['x', input.sha256],
      ['server', serverHost]
    ],
    content: 'Upload Blob'
  };

  return signNostrEventWithNdk(event).catch(() => {
    throw new BlossomUploadError('signer', 'Browser signer rejected upload authorization.');
  });
}

export async function sha256Hex(blob: Blob): Promise<string> {
  const bytes = await blob.arrayBuffer();
  const digest = await crypto.subtle.digest('SHA-256', bytes);

  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function selectServers(input: BlossomUploadInput): string[] {
  const directServers = (input.servers ?? [])
    .map(normalizeServerUrl)
    .filter((server): server is string => Boolean(server));

  if (directServers.length) {
    return [...new Set(directServers)];
  }

  return resolveBlossomServers(input.serverListEvents, input.fallbackServer ?? DEFAULT_BLOSSOM_SERVER);
}

async function putBlob(
  server: string,
  blob: Blob,
  sha256: string,
  authEvent: SignedNostrEvent
): Promise<Response> {
  const response = await fetch(`${server}/upload`, {
    method: 'PUT',
    headers: {
      authorization: `Nostr ${base64Url(JSON.stringify(authEvent))}`,
      'content-type': blob.type || 'application/octet-stream',
      'x-sha-256': sha256
    },
    body: blob
  }).catch(() => {
    throw new BlossomUploadError('upload', `Upload request to ${server} failed.`);
  });

  if (!response.ok) {
    throw new BlossomUploadError('upload', `Upload failed with HTTP ${response.status}.`);
  }

  return response;
}

async function parseDescriptorResponse(response: Response): Promise<BlossomDescriptor> {
  const body = await response.json().catch(() => {
    throw new BlossomUploadError('response', 'Blossom server returned non-JSON response.');
  });

  if (!isDescriptor(body)) {
    throw new BlossomUploadError('response', 'Blossom server returned an invalid descriptor.');
  }

  return body;
}

function isDescriptor(value: BlossomDescriptor): value is BlossomDescriptor {
  return (
    typeof value?.url === 'string' &&
    typeof value.sha256 === 'string' &&
    typeof value.size === 'number' &&
    typeof value.type === 'string' &&
    typeof value.uploaded === 'number'
  );
}

function isServerListEvent(value: BlossomServerListEvent): value is BlossomServerListEvent {
  return value?.kind === blossomServerListKind && Array.isArray(value.tags);
}

function normalizeServerUrl(value: string | undefined): string | null {
  if (!value) {
    return null;
  }

  try {
    const url = new URL(value);
    if (url.protocol !== 'https:' && url.protocol !== 'http:') {
      return null;
    }

    url.pathname = url.pathname.replace(/\/+$/, '');
    return url.toString().replace(/\/$/, '');
  } catch {
    return null;
  }
}

function hostnameForServer(server: string): string {
  try {
    return new URL(server).hostname.toLowerCase();
  } catch {
    throw new BlossomUploadError('server_selection', 'Invalid Blossom server URL.');
  }
}

function base64Url(value: string): string {
  return btoa(value).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
