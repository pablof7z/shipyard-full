import { test, expect } from '@playwright/test';
import {
  mockAuthenticatedSession,
  injectMockNostr,
  MOCK_TOKEN,
  MOCK_PUBKEY
} from './helpers/api-mock';

test.describe('Settings page — guarded route', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => localStorage.clear());
  });

  test('redirects unauthenticated visitors to landing', async ({ page }) => {
    await page.goto('/settings');
    await expect(page).toHaveURL(/\/$/);
    await expect(page).toHaveTitle('Shipyard - Schedule your Nostr posts');
  });
});

test.describe('NIP-07 login flow', () => {
  test.beforeEach(async ({ page }) => {
    // Wait for full Svelte hydration before tests interact — dev server needs networkidle
    await page.addInitScript(() => localStorage.clear());
  });

  test('browser extension login from landing enters the app', async ({ page }) => {
    await injectMockNostr(page, MOCK_PUBKEY);
    await mockAuthenticatedSession(page);

    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.getByRole('link', { name: 'Sign in with Nostr' }).first().click();
    await page.getByRole('button', { name: 'Use browser extension' }).click();

    await expect(page).toHaveURL(/\/dashboard/, { timeout: 10_000 });
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('shows credential login when no NIP-07 extension is present', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.getByRole('link', { name: 'Sign in with Nostr' }).first().click();

    await expect(page.getByRole('dialog', { name: 'Sign in to Shipyard' })).toBeVisible();
    await expect(page.getByPlaceholder('Paste your key or remote signer link')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Use browser extension' })).toHaveCount(0);
  });
});

test.describe('Settings — logged-in state', () => {
  test.beforeEach(async ({ page }) => {
    await mockAuthenticatedSession(page);
    await page.addInitScript(
      ({ token, pubkey }) => {
        localStorage.setItem('shipyard.session_token', token);
        localStorage.setItem('shipyard.owner_pubkey', pubkey);
      },
      { token: MOCK_TOKEN, pubkey: MOCK_PUBKEY }
    );
  });

  test('renders settings shell', async ({ page }) => {
    await page.goto('/settings');
    await expect(page).toHaveTitle('Settings - Shipyard');
    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Account', level: 2 })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Team', level: 2 })).toBeVisible({ timeout: 8000 });
    await expect(page.getByRole('heading', { name: 'Agents', level: 2 })).toBeVisible();
  });

  test('shows signed-in account details', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText(/Signed in until/)).toBeVisible({ timeout: 8000 });
    await expect(page.getByRole('button', { name: 'Sign out' })).toBeEnabled();
  });

  test('shows teammate invite controls for owner accounts', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByPlaceholder("Teammate's npub")).toBeVisible({ timeout: 8000 });
    await expect(page.getByRole('button', { name: 'Invite' })).toBeDisabled();
  });

  test('shows agent skill prompt', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('link', { name: 'View SKILL.md' })).toHaveAttribute(
      'href',
      '/SKILL.md'
    );
    await expect(page.getByLabel('Agent prompt')).toHaveValue(/\/SKILL\.md/);
  });

  test('sign out clears session and returns to landing', async ({ page }) => {
    await page.goto('/settings');
    const signOutButton = page.getByRole('button', { name: 'Sign out' });
    await expect(signOutButton).toBeEnabled({ timeout: 8000 });
    await signOutButton.click();
    await expect(page).toHaveURL(/\/$/, { timeout: 5000 });
    await expect(page).toHaveTitle('Shipyard - Schedule your Nostr posts');
  });
});
