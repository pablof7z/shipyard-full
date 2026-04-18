import { test, expect } from '@playwright/test';
import {
  mockAuthenticatedSession,
  mockQueue,
  MOCK_TOKEN,
  MOCK_PUBKEY,
  API_BASE
} from './helpers/api-mock';

test.describe('Queues page — unauthenticated', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => localStorage.clear());
    await page.route(`${API_BASE}/v1/queues`, (route) =>
      route.fulfill({ json: [] })
    );
  });

  test('renders page title', async ({ page }) => {
    await page.goto('/queues');
    await expect(page).toHaveTitle('Queues - Shipyard');
  });

  test('shows Queues heading', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.getByRole('heading', { name: 'Queues', level: 1 })).toBeVisible();
  });

  test('shows eyebrow label "Scheduling"', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.locator('.eyebrow')).toHaveText('Scheduling');
  });

  test('shows session notice when not logged in', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.locator('.notice')).toContainText('Settings');
  });

  test('shows Create Queue form', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.getByRole('heading', { name: 'Create Queue', level: 2 })).toBeVisible();
    await expect(page.getByPlaceholder('Weekday posts')).toBeVisible();
  });

  test('Create button is disabled when name is empty', async ({ page }) => {
    await page.goto('/queues');
    await expect(page.getByRole('button', { name: 'Create' })).toBeDisabled();
  });
});

test.describe('Queues page — authenticated', () => {
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

  /** Wait for active queues to finish loading */
  async function waitForQueuesLoaded(page: import('@playwright/test').Page) {
    const activeSection = page.locator('.card-form').filter({ hasText: 'Active Queues' });
    await expect(activeSection.locator('.rows.compact .row').first())
      .not.toContainText('Loading', { timeout: 8000 });
  }

  test('lists active queues from API', async ({ page }) => {
    await page.goto('/queues');
    await waitForQueuesLoaded(page);
    await expect(page.getByText('Weekday Posts')).toBeVisible();
  });

  test('shows queue cadence label (1d for 86400s)', async ({ page }) => {
    await page.goto('/queues');
    await waitForQueuesLoaded(page);
    const activeSection = page.locator('.card-form').filter({ hasText: 'Active Queues' });
    await expect(activeSection.locator('.rows.compact .row').first()).toContainText('1d');
  });

  test('queue Select button allows editing', async ({ page }) => {
    await page.goto('/queues');
    await waitForQueuesLoaded(page);

    const activeSection = page.locator('.card-form').filter({ hasText: 'Active Queues' });
    await activeSection.getByRole('button', { name: 'Select' }).first().click();

    await expect(page.getByRole('heading', { name: 'Update Queue', level: 2 })).toBeVisible();

    // After selecting, editName is bound to the input — check via toHaveValue (DOM property)
    const updateForm = page.locator('.card-form').filter({ hasText: 'Update Queue' });
    await expect(updateForm.locator('input').first()).toHaveValue('Weekday Posts');
  });

  test('Create button enables when name is filled', async ({ page }) => {
    await page.goto('/queues');
    await page.getByPlaceholder('Weekday posts').fill('New Test Queue');
    await expect(page.getByRole('button', { name: 'Create' })).toBeEnabled();
  });

  test('create queue submits to API and shows success', async ({ page }) => {
    const newQueue = { ...mockQueue, id: 'queue-new', name: 'New Test Queue' };
    let postCalled = false;
    let getCallCount = 0;

    await page.route(`${API_BASE}/v1/queues`, (route) => {
      if (route.request().method() === 'POST') {
        postCalled = true;
        return route.fulfill({ json: newQueue });
      }
      getCallCount++;
      return route.fulfill({ json: getCallCount === 1 ? [mockQueue] : [mockQueue, newQueue] });
    });

    await page.goto('/queues');
    await page.getByPlaceholder('Weekday posts').fill('New Test Queue');
    await page.getByRole('button', { name: 'Create' }).click();

    await expect(page.locator('.notice.success')).toContainText('Queue created', { timeout: 8000 });
    expect(postCalled).toBe(true);
  });

  test('archive queue button calls API and shows success', async ({ page }) => {
    let archiveCalled = false;
    await page.route(/\/v1\/queues\/queue-1\/archive/, (route) => {
      archiveCalled = true;
      return route.fulfill({ json: { ...mockQueue, archived_at: new Date().toISOString() } });
    });

    await page.goto('/queues');
    await waitForQueuesLoaded(page);

    const activeSection = page.locator('.card-form').filter({ hasText: 'Active Queues' });
    await activeSection.getByRole('button', { name: 'Archive' }).first().click();

    await expect(page.locator('.notice.success')).toContainText('Queue archived', { timeout: 8000 });
    expect(archiveCalled).toBe(true);
  });
});
