import { test, expect } from '@playwright/test';
import { seedAuth, mockApi } from './helpers.js';

// All tabs defined in the app (label matches aria-label="${label} tab")
const TABS = [
  { id: 'smart-folder', label: 'Smart Folder' },
  { id: 'file-upload', label: 'File Upload' },
  { id: 'llm-query', label: 'AI Query' },
  { id: 'schemas', label: 'Schema' },
  { id: 'query', label: 'Query' },
  { id: 'mutation', label: 'Mutation' },
  { id: 'ingestion', label: 'JSON Ingestion' },
  { id: 'native-index', label: 'Native Index' },
  { id: 'data-browser', label: 'Data Browser' },
  { id: 'word-graph', label: 'Word Graph' },
];

/** Locate a tab button by its label (uses aria-label="${label} tab"). */
function tabButton(page, label) {
  return page.getByLabel(`${label} tab`, { exact: true });
}

/** Wait for the app to finish loading by checking a tab is visible. */
async function waitForApp(page) {
  await expect(tabButton(page, 'Smart Folder')).toBeVisible({ timeout: 10_000 });
}

test.describe('UI Smoke Tests', () => {
  let consoleErrors;

  test.beforeEach(async ({ page }) => {
    consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', err => consoleErrors.push(err.message));

    await seedAuth(page);
    await mockApi(page);
  });

  test.afterEach(async () => {
    const realErrors = consoleErrors.filter(
      e =>
        !e.includes('EventSource') &&
        !e.includes('net::ERR') &&
        !e.includes('Failed to fetch') &&
        !e.includes('Failed to load resource') &&
        !e.includes('WebSocket') &&
        !e.includes('MIME type') &&
        !e.includes('validateDOMNesting'),
    );
    expect(realErrors, `Unexpected console errors:\n${realErrors.join('\n')}`).toHaveLength(0);
  });

  test('app loads and shows the dashboard', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);
    // Default tab should have aria-current="page"
    await expect(tabButton(page, 'Smart Folder')).toHaveAttribute('aria-current', 'page');
  });

  // Click every tab and verify it renders without crashing
  for (const tab of TABS) {
    test(`navigate to "${tab.label}" tab`, async ({ page }) => {
      await page.goto('/');
      await waitForApp(page);

      await tabButton(page, tab.label).click();

      // URL hash should update
      await expect(page).toHaveURL(new RegExp(`#${tab.id}`));

      // The clicked tab should now be active
      await expect(tabButton(page, tab.label)).toHaveAttribute('aria-current', 'page');

      // Section title should reflect the tab
      const sectionTitle = tab.id.replace('-', ' ');
      await expect(page.locator('.text-xs.uppercase').first()).toContainText(new RegExp(sectionTitle, 'i'));
    });
  }

  test('settings modal opens and cycles through sub-tabs', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);

    // Open settings via the Settings button
    await page.getByRole('button', { name: /settings/i }).click();

    // Modal should be visible
    const modal = page.getByRole('dialog');
    await expect(modal).toBeVisible();

    // Click through each settings sub-tab
    const settingsTabs = ['AI', 'Key', 'Schema', 'Database'];
    for (const label of settingsTabs) {
      const btn = modal.getByRole('button', { name: new RegExp(label, 'i') }).first();
      if (await btn.isVisible()) {
        await btn.click();
        await page.waitForTimeout(200);
      }
    }

    // Close with Escape
    await page.keyboard.press('Escape');
    await expect(modal).not.toBeVisible();
  });

  test('Schema tab shows mock schema', async ({ page }) => {
    await page.goto('/#schemas');
    await waitForApp(page);
    await expect(page.getByText('test_schema')).toBeVisible();
  });

  test('Query tab has execute button', async ({ page }) => {
    await page.goto('/#query');
    await waitForApp(page);
    await expect(page.getByText(/schema/i).first()).toBeVisible();
    await expect(
      page.getByRole('button', { name: /execute|run|query/i }).first(),
    ).toBeVisible();
  });

  test('Mutation tab has schema selector', async ({ page }) => {
    await page.goto('/#mutation');
    await waitForApp(page);
    // Should have schema and operation type selectors
    await expect(page.locator('select').first()).toBeVisible();
  });

  test('JSON Ingestion tab loads sample data', async ({ page }) => {
    await page.goto('/#ingestion');
    await waitForApp(page);

    // Click the first visible sample-data button
    for (const label of ['Blog', 'Twitter', 'Instagram', 'LinkedIn', 'TikTok']) {
      const btn = page.getByRole('button', { name: new RegExp(label, 'i') });
      if (await btn.isVisible()) {
        await btn.click();
        break;
      }
    }
  });

  test('Native Index tab accepts search input', async ({ page }) => {
    await page.goto('/#native-index');
    await waitForApp(page);
    const searchInput = page.getByPlaceholder(/search/i).first();
    await expect(searchInput).toBeVisible();
    await searchInput.fill('hello');
  });

  test('rapid tab switching causes no errors', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);

    for (const tab of TABS) {
      await tabButton(page, tab.label).click();
    }

    await page.waitForTimeout(500);
  });
});
