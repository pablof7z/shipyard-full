import { test, expect } from '@playwright/test';
import {
  mockStatusOnly,
  mockAuthenticatedSession,
  MOCK_TOKEN,
  MOCK_PUBKEY
} from './helpers/api-mock';

test.describe('Dashboard — unauthenticated', () => {
  test.beforeEach(async ({ page }) => {
    await mockStatusOnly(page);
    await page.addInitScript(() => localStorage.clear());
  });

  test('renders page title and heading', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle('Shipyard');
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('shows eyebrow label "Publishing"', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.eyebrow')).toHaveText('Publishing');
  });

  test('shows 4 stat cards', async ({ page }) => {
    await page.goto('/');
    const stats = page.locator('.stats-grid .stat');
    await expect(stats).toHaveCount(4);
  });

  test('stat labels are Scheduled, Pending Review, Published Today, Needs Attention', async ({ page }) => {
    await page.goto('/');
    const stats = page.locator('.stats-grid .stat span');
    await expect(stats.nth(0)).toHaveText('Scheduled');
    await expect(stats.nth(1)).toHaveText('Pending Review');
    await expect(stats.nth(2)).toHaveText('Published Today');
    await expect(stats.nth(3)).toHaveText('Needs Attention');
  });

  test('shows session notice when not logged in', async ({ page }) => {
    await page.goto('/');
    const notice = page.locator('.notice').first();
    await expect(notice.getByRole('link', { name: /Sign in/ })).toHaveAttribute('href', '/settings#login');
  });

  test('shows sidebar sign in link when not logged in', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.account-pill').getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/settings#login');
  });

  test('shows Upcoming section with no items', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Upcoming' })).toBeVisible();
    await expect(page.locator('.panel').first()).toContainText('No upcoming publish items');
  });

  test('shows Pending Review section with no items', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Pending Review' })).toBeVisible();
    await expect(page.getByText('No proposals waiting for owner action')).toBeVisible();
  });

  test('Write link is visible in page header', async ({ page }) => {
    await page.goto('/');
    // Scope to main content to avoid ambiguity with nav link
    await expect(page.getByRole('main').getByRole('link', { name: 'Write' })).toBeVisible();
  });
});

test.describe('Dashboard — authenticated', () => {
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

  test('shows API connected notice when session is set', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.notice.muted')).toContainText('API connected', { timeout: 8000 });
  });

  test('shows scheduled item in Upcoming section', async ({ page }) => {
    await page.goto('/');
    // Wait for loading to complete and item to appear
    const upcomingSection = page.locator('.panel').first().locator('.rows');
    await expect(upcomingSection).toContainText('Upcoming post', { timeout: 8000 });
  });

  test('scheduled stat reflects mock data', async ({ page }) => {
    await page.goto('/');
    const scheduledStat = page.locator('.stats-grid .stat').nth(0).locator('strong');
    await expect(scheduledStat).toHaveText('1', { timeout: 8000 });
  });
});
