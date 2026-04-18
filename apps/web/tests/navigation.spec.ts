import { test, expect } from '@playwright/test';
import { mockStatusOnly, API_BASE } from './helpers/api-mock';

test.describe('App navigation', () => {
  test.beforeEach(async ({ page }) => {
    await mockStatusOnly(page);
    // Suppress API calls that would fail without a real backend
    await page.route(`${API_BASE}/**`, (route) => route.fulfill({ json: [] }));
    await page.addInitScript(() => localStorage.clear());
  });

  test('navigates from dashboard to scheduled via View all', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'View all' }).first().click();
    await expect(page).toHaveURL(/\/scheduled/);
  });

  test('navigates from dashboard to Proposals review via link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'Review' }).click();
    await expect(page).toHaveURL(/\/proposals/);
  });

  test('navigates to Write page from dashboard header button', async ({ page }) => {
    await page.goto('/');
    // Scope to main content to avoid the nav link
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

  test('navigates to Proposals page', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('heading', { name: 'Proposals', level: 1 })).toBeVisible();
  });
});
