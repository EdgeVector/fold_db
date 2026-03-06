/**
 * Shared helpers for E2E tests.
 * Sets up API mocking and pre-seeds localStorage so the app
 * renders the main dashboard without needing a real backend.
 *
 * NOTE: The ApiClient wraps all responses in { success, data, status, ... }.
 * Mocks return the raw backend response — the client adds the wrapper.
 */

/** Seed localStorage so the app skips the login screen. */
export function seedAuth(page) {
  return page.addInitScript(() => {
    localStorage.setItem('fold_user_id', 'e2e-test-user');
    localStorage.setItem('fold_user_hash', 'e2e-test-hash');
    // Mark onboarding completed so the wizard doesn't appear
    localStorage.setItem('folddb_onboarding_completed_e2e-test-hash', '1');
    // Dismiss AI setup banner
    localStorage.setItem('folddb_setup_dismissed', '1');
  });
}

/** Helper: fulfill with JSON and correct content-type header. */
function json(route, body) {
  return route.fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify(body),
  });
}

/** Mock all backend API routes so the app renders without a real server. */
export async function mockApi(page) {
  // System endpoints
  await page.route('**/api/system/auto-identity', route =>
    json(route, { user_id: 'e2e-test-user', user_hash: 'e2e-test-hash' }),
  );
  await page.route('**/api/system/status', route =>
    json(route, { status: 'ok', version: '0.1.0', storage_mode: 'local' }),
  );
  await page.route('**/api/system/database-status', route =>
    json(route, { initialized: true, has_saved_config: true }),
  );
  await page.route('**/api/system/database-config', route =>
    json(route, { storage_mode: 'local' }),
  );
  await page.route('**/api/system/public-key', route =>
    json(route, { public_key: 'e2e-mock-key' }),
  );
  await page.route('**/api/system/private-key', route =>
    json(route, null),
  );
  await page.route('**/api/security/system-key', route =>
    json(route, { public_key: 'e2e-mock-system-key' }),
  );

  // Schema endpoints — backend returns { schemas: [...] }
  const mockSchema = {
    name: 'test_schema',
    state: 'approved',
    fields: {
      title: { field_type: 'Single', permission: { owner_read: true, owner_write: true } },
      body: { field_type: 'Single', permission: { owner_read: true, owner_write: true } },
    },
  };
  await page.route('**/api/schemas', route =>
    json(route, { schemas: [mockSchema], count: 1 }),
  );
  await page.route('**/api/schema/*', route =>
    json(route, mockSchema),
  );

  // Ingestion config (AI settings)
  await page.route('**/api/ingestion/config', route =>
    json(route, { openrouter_api_key: null, ollama_url: null, model: null, provider: null }),
  );

  // Query / mutation / search — return empty results
  await page.route('**/api/query', route =>
    json(route, { results: [] }),
  );
  await page.route('**/api/mutation', route =>
    json(route, {}),
  );
  await page.route('**/api/native-index/search', route =>
    json(route, { results: [] }),
  );
  await page.route('**/api/ingestion/process', route =>
    json(route, { schemas_written: [] }),
  );
  await page.route('**/api/ingestion/upload', route =>
    json(route, {}),
  );
  await page.route('**/api/llm-query/**', route =>
    json(route, { response: 'mock reply', results: [] }),
  );
  await page.route('**/api/logs/**', route =>
    json(route, []),
  );
  await page.route('**/api/logs', route =>
    json(route, []),
  );
  await page.route('**/api/indexing/**', route =>
    json(route, {}),
  );
  await page.route('**/api/ingestion/progress', route =>
    json(route, { active: false, progress: null }),
  );
  await page.route('**/api/system/complete-path', route =>
    json(route, { completions: [] }),
  );

  // Log stream (EventSource — return empty event stream to avoid errors)
  await page.route('**/api/logs/stream', route =>
    route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: 'data: []\n\n',
    }),
  );
}
