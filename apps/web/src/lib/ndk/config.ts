export const APP_NAME = 'Shipyard';

export const DEFAULT_BLOSSOM_SERVER = 'https://blossom.primal.net';

const FALLBACK_RELAYS = [
  'wss://relay.damus.io',
  'wss://purplepag.es',
  'wss://relay.primal.net'
];

export const DEFAULT_RELAYS = parseRelayList(
  import.meta.env.PUBLIC_NOSTR_RELAYS as string | undefined,
  FALLBACK_RELAYS
);

export const NDK_CONTEXT_KEY = 'ndk';

function parseRelayList(value: string | undefined, fallback: string[]): string[] {
  if (!value) return fallback;

  const parsed = value
    .split(',')
    .map((relay) => relay.trim())
    .filter(Boolean);

  return parsed.length > 0 ? parsed : fallback;
}
