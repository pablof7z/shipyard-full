export const defaultRelays = ['wss://relay.damus.io', 'wss://relay.primal.net'];

export const defaultBlossomServer = 'https://blossom.primal.net';

export type NdkSessionState = {
  userPubkey: string | null;
  activeOwnerPubkey: string | null;
  relays: string[];
};

export function createInitialSession(): NdkSessionState {
  return {
    userPubkey: null,
    activeOwnerPubkey: null,
    relays: defaultRelays
  };
}
