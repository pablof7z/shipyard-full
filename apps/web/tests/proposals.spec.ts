import { test, expect } from '@playwright/test';
import {
  mockAuthenticatedSession,
  mockProposal,
  MOCK_TOKEN,
  MOCK_PUBKEY,
  API_BASE
} from './helpers/api-mock';

test.describe('Proposals page — unauthenticated', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => localStorage.clear());
    await page.route(`${API_BASE}/v1/proposals`, (route) =>
      route.fulfill({ json: [] })
    );
    await page.route(`${API_BASE}/v1/queues`, (route) =>
      route.fulfill({ json: [] })
    );
  });

  test('renders page title', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page).toHaveTitle('Proposals - Shipyard');
  });

  test('shows Proposals heading', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('heading', { name: 'Proposals', level: 1 })).toBeVisible();
  });

  test('shows eyebrow label "Review"', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.locator('.eyebrow')).toHaveText('Review');
  });

  test('shows session notice when not logged in', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.locator('.notice').getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/settings#login');
  });

  test('shows Create Proposal form', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('heading', { name: 'Create Proposal', level: 2 })).toBeVisible();
    await expect(page.getByPlaceholder('Draft note content')).toBeVisible();
  });

  test('Submit button is disabled when unsigned event is empty', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('button', { name: 'Submit' })).toBeDisabled();
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

  /** Wait for proposals panel to finish loading */
  async function waitForProposalsLoaded(page: import('@playwright/test').Page) {
    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    await expect(panel.locator('.rows .row').first())
      .not.toContainText('Loading', { timeout: 8000 });
  }

  test('lists pending proposals from API', async ({ page }) => {
    await page.goto('/proposals');
    await waitForProposalsLoaded(page);
    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    await expect(panel.getByRole('article').first()).toContainText('Hello Nostr world');
  });

  test('shows status badge on proposal row', async ({ page }) => {
    await page.goto('/proposals');
    await waitForProposalsLoaded(page);
    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    // StatusBadge renders 'Proposed' for PROPOSED state
    await expect(panel.locator('.rows .row').first()).toContainText('Proposed');
  });

  test('Build Unsigned JSON button populates the textarea', async ({ page }) => {
    await page.goto('/proposals');
    // Wait for full Svelte hydration — dev server needs networkidle for onclick handlers
    await page.waitForLoadState('networkidle');

    await page.getByPlaceholder('Draft note content').fill('My test note');
    await page.getByRole('button', { name: 'Build Unsigned JSON' }).click();

    // The unsigned event textarea has rows=12 and is the first spellcheck="false" textarea
    const unsignedTextarea = page.locator('textarea[spellcheck="false"]').first();
    await expect(unsignedTextarea).toHaveValue(/My test note/, { timeout: 5000 });
    await expect(unsignedTextarea).toHaveValue(/"kind": 1/);
  });

  test('Select button populates the proposal select dropdown', async ({ page }) => {
    await page.goto('/proposals');
    await waitForProposalsLoaded(page);
    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    await panel.getByRole('button', { name: 'Select' }).first().click();

    const ownerActionForm = page.locator('.card-form').filter({ hasText: 'Owner Action' });
    const proposalSelect = ownerActionForm.locator('select').first();
    await expect(proposalSelect).toHaveValue('proposal-1');
  });

  test('Reject button is disabled when no proposal is selected', async ({ page }) => {
    await page.goto('/proposals');
    await expect(page.getByRole('button', { name: 'Reject' })).toBeDisabled();
  });

  test('Cancel button calls delete API and shows success', async ({ page }) => {
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

    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    await panel.getByRole('button', { name: 'Cancel' }).first().click();

    await expect(page.locator('.notice.success')).toContainText('Proposal cancelled', { timeout: 8000 });
    expect(deleteCalled).toBe(true);
  });

  test('reject selected proposal shows success', async ({ page }) => {
    await page.route(/\/v1\/proposals\/proposal-1\/reject/, (route) =>
      route.fulfill({ json: { ...mockProposal, state: 'REJECTED' } })
    );

    await page.goto('/proposals');
    await waitForProposalsLoaded(page);

    const panel = page.locator('.card-form').filter({ hasText: 'Pending Proposals' });
    await panel.getByRole('button', { name: 'Select' }).first().click();
    await page.getByRole('button', { name: 'Reject' }).click();

    await expect(page.locator('.notice.success')).toContainText('Proposal rejected', { timeout: 8000 });
  });
});
