import { test, expect } from '@playwright/test';
import {
  mockAuthenticatedSession,
  injectMockNostr,
  MOCK_TOKEN,
  MOCK_PUBKEY
} from './helpers/api-mock';

test.describe('Settings page', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => localStorage.clear());
  });

  test('renders page title', async ({ page }) => {
    await page.goto('/settings');
    await expect(page).toHaveTitle('Settings - Shipyard');
  });

  test('shows Settings heading', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible();
  });

  test('shows Session and Login sections', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('heading', { name: 'Session', level: 2 })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Login', level: 2 })).toBeVisible();
  });

  test('shows Browser Signer button', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('button', { name: 'Browser Signer' })).toBeVisible();
  });

  test('shows session token and owner pubkey fields', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByPlaceholder(/UUID from/)).toBeVisible();
    await expect(page.getByPlaceholder(/64 hex or npub/)).toBeVisible();
  });

  test('shows Relays and Delegates sections', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('heading', { name: 'Relays', level: 2 })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Delegates', level: 2 })).toBeVisible();
  });
});

test.describe('NIP-07 login flow', () => {
  test.beforeEach(async ({ page }) => {
    // Wait for full Svelte hydration before tests interact — dev server needs networkidle
    await page.addInitScript(() => localStorage.clear());
  });

  test('Browser Signer button triggers login and shows success', async ({ page }) => {
    await injectMockNostr(page, MOCK_PUBKEY);
    await mockAuthenticatedSession(page);

    await page.goto('/settings');
    // Wait for Svelte hydration to complete so onclick handlers are attached
    await page.waitForLoadState('networkidle');

    await page.getByRole('button', { name: 'Browser Signer' }).click();

    await expect(page.locator('.notice.success')).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('.notice.success')).toContainText('Browser signer login accepted');
  });

  test('shows error when no NIP-07 extension is present', async ({ page }) => {
    // No window.nostr injected — it is undefined by default in Chromium headless

    await page.goto('/settings');
    // Wait for Svelte hydration to complete so onclick handlers are attached
    await page.waitForLoadState('networkidle');

    await page.getByRole('button', { name: 'Browser Signer' }).click();

    await expect(page.locator('.notice.error')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.notice.error')).toContainText('No NIP-07 signer');
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

  test('shows session info when token is saved', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.locator('.meta-line').first()).toContainText('Signed in as', { timeout: 8000 });
  });

  test('shows relay URL field populated from API', async ({ page }) => {
    await page.goto('/settings');
    const relayField = page.getByPlaceholder('wss://relay.example.com');
    await expect(relayField).toHaveValue('wss://relay.example.com', { timeout: 8000 });
  });

  test('shows Log out button enabled when session is active', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('button', { name: 'Log out' })).toBeEnabled({ timeout: 8000 });
  });

  test('logout clears session and shows cleared message', async ({ page }) => {
    await page.goto('/settings');
    const logoutBtn = page.getByRole('button', { name: 'Log out' });
    await expect(logoutBtn).toBeEnabled({ timeout: 8000 });
    await logoutBtn.click();
    await expect(page.locator('.notice.success')).toContainText('Session cleared', { timeout: 5000 });
  });
});
