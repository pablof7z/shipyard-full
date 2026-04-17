declare global {
  interface Window {
    nostr?: {
      getPublicKey?: () => Promise<string>;
      signEvent?: (event: {
        pubkey: string;
        created_at: number;
        kind: number;
        tags: string[][];
        content: string;
      }) => Promise<{
        id: string;
        pubkey: string;
        created_at: number;
        kind: number;
        tags: string[][];
        content: string;
        sig: string;
      }>;
    };
  }

  namespace App {
    interface Locals {
      requestId?: string;
    }
  }
}

export {};
