import type { Page } from '@playwright/test';

export const API_BASE = 'http://localhost:8080';

// localStorage key names used by the app (from src/lib/api/session.ts)
const SESSION_TOKEN_KEY = 'shipyard.session_token';
const OWNER_PUBKEY_KEY = 'shipyard.owner_pubkey';

export const MOCK_TOKEN = 'test-session-token-uuid';
export const MOCK_PUBKEY = 'abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234';

export const mockStatus = { status: 'ok', version: '0.1.0' };

export const mockSession = {
  user_pubkey: MOCK_PUBKEY,
  expires_at: new Date(Date.now() + 86_400_000).toISOString()
};

export const mockAccount = {
  owner_pubkey: MOCK_PUBKEY,
  relationship: 'owner'
};

export const mockLoginResponse = {
  session_token: MOCK_TOKEN,
  user_pubkey: MOCK_PUBKEY
};

export const mockQueue = {
  id: 'queue-1',
  name: 'Weekday Posts',
  description: 'Mon-Fri content',
  cadence_seconds: 86400,
  start_at: new Date(Date.now() + 3600_000).toISOString(),
  archived_at: null
};

export const mockProposal = {
  id: 'proposal-1',
  state: 'PROPOSED',
  owner_pubkey: MOCK_PUBKEY,
  created_by_pubkey: MOCK_PUBKEY,
  unsigned_event_json: {
    content: 'Hello Nostr world',
    kind: 1,
    tags: [],
    created_at: 0,
    pubkey: MOCK_PUBKEY,
    id: null,
    sig: null
  },
  signed_event_json: null,
  publish_time: new Date(Date.now() + 3600_000).toISOString(),
  created_at: new Date().toISOString(),
  published_at: null,
  event_id: null,
  queue_id: null
};

export const mockPublishItem = {
  id: 'item-1',
  state: 'SCHEDULED',
  owner_pubkey: MOCK_PUBKEY,
  created_by_pubkey: MOCK_PUBKEY,
  unsigned_event_json: {
    content: 'Upcoming post',
    kind: 1,
    tags: [],
    created_at: 0,
    pubkey: MOCK_PUBKEY,
    id: null,
    sig: null
  },
  signed_event_json: null,
  publish_time: new Date(Date.now() + 7200_000).toISOString(),
  created_at: new Date().toISOString(),
  published_at: null,
  event_id: null,
  queue_id: null
};

/** Mock the /v1/status endpoint only (unauthenticated state) */
export async function mockStatusOnly(page: Page) {
  await page.route(`${API_BASE}/v1/status`, (route) =>
    route.fulfill({ json: mockStatus })
  );
}

/** Mock all API endpoints for a logged-in owner session */
export async function mockAuthenticatedSession(page: Page) {
  await page.route(`${API_BASE}/v1/status`, (route) =>
    route.fulfill({ json: mockStatus })
  );
  await page.route(`${API_BASE}/v1/auth/login`, (route) =>
    route.fulfill({ json: mockLoginResponse })
  );
  await page.route(`${API_BASE}/v1/auth/session`, (route) =>
    route.fulfill({ json: mockSession })
  );
  await page.route(`${API_BASE}/v1/auth/logout`, (route) =>
    route.fulfill({ status: 200, json: {} })
  );
  await page.route(`${API_BASE}/v1/accounts`, (route) =>
    route.fulfill({ json: { accounts: [mockAccount] } })
  );
  // Delegates: /v1/accounts/:pubkey/delegates — use regex to reliably match
  await page.route(/\/v1\/accounts\/[^/]+\/delegates/, (route) =>
    route.fulfill({ json: [] })
  );
  await page.route(`${API_BASE}/v1/publish-items`, (route) =>
    route.fulfill({ json: [mockPublishItem] })
  );
  await page.route(`${API_BASE}/v1/proposals`, (route) => {
    if (route.request().method() === 'GET') {
      return route.fulfill({ json: [mockProposal] });
    }
    return route.fulfill({ json: mockProposal });
  });
  // Proposal sub-routes: /v1/proposals/:id, /v1/proposals/:id/reject, etc.
  await page.route(/\/v1\/proposals\/[^/]+(\/[^/]+)?$/, (route) => {
    if (route.request().method() === 'DELETE') {
      return route.fulfill({ status: 204 });
    }
    return route.fulfill({ json: mockProposal });
  });
  await page.route(`${API_BASE}/v1/queues`, (route) => {
    if (route.request().method() === 'GET') {
      return route.fulfill({ json: [mockQueue] });
    }
    return route.fulfill({ json: mockQueue });
  });
  // Queue sub-routes: /v1/queues/:id, /v1/queues/:id/archive, etc.
  await page.route(/\/v1\/queues\/[^/]+(\/[^/]+)?$/, (route) =>
    route.fulfill({ json: mockQueue })
  );
  await page.route(`${API_BASE}/v1/relays`, (route) =>
    route.fulfill({ json: { relay_urls: ['wss://relay.example.com'] } })
  );
}

/** Inject a mock NIP-07 window.nostr extension */
export async function injectMockNostr(page: Page, pubkey: string) {
  await page.addInitScript((pk) => {
    Object.defineProperty(window, 'nostr', {
      configurable: true,
      writable: true,
      value: {
        getPublicKey: () => Promise.resolve(pk),
        signEvent: (event: Record<string, unknown>) =>
          Promise.resolve({ ...event, id: 'mock-event-id', sig: 'mock-sig' })
      }
    });
  }, pubkey);
}
