import { test, expect } from '@playwright/test';
import { mockAuthenticatedSession, mockProposal, MOCK_TOKEN, MOCK_PUBKEY } from './helpers/api-mock';

test.describe('Proposals page — guarded route', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => localStorage.clear());
  });

  test('redirects unauthenticated visitors to landing', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page).toHaveURL(/\/$/);
    await expect(page).toHaveTitle('Shipyard - Schedule your Nostr posts');
  });
});

test.describe('Proposals page — authenticated', () => {
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

  test('renders page title and form shell', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page).toHaveTitle('Review - Shipyard');
    await expect(page.getByRole('heading', { name: 'Review', level: 1 })).toBeVisible();
    await expect(page.locator('.eyebrow')).toHaveText('Team');
    await expect(page.getByRole('button', { name: 'Refresh' })).toBeEnabled();
  });

  async function waitForProposalsLoaded(page: import('@playwright/test').Page) {
    await expect(page.locator('.review-list .review-card').first()).toContainText(
      'Hello Nostr world',
      { timeout: 8000 }
    );
  }

  test('lists pending proposals from API', async ({ page }) => {
    await page.goto('/proposals');
    await waitForProposalsLoaded(page);
    await expect(page.locator('.review-card').first()).toContainText('Hello Nostr world');
  });

  test('shows schedule metadata on review card', async ({ page }) => {
    await page.goto('/proposals');
    await waitForProposalsLoaded(page);
    await expect(page.locator('.review-card').first()).toContainText('Scheduled for');
  });

  test('selecting a card shows bulk review actions', async ({ page }) => {
    await page.goto('/proposals');
    const card = page.locator('.review-card').first();
    await waitForProposalsLoaded(page);
    await card.locator('input[type="checkbox"]').check();
    await expect(page.locator('.bulk-bar')).toContainText('1 selected');
    await expect(page.getByRole('button', { name: 'Approve 1' })).toBeEnabled();
  });

  test('Remove button calls delete API and shows success', async ({ page }) => {
    let deleteCalled = false;
    await page.route(/\/v1\/proposals\/proposal-1$/, (route) => {
      if (route.request().method() === 'DELETE') {
        deleteCalled = true;
        return route.fulfill({ status: 204 });
      }
      return route.continue();
    });

    await page.goto('/proposals');
    await waitForProposalsLoaded(page);

    await page.locator('.review-card').first().getByRole('button', { name: 'Remove' }).click();

    await expect(page.locator('.notice.success')).toContainText('Removed.', { timeout: 8000 });
    expect(deleteCalled).toBe(true);
  });

  test('reject proposal with note shows success', async ({ page }) => {
    await page.route(/\/v1\/proposals\/proposal-1\/reject/, (route) =>
      route.fulfill({ json: { ...mockProposal, state: 'REJECTED' } })
    );

    await page.goto('/proposals');
    await waitForProposalsLoaded(page);

    const card = page.locator('.review-card').first();
    await card.getByRole('button', { name: 'Reject' }).click();
    await page.getByPlaceholder('Optional note for your teammate').fill('Needs edits');
    await card.getByRole('button', { name: 'Reject' }).click();

    await expect(page.locator('.notice.success')).toContainText('Rejected.', { timeout: 8000 });
  });
});
