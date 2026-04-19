import { test, expect } from '@playwright/test';
import { mockAuthenticatedSession, MOCK_TOKEN, MOCK_PUBKEY } from './helpers/api-mock';

test.describe('App navigation', () => {
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

  test('navigates from dashboard to scheduled via View all', async ({ page }) => {
    await page.goto('/dashboard');
    await page.getByRole('link', { name: 'View all' }).first().click();
    await expect(page).toHaveURL(/\/scheduled/);
  });

  test('navigates from dashboard to proposals review via link', async ({ page }) => {
    await page.goto('/dashboard');
    await page.getByLabel('Pending review').getByRole('link', { name: 'Review' }).click();
    await expect(page).toHaveURL(/\/proposals/);
  });

  test('navigates to Write page from dashboard header button', async ({ page }) => {
    await page.goto('/dashboard');
    await page.getByRole('main').getByRole('link', { name: 'Write' }).click();
    await expect(page).toHaveURL(/\/write/);
  });

  test('navigates to Settings page', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible();
  });

  test('navigates to Queues page', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.getByRole('heading', { name: 'Queues', level: 1 })).toBeVisible();
  });

  test('navigates to Review page', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('heading', { name: 'Review', level: 1 })).toBeVisible();
  });
});
