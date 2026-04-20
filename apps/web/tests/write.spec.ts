import { test, expect } from '@playwright/test';
import {
  injectMockNostr,
  mockAuthenticatedSession,
  MOCK_PUBKEY,
  MOCK_TOKEN,
  API_BASE
} from './helpers/api-mock';

test.describe('Write page', () => {
  test.beforeEach(async ({ page }) => {
    await mockAuthenticatedSession(page);
    await injectMockNostr(page, MOCK_PUBKEY);
    await page.addInitScript(
      ({ token, pubkey }) => {
        localStorage.setItem('shipyard.session_token', token);
        localStorage.setItem('shipyard.owner_pubkey', pubkey);
      },
      { token: MOCK_TOKEN, pubkey: MOCK_PUBKEY }
    );
    await page.addInitScript(() => {
      class RelaySocket extends EventTarget {
        url: string;

        constructor(url: string) {
          super();
          this.url = url;
          window.setTimeout(() => this.dispatchEvent(new Event('open')), 0);
        }

        send(data: string) {
          localStorage.setItem('shipyard.test.relay_event', data);
          const [, event] = JSON.parse(data) as [string, { id: string }];
          window.setTimeout(
            () =>
              this.dispatchEvent(
                new MessageEvent('message', { data: JSON.stringify(['OK', event.id, true, '']) })
              ),
            0
          );
        }

        close() {}
      }

      Object.defineProperty(window, 'WebSocket', { configurable: true, value: RelaySocket });
    });
  });

  test('send now publishes directly to relays without scheduling through Shipyard', async ({ page }) => {
    const scheduleRequests: string[] = [];
    page.on('request', (request) => {
      if (request.url() === `${API_BASE}/v1/publish-items/schedule`) {
        scheduleRequests.push(request.url());
      }
    });

    await page.goto('/write');
    await page.waitForLoadState('networkidle');
    await expect(page.getByPlaceholder("What's on your mind?")).toBeVisible({ timeout: 8000 });
    await page.getByLabel('Publish mode').selectOption('SEND_NOW');
    await expect(page.getByRole('button', { name: 'Send Now' })).toBeVisible();
    await page.getByPlaceholder("What's on your mind?").fill('Direct relay publish');
    await page.getByRole('button', { name: 'Send Now' }).click();

    await expect(page.getByText('Published.')).toBeVisible();
    const relayEvent = await page.evaluate(() => localStorage.getItem('shipyard.test.relay_event'));
    expect(relayEvent).toContain('Direct relay publish');
    expect(scheduleRequests).toHaveLength(0);
  });
});
