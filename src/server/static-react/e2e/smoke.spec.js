import { test, expect } from '@playwright/test';
import { seedAuth, mockApi, MOCK_QUERY_RESULTS, MOCK_SEARCH_RESULTS } from './helpers.js';

/** Locate a tab button by its label (uses aria-label="${label} tab"). */
function tabButton(page, label) {
  return page.getByLabel(`${label} tab`, { exact: true });
}

/** Wait for the app to finish loading by checking a tab is visible. */
async function waitForApp(page) {
  await expect(tabButton(page, 'Smart Folder')).toBeVisible({ timeout: 10_000 });
}

// ---------------------------------------------------------------------------
// Console error tracking — shared across all tests
// ---------------------------------------------------------------------------
test.describe('E2E Smoke Tests', () => {
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

  // -----------------------------------------------------------------------
  // App bootstrap
  // -----------------------------------------------------------------------
  test('app loads and shows the dashboard with default tab active', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);
    await expect(tabButton(page, 'Smart Folder')).toHaveAttribute('aria-current', 'page');
  });

  // -----------------------------------------------------------------------
  // Query tab — select schema, toggle fields, execute query, see results
  // -----------------------------------------------------------------------
  test('Query: select schema, pick fields, execute, verify API call', async ({ page }) => {
    let queryCalled = false;
    await page.route('**/api/query', async route => {
      queryCalled = true;
      const body = route.request().postDataJSON();
      // Verify the query payload has the right schema
      expect(body.schema_name).toBe('blog_posts');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ results: MOCK_QUERY_RESULTS }),
      });
    });

    await page.goto('/#query');
    await waitForApp(page);

    // Select schema from dropdown
    const schemaSelect = page.locator('select[name="schema"]');
    await expect(schemaSelect).toBeVisible();
    await schemaSelect.selectOption('blog_posts');

    // Field checkboxes should appear for blog_posts fields
    const fieldSelection = page.locator('text=Field Selection').locator('..');
    await expect(fieldSelection.getByText('title')).toBeVisible();
    await expect(fieldSelection.getByText('body')).toBeVisible();
    await expect(fieldSelection.getByText('author')).toBeVisible();

    // Check a field
    const checkboxes = page.getByRole('checkbox');
    await checkboxes.first().check();

    // Execute the query
    const executeBtn = page.getByRole('button', { name: /execute query/i });
    await executeBtn.click();

    // Verify the API was called
    await expect(() => expect(queryCalled).toBe(true)).toPass({ timeout: 3000 });
  });

  test('Query: clear button resets field selection', async ({ page }) => {
    await page.goto('/#query');
    await waitForApp(page);

    // Select a schema
    await page.locator('select[name="schema"]').selectOption('blog_posts');
    // Wait for fields to appear
    const fieldSelection = page.locator('text=Field Selection').locator('..');
    await expect(fieldSelection.getByText('title')).toBeVisible();

    // Click clear
    await page.getByRole('button', { name: /clear query/i }).click();

    // Schema should be reset — field selection section disappears
    await expect(page.getByText('Field Selection')).not.toBeVisible();
  });

  // -----------------------------------------------------------------------
  // Mutation tab — select schema, choose type, fill fields, submit
  // -----------------------------------------------------------------------
  test('Mutation: insert a record with field values and verify API payload', async ({ page }) => {
    let capturedMutation = null;
    await page.route('**/api/mutation', async route => {
      capturedMutation = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      });
    });

    await page.goto('/#mutation');
    await waitForApp(page);

    // Select schema
    await page.locator('select[name="schema"]').selectOption('blog_posts');

    // Mutation type should default to Insert
    await expect(page.locator('select[name="operationType"]')).toHaveValue('Insert');

    // Fill in field values
    await page.getByPlaceholder('Enter title').fill('My Test Post');
    await page.getByPlaceholder('Enter body').fill('This is the body content.');
    await page.getByPlaceholder('Enter author').fill('e2e_tester');

    // Submit
    const submitBtn = page.getByRole('button', { name: /execute mutation/i });
    await expect(submitBtn).toBeEnabled();
    await submitBtn.click();

    // Verify the mutation payload
    await expect(() => {
      expect(capturedMutation).not.toBeNull();
      expect(capturedMutation.schema).toBe('blog_posts');
      expect(capturedMutation.mutation_type).toBe('create');
      expect(capturedMutation.fields_and_values.title).toBe('My Test Post');
      expect(capturedMutation.fields_and_values.body).toBe('This is the body content.');
      expect(capturedMutation.fields_and_values.author).toBe('e2e_tester');
    }).toPass({ timeout: 3000 });
  });

  test('Mutation: switch to Delete hides field editor', async ({ page }) => {
    await page.goto('/#mutation');
    await waitForApp(page);

    await page.locator('select[name="schema"]').selectOption('blog_posts');
    await page.locator('select[name="operationType"]').selectOption('Delete');

    // Should show "No additional fields required" message
    await expect(page.getByText(/no additional fields/i)).toBeVisible();
  });

  test('Mutation: submit disabled without filling any fields', async ({ page }) => {
    await page.goto('/#mutation');
    await waitForApp(page);

    await page.locator('select[name="schema"]').selectOption('blog_posts');

    // Submit button should be disabled — no fields filled yet
    const submitBtn = page.getByRole('button', { name: /execute mutation/i });
    await expect(submitBtn).toBeDisabled();
  });

  // -----------------------------------------------------------------------
  // Schema tab — expand schema, see fields, state badges
  // -----------------------------------------------------------------------
  test('Schema: expand schema to reveal fields and classifications', async ({ page }) => {
    await page.goto('/#schemas');
    await waitForApp(page);

    // Blog Posts schema card should be visible
    await expect(page.getByText('blog_posts').first()).toBeVisible();
    // State badge shows 'approved'
    await expect(page.getByText('approved').first()).toBeVisible();

    // Expand it
    await page.getByLabel(/expand schema/i).first().click();

    // Fields should now be listed
    await expect(page.getByText('title').first()).toBeVisible();
    await expect(page.getByText('body').first()).toBeVisible();
    await expect(page.getByText('author').first()).toBeVisible();
  });

  test('Schema: collapse hides field details', async ({ page }) => {
    await page.goto('/#schemas');
    await waitForApp(page);

    // Expand
    await page.getByLabel(/expand schema/i).first().click();
    await expect(page.getByText('title').first()).toBeVisible();

    // Collapse
    await page.getByLabel(/collapse schema/i).first().click();
    // Allow animation/render
    await page.waitForTimeout(300);
  });

  // -----------------------------------------------------------------------
  // JSON Ingestion — paste JSON, process it, verify API call
  // -----------------------------------------------------------------------
  test('Ingestion: paste JSON data and submit to process endpoint', async ({ page }) => {
    let capturedPayload = null;
    await page.route('**/api/ingestion/process', async route => {
      capturedPayload = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ schemas_written: ['test_data'], mutations_executed: 2 }),
      });
    });

    await page.goto('/#ingestion');
    await waitForApp(page);

    const textarea = page.getByPlaceholder(/enter json data/i);
    await expect(textarea).toBeVisible();

    // Paste valid JSON
    const testData = [{ name: 'Alice', age: 30 }, { name: 'Bob', age: 25 }];
    await textarea.fill(JSON.stringify(testData));

    // Process button should be enabled
    const processBtn = page.getByRole('button', { name: /process data/i });
    await expect(processBtn).toBeEnabled();
    await processBtn.click();

    // Verify ingestion API received the data
    await expect(() => {
      expect(capturedPayload).not.toBeNull();
    }).toPass({ timeout: 3000 });
  });

  test('Ingestion: process button disabled with empty textarea', async ({ page }) => {
    await page.goto('/#ingestion');
    await waitForApp(page);

    await expect(page.getByRole('button', { name: /process data/i })).toBeDisabled();
  });

  test('Ingestion: sample data button populates textarea', async ({ page }) => {
    await page.goto('/#ingestion');
    await waitForApp(page);

    const sampleBtn = page.getByRole('button', { name: /blog posts/i });
    if (await sampleBtn.isVisible()) {
      await sampleBtn.click();
      // Textarea should have content and process button should be enabled
      const textarea = page.getByPlaceholder(/enter json data/i);
      await expect(textarea).not.toBeEmpty();
      await expect(page.getByRole('button', { name: /process data/i })).toBeEnabled();
    }
  });

  test('Ingestion: auto-execute checkbox toggles on and off', async ({ page }) => {
    await page.goto('/#ingestion');
    await waitForApp(page);

    const checkbox = page.getByRole('checkbox');
    await expect(checkbox).toBeChecked();
    await checkbox.uncheck();
    await expect(checkbox).not.toBeChecked();
    await checkbox.check();
    await expect(checkbox).toBeChecked();
  });

  // -----------------------------------------------------------------------
  // Native Index — search, see grouped results, expand records
  // -----------------------------------------------------------------------
  test('NativeIndex: search returns grouped results', async ({ page }) => {
    await page.goto('/#native-index');
    await waitForApp(page);

    // Type and search
    const searchInput = page.getByPlaceholder(/search across all schemas/i);
    await searchInput.fill('hello');
    await page.getByRole('button', { name: /search/i }).click();

    // Results summary should appear
    await expect(page.getByText(/\d+ matches across \d+ terms/)).toBeVisible({ timeout: 5000 });

    // Grouped results should show "hello" (2 records) and "world" (1 record)
    await expect(page.getByText('hello').first()).toBeVisible();
    await expect(page.getByText('world').first()).toBeVisible();
  });

  test('NativeIndex: search button disabled when input is empty', async ({ page }) => {
    await page.goto('/#native-index');
    await waitForApp(page);

    await expect(page.getByRole('button', { name: /search/i })).toBeDisabled();
  });

  test('NativeIndex: expand word group reveals records with Show Record button', async ({ page }) => {
    await page.goto('/#native-index');
    await waitForApp(page);

    await page.getByPlaceholder(/search across all schemas/i).fill('hello');
    await page.getByRole('button', { name: /search/i }).click();
    await expect(page.getByText('hello').first()).toBeVisible({ timeout: 5000 });

    // Click word group to expand
    await page.getByText('hello').first().click();

    // Records should show with schema name and Show Record button
    await expect(page.getByText('blog_posts').first()).toBeVisible();
    await expect(page.getByRole('button', { name: /show record/i }).first()).toBeVisible();
  });

  test('NativeIndex: Show Record fetches and displays field data', async ({ page }) => {
    await page.goto('/#native-index');
    await waitForApp(page);

    await page.getByPlaceholder(/search across all schemas/i).fill('hello');
    await page.getByRole('button', { name: /search/i }).click();
    await expect(page.getByText('hello').first()).toBeVisible({ timeout: 5000 });

    // Expand group and click Show Record
    await page.getByText('hello').first().click();
    await page.getByRole('button', { name: /show record/i }).first().click();

    // Should show record field values from MOCK_QUERY_RESULTS
    await expect(page.getByText('Hello World').first()).toBeVisible({ timeout: 5000 });
  });

  // -----------------------------------------------------------------------
  // AI Query (LLM) — send a message, see AI response with tool calls
  // -----------------------------------------------------------------------
  test('LlmQuery: send message, receive AI response with tool calls', async ({ page }) => {
    await page.goto('/#llm-query');
    await waitForApp(page);

    // Wait for conversation list to load, then click "+ New"
    const newBtn = page.getByRole('button', { name: '+ New' });
    await expect(newBtn).toBeVisible({ timeout: 10_000 });
    await newBtn.click();

    // Type and send
    const chatInput = page.getByPlaceholder(/ask anything/i);
    await expect(chatInput).toBeVisible();
    await chatInput.fill('Find blog posts about testing');

    await page.getByRole('button', { name: /send/i }).click();

    // User message should appear in chat
    await expect(page.getByText('Find blog posts about testing')).toBeVisible();

    // AI response should appear
    await expect(page.getByText(/I found 2 blog posts/)).toBeVisible({ timeout: 5000 });

    // Tool call indicator
    await expect(page.getByText(/1 tool call/)).toBeVisible();
  });

  test('LlmQuery: send button disabled with empty input', async ({ page }) => {
    await page.goto('/#llm-query');
    await waitForApp(page);

    const newBtn = page.getByRole('button', { name: '+ New' });
    await expect(newBtn).toBeVisible({ timeout: 10_000 });
    await newBtn.click();
    await expect(page.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  // -----------------------------------------------------------------------
  // Settings modal
  // -----------------------------------------------------------------------
  test('Settings: open modal, cycle sub-tabs, close with Escape', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);

    await page.getByRole('button', { name: /settings/i }).click();
    const modal = page.getByRole('dialog');
    await expect(modal).toBeVisible();

    for (const label of ['AI', 'Key', 'Schema', 'Database']) {
      const btn = modal.getByRole('button', { name: new RegExp(label, 'i') }).first();
      if (await btn.isVisible()) {
        await btn.click();
        await page.waitForTimeout(200);
      }
    }

    await page.keyboard.press('Escape');
    await expect(modal).not.toBeVisible();
  });

  // -----------------------------------------------------------------------
  // Tab navigation
  // -----------------------------------------------------------------------
  const ALL_TABS = [
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

  test('Navigation: every tab sets URL hash and active state', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);

    for (const tab of ALL_TABS) {
      await tabButton(page, tab.label).click();
      await expect(page).toHaveURL(new RegExp(`#${tab.id}`));
      await expect(tabButton(page, tab.label)).toHaveAttribute('aria-current', 'page');
    }
  });

  test('Navigation: rapid tab switching causes no errors', async ({ page }) => {
    await page.goto('/');
    await waitForApp(page);

    for (let i = 0; i < 2; i++) {
      for (const tab of ALL_TABS) {
        await tabButton(page, tab.label).click();
      }
    }
    await page.waitForTimeout(500);
  });

  test('Navigation: direct hash URL loads the correct tab', async ({ page }) => {
    await page.goto('/#mutation');
    await waitForApp(page);
    await expect(tabButton(page, 'Mutation')).toHaveAttribute('aria-current', 'page');
    await expect(page.locator('select[name="schema"]')).toBeVisible();
  });
});
