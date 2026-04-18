import type NDK from '@nostr-dev-kit/ndk';
import type { NDKSigner, NDKImetaTag } from '@nostr-dev-kit/ndk';
import NDKBlossom from '@nostr-dev-kit/blossom';
import { resolveBlossomServers } from './server';

// ── Error taxonomy ─────────────────────────────────────────────────────────

export type BlossomUploadErrorKind =
  | 'signer_failure'
  | 'server_selection_failure'
  | 'upload_failure'
  | 'response_parsing_failure';

export class BlossomUploadError extends Error {
  constructor(
    public readonly kind: BlossomUploadErrorKind,
    message: string,
    public readonly cause?: unknown
  ) {
    super(message);
    this.name = 'BlossomUploadError';
  }
}

export type BlossomUploadResult = {
  url: string;
  sha256: string;
  size: number;
  mimeType?: string;
  imeta: NDKImetaTag;
};

// ── Upload ──────────────────────────────────────────────────────────────────

/**
 * Uploads a file to the user's primary Blossom server.
 * Uses NDKBlossom under the hood; discovers the server from kind 10063.
 *
 * @throws {BlossomUploadError} with a discriminated `kind` field
 */
export async function blossomUpload(
  ndk: NDK,
  file: File,
  signer?: NDKSigner
): Promise<BlossomUploadResult> {
  // 1. Resolve signer
  const activeSigner = signer ?? ndk.signer;
  if (!activeSigner) {
    throw new BlossomUploadError(
      'signer_failure',
      'No NDK signer available — connect a signer before uploading'
    );
  }

  // 2. Resolve server list
  let servers: string[];
  try {
    const user = await activeSigner.user();
    servers = await resolveBlossomServers(ndk, user.pubkey);
  } catch (err) {
    throw new BlossomUploadError(
      'server_selection_failure',
      'Failed to resolve Blossom server list',
      err
    );
  }

  // 3. Upload via NDKBlossom
  const blossom = new NDKBlossom(ndk, activeSigner);
  let imeta: NDKImetaTag;
  try {
    imeta = await blossom.upload(file);
  } catch (err) {
    throw new BlossomUploadError('upload_failure', `Blossom upload failed: ${String(err)}`, err);
  }

  // 4. Parse response
  try {
    const url = imeta.url;
    const sha256 = imeta.x;
    const size = imeta.size ? parseInt(imeta.size, 10) : 0;

    if (!url) {
      throw new Error('Missing url in imeta response');
    }

    return {
      url,
      sha256: sha256 ?? '',
      size,
      mimeType: imeta.m,
      imeta
    };
  } catch (err) {
    throw new BlossomUploadError(
      'response_parsing_failure',
      'Failed to parse Blossom upload response',
      err
    );
  }
}

/** @internal exported for tests */
export { resolveBlossomServers };
