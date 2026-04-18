import type { BrowserNostrSigner } from '$lib/nostr/drafts';

declare global {
  interface Window {
    nostr?: BrowserNostrSigner;
  }

  namespace App {
    interface Locals {
      requestId?: string;
    }
  }
}

export {};
