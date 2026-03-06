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

/**
 * Rich mock schemas for testing real interactions.
 */
export const MOCK_SCHEMAS = [
  {
    name: 'blog_posts',
    descriptive_name: 'Blog Posts',
    state: 'approved',
    fields: ['title', 'body', 'author'],
    field_classifications: {
      title: ['text'],
      body: ['text'],
      author: ['text'],
    },
  },
  {
    name: 'user_profiles',
    descriptive_name: 'User Profiles',
    state: 'approved',
    fields: ['username', 'email', 'bio'],
    field_classifications: {
      username: ['text'],
      email: ['text'],
      bio: ['text'],
    },
  },
  {
    name: 'pending_schema',
    descriptive_name: 'Pending Schema',
    state: 'available',
    fields: ['data'],
    field_classifications: { data: ['text'] },
  },
];

/** Mock query results returned by /api/query */
export const MOCK_QUERY_RESULTS = [
  {
    key: { hash: 'abc123', range: null },
    fields: {
      title: 'Hello World',
      body: 'This is a test blog post.',
      author: 'test_user',
    },
  },
  {
    key: { hash: 'def456', range: null },
    fields: {
      title: 'Second Post',
      body: 'Another test post.',
      author: 'test_user',
    },
  },
];

/** Mock native index search results */
export const MOCK_SEARCH_RESULTS = [
  {
    value: 'hello',
    schema_name: 'blog_posts',
    field: 'title',
    key_value: { hash: 'abc123', range: null },
  },
  {
    value: 'hello',
    schema_name: 'blog_posts',
    field: 'body',
    key_value: { hash: 'abc123', range: null },
  },
  {
    value: 'world',
    schema_name: 'blog_posts',
    field: 'title',
    key_value: { hash: 'abc123', range: null },
  },
];

/** Mock all backend API routes so the app renders without a real server. */
export async function mockApi(page, overrides = {}) {
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

  // Schema endpoints
  const schemas = overrides.schemas || MOCK_SCHEMAS;
  await page.route('**/api/schemas', route =>
    json(route, { schemas, count: schemas.length }),
  );
  await page.route('**/api/schema/*/approve', route =>
    json(route, { success: true, backfillHash: 'mock-backfill-123' }),
  );
  await page.route('**/api/schema/*/block', route =>
    json(route, { success: true }),
  );
  await page.route('**/api/schema/*', route =>
    json(route, schemas[0]),
  );

  // Ingestion config (AI settings)
  await page.route('**/api/ingestion/config', route =>
    json(route, { openrouter_api_key: null, ollama_url: null, model: null, provider: null }),
  );

  // Query — return mock results
  const defaultQueryHandler = route => json(route, { results: MOCK_QUERY_RESULTS });
  await page.route('**/api/query', overrides.queryHandler || defaultQueryHandler);

  // Mutation — return success
  const defaultMutationHandler = route => json(route, { ok: true });
  await page.route('**/api/mutation', overrides.mutationHandler || defaultMutationHandler);

  // Native index search — use regex to match URL with query params
  const defaultSearchHandler = route => json(route, { results: MOCK_SEARCH_RESULTS });
  await page.route(/\/api\/native-index\/search/, overrides.searchHandler || defaultSearchHandler);

  // Ingestion
  const defaultIngestionHandler = route => json(route, { schemas_written: ['blog_posts'], mutations_executed: 3 });
  await page.route('**/api/ingestion/process', overrides.ingestionHandler || defaultIngestionHandler);
  await page.route('**/api/ingestion/upload', route =>
    json(route, { schema: 'uploaded_schema', new_schema: true, mutations_generated: 5, mutations_executed: 5 }),
  );

  // LLM query — use regex to match all sub-paths
  const defaultLlmHandler = route => json(route, {
    answer: 'Based on your data, I found 2 blog posts about testing.',
    session_id: 'mock-session-001',
    tool_calls: [
      { tool: 'query', input: { schema: 'blog_posts' }, output: { results: MOCK_QUERY_RESULTS } },
    ],
  });
  await page.route(/\/api\/llm-query\//, overrides.llmHandler || defaultLlmHandler);

  // Logs
  await page.route('**/api/logs/**', route => json(route, []));
  await page.route('**/api/logs', route => json(route, []));

  // Indexing, progress, path completion
  await page.route('**/api/indexing/**', route => json(route, {}));
  await page.route('**/api/ingestion/progress', route =>
    json(route, { active: false, progress: null }),
  );
  await page.route(/\/api\/system\/complete-path/, route =>
    json(route, { completions: ['/home/user/Documents', '/home/user/Downloads'] }),
  );

  // Log stream (EventSource)
  await page.route('**/api/logs/stream', route =>
    route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: 'data: []\n\n',
    }),
  );
}
