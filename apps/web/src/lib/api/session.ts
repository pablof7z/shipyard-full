import { browser } from '$app/environment';

const sessionTokenKey = 'shipyard.session_token';
const ownerPubkeyKey = 'shipyard.owner_pubkey';

export const sessionUpdatedEvent = 'shipyard-session-updated';

export type ShipyardSession = {
  token: string;
  ownerPubkey: string;
};

export function readShipyardSession(): ShipyardSession {
  if (!browser) {
    return { token: '', ownerPubkey: '' };
  }

  return {
    token: localStorage.getItem(sessionTokenKey) ?? '',
    ownerPubkey: localStorage.getItem(ownerPubkeyKey) ?? ''
  };
}

export function writeShipyardSession(session: ShipyardSession): void {
  if (!browser) {
    return;
  }

  if (session.token.trim()) {
    localStorage.setItem(sessionTokenKey, session.token.trim());
  } else {
    localStorage.removeItem(sessionTokenKey);
  }

  if (session.ownerPubkey.trim()) {
    localStorage.setItem(ownerPubkeyKey, session.ownerPubkey.trim());
  } else {
    localStorage.removeItem(ownerPubkeyKey);
  }

  window.dispatchEvent(new CustomEvent(sessionUpdatedEvent));
}

export function clearShipyardSession(): void {
  writeShipyardSession({ token: '', ownerPubkey: '' });
}

export function compactPubkey(pubkey: string, leading = 10, trailing = 6): string {
  if (!pubkey) {
    return 'No account';
  }

  if (pubkey.length <= leading + trailing + 3) {
    return pubkey;
  }

  return `${pubkey.slice(0, leading)}...${pubkey.slice(-trailing)}`;
}
