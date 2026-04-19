import { test, expect } from '@playwright/test';
import {
  mockAuthenticatedSession,
  mockStatusOnly,
  MOCK_TOKEN,
  MOCK_PUBKEY
} from './helpers/api-mock';

test.describe('Landing page — unauthenticated', () => {
  test.beforeEach(async ({ page }) => {
    await mockStatusOnly(page);
    await page.addInitScript(() => localStorage.clear());
  });

  test('renders marketing title and hero', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle('Shipyard - Schedule your Nostr posts');
    await expect(page.getByText('Shipyard').first()).toBeVisible();
    await expect(page.getByRole('heading', { name: 'A quiet space for loud ideas.' })).toBeVisible();
  });

  test('opens login modal from primary CTA', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'Sign in with Nostr' }).first().click();
    await expect(page.getByRole('dialog', { name: 'Sign in to Shipyard' })).toBeVisible();
  });

  test('shows CLI and agent skill links', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Schedule your Nostr posts.' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Agent skill' })).toHaveAttribute('href', '/SKILL.md');
  });
});

test.describe('Dashboard — authenticated', () => {
  test.beforeEach(async ({ page }) => {
    await mockAuthenticatedSession(page);
    await page.addInitScript(
      ({ token, pubkey }) => {
        localStorage.setItem('shipyard.session_token', token);
        localStorage.setItem('shipyard.owner_pubkey', pubkey);
        localStorage.setItem('shipyard.welcome_seen', '1');
      },
      { token: MOCK_TOKEN, pubkey: MOCK_PUBKEY }
    );
  });

  test('renders page title and heading', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page).toHaveTitle('Shipyard');
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('shows publishing stat cards', async ({ page }) => {
    await page.goto('/dashboard');
    const stats = page.locator('.stats-grid .stat');
    await expect(stats).toHaveCount(4);
    await expect(stats.nth(0)).toContainText('Scheduled');
    await expect(stats.nth(1)).toContainText('Pending review');
  });

  test('shows scheduled item in Upcoming section', async ({ page }) => {
    await page.goto('/dashboard');
    const upcomingSection = page.locator('.panel').first().locator('.rows');
    await expect(upcomingSection).toContainText('Upcoming post', { timeout: 8000 });
  });

  test('scheduled stat reflects mock data', async ({ page }) => {
    await page.goto('/dashboard');
    const scheduledStat = page.locator('.stats-grid .stat').nth(0).locator('strong');
    await expect(scheduledStat).toHaveText('1', { timeout: 8000 });
  });

  test('Write link is visible in page header', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page.getByRole('main').getByRole('link', { name: 'Write' })).toBeVisible();
  });
});
