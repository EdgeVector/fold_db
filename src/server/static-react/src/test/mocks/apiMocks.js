/**
 * @fileoverview API Mocking Utilities for Testing
 * 
 * Provides comprehensive API mocking utilities using MSW (Mock Service Worker)
 * for testing the unified API client system. Includes handlers for all major
 * API endpoints with realistic response patterns and error scenarios.
 * 
 * TASK-006: Testing Enhancement - Created API mocking utilities
 * 
 * @module apiMocks
 * @since 2.0.0
 */

import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import { beforeAll, afterEach, afterAll } from 'vitest';
import {
  createMockSchema,
  createMockRangeSchema,
  mockDelay,
  SCHEMA_STATES
} from '../utils/testUtilities.jsx';
import {
  MOCK_API_DELAY_MS,
  TEST_TIMEOUT_DEFAULT_MS
} from '../config/constants';

// ============================================================================
// MOCK DATA
// ============================================================================

/**
 * Mock schemas for testing
 */
export const mockSchemas = {
  user_profiles: createMockSchema({
    name: 'user_profiles',
    descriptive_name: 'User Profiles',
    state: SCHEMA_STATES.APPROVED,
    fields: {
      id: { field_type: 'String' },
      name: { field_type: 'String' },
      email: { field_type: 'String' },
      age: { field_type: 'Number' }
    }
  }),
  time_series: createMockRangeSchema({
    name: 'time_series',
    descriptive_name: 'Time Series',
    state: SCHEMA_STATES.APPROVED
  }),
  events: createMockSchema({
    name: 'events',
    descriptive_name: 'Events',
    state: SCHEMA_STATES.AVAILABLE,
    fields: {
      event_id: { field_type: 'String' },
      event_type: { field_type: 'String' },
      timestamp: { field_type: 'String' }
    }
  }),
  blocked_schema: createMockSchema({
    name: 'blocked_schema',
    descriptive_name: 'Blocked Schema',
    state: SCHEMA_STATES.BLOCKED,
    fields: {
      id: { field_type: 'String' },
      data: { field_type: 'String' }
    }
  })
};

/**
 * Mock persisted schema states
 */
export const mockPersistedStates = {
  user_profiles: SCHEMA_STATES.APPROVED,
  time_series: SCHEMA_STATES.APPROVED,
  events: SCHEMA_STATES.AVAILABLE,
  blocked_schema: SCHEMA_STATES.BLOCKED
};

/**
 * Mock available schema names
 */
export const mockAvailableSchemas = Object.keys(mockSchemas);

/**
 * Mock authentication data
 */
export const mockAuthData = {
  systemPublicKey: 'mock_system_public_key_123',
  isValid: true,
  keyId: 'system_key_001'
};

/**
 * Mock mutation results
 */
export const mockMutationResults = {
  create: {
    success: true,
    data: { id: 'created_item_123', status: 'created' }
  },
  update: {
    success: true,
    data: { id: 'updated_item_456', status: 'updated' }
  },
  delete: {
    success: true,
    data: { id: 'deleted_item_789', status: 'deleted' }
  }
};

/**
 * Mock query results
 */
export const mockQueryResults = {
  basic: {
    success: true,
    data: [
      { id: '1', name: 'John Doe', email: 'john@example.com' },
      { id: '2', name: 'Jane Smith', email: 'jane@example.com' }
    ]
  },
  range: {
    success: true,
    data: [
      { timestamp: '2025-01-01T00:00:00Z', value: 100 },
      { timestamp: '2025-01-01T01:00:00Z', value: 105 }
    ]
  },
  empty: {
    success: true,
    data: []
  }
};

// ============================================================================
// MSW HANDLERS
// ============================================================================

/**
 * Default MSW request handlers for API endpoints
 */
export const defaultHandlers = [
  // Security endpoints - missing handlers causing unhandled request errors
  http.post('/api/security/verify', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: { verified: true, message: 'Verification successful' }
      })
    );
  }),

  http.get('/api/security/system-public-key', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          publicKey: 'mock-system-public-key-base64',
          keyId: 'system-key-1',
          algorithm: 'Ed25519'
        }
      })
    );
  }),

  http.post('/api/validate', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: { valid: true, errors: [] }
      })
    );
  }),

  // Schema endpoints - with /api/ prefix for legacy compatibility
  http.get('/api/schemas/available', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: mockAvailableSchemas
      })
    );
  }),

  http.get('/api/schemas', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: mockPersistedStates
      })
    );
  }),

  // Schema endpoints - without /api/ prefix for schemaClient
  http.get('/schemas', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: Object.values(mockSchemas) // Return array of schemas as schemaClient expects
      })
    );
  }),

  // Test compatibility: Add /api/schemas endpoint that tests expect
  http.get('/api/schemas', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: Object.values(mockSchemas) // Return array of schemas as tests expect
      })
    );
  }),

  http.get('/schema/:schemaName', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: schema
      })
    );
  }),

  http.get('/api/schema/:schemaName', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: schema,
        ...schema
      })
    );
  }),

  // Schema operations
  http.post('/api/schema/:schemaName/approve', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }

    if (schema.state !== SCHEMA_STATES.AVAILABLE) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} cannot be approved from state ${schema.state}` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          schema: { ...schema, state: SCHEMA_STATES.APPROVED }
        }
      })
    );
  }),

  http.post('/api/schema/:schemaName/block', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          schema: { ...schema, state: SCHEMA_STATES.BLOCKED }
        }
      })
    );
  }),

  http.post('/api/schema/:schemaName/load', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          schema: { ...schema, state: SCHEMA_STATES.APPROVED }
        }
      })
    );
  }),

  http.post('/api/schema/:schemaName/unload', async (req, res, ctx) => {
    const { schemaName } = req.params;
    await mockDelay(MOCK_API_DELAY_MS);
    
    const schema = mockSchemas[schemaName];
    if (!schema) {
      return res(
        ctx.status(404),
        ctx.json({
          success: false,
          error: { message: `Schema ${schemaName} not found` }
        })
      );
    }

    if (schema.state === SCHEMA_STATES.APPROVED) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: `Cannot unload approved schema ${schemaName}` }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          schema: { ...schema, state: SCHEMA_STATES.AVAILABLE }
        }
      })
    );
  }),

  // Mutation endpoints
  http.post('/api/mutation', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    // Validate mutation structure
    if (!body.schema || !body.mutation_type || !body.data) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: 'Invalid mutation structure' }
        })
      );
    }

    // Check if schema is approved
    const schema = mockSchemas[body.schema];
    if (!schema || schema.state !== SCHEMA_STATES.APPROVED) {
      return res(
        ctx.status(403),
        ctx.json({
          success: false,
          error: { message: 'Only approved schemas can be used for mutations' }
        })
      );
    }

    const mutationType = body.mutation_type.toLowerCase();
    const result = mockMutationResults[mutationType] || mockMutationResults.create;
    
    return res(
      ctx.status(200),
      ctx.json(result)
    );
  }),

  // Query endpoints
  http.post('/api/query', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    // Validate query structure
    if (!body.schema || !body.fields) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: 'Invalid query structure' }
        })
      );
    }

    // Check if schema is approved
    const schema = mockSchemas[body.schema];
    if (!schema || schema.state !== SCHEMA_STATES.APPROVED) {
      return res(
        ctx.status(403),
        ctx.json({
          success: false,
          error: { message: 'Only approved schemas can be used for queries' }
        })
      );
    }

    // Return different results based on schema type
    const isRangeSchema = schema.schema_type?.Range;
    const result = isRangeSchema ? mockQueryResults.range : mockQueryResults.basic;
    
    return res(
      ctx.status(200),
      ctx.json(result)
    );
  }),

  // Native index search endpoint
  http.get('/api/native-index/search', async (req, res, ctx) => {
    const term = req.url.searchParams.get('term') || '';
    await mockDelay(MOCK_API_DELAY_MS);
    const sample = [
      {
        schema_name: 'user_profiles',
        field: 'name',
        key_value: { hash: null, range: 'user-1' },
        value: 'John Doe',
        metadata: { word: term.toLowerCase() }
      },
      {
        schema_name: 'time_series',
        field: 'value',
        key_value: { hash: 'metrics', range: '2025-01-01T00:00:00Z' },
        value: 100,
        metadata: { word: term.toLowerCase() }
      }
    ];
    return res(ctx.status(200), ctx.json(sample));
  }),

  // Security endpoints
  http.get('/api/security/system-public-key', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: mockAuthData
      })
    );
  }),

  http.post('/api/security/register-key', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    if (!body.publicKey || !body.signature) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: 'Missing required fields' }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          keyId: `user_key_${Date.now()}`,
          status: 'registered'
        }
      })
    );
  }),

  http.post('/api/security/verify', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    if (!body.message || !body.signature) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: 'Missing message or signature' }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: { valid: true }
      })
    );
  }),

  // Key Lifecycle Management endpoints
  http.post('/api/keys', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    if (!body.publicKey) {
      return res(
        ctx.status(400),
        ctx.json({
          success: false,
          error: { message: 'Missing required field: publicKey' }
        })
      );
    }
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          id: `key_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
          ...body,
          status: 'active',
          createdAt: new Date().toISOString()
        }
      })
    );
  }),

  http.get('/api/keys', async (req, res, ctx) => {
    await mockDelay(MOCK_API_DELAY_MS);
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: [
          {
            id: 'key_1',
            publicKey: 'mock_public_key_1',
            status: 'active',
            algorithm: 'Ed25519',
            createdAt: new Date().toISOString()
          }
        ]
      })
    );
  }),

  http.patch('/api/keys/:keyId', async (req, res, ctx) => {
    const body = await req.json();
    await mockDelay(MOCK_API_DELAY_MS);
    
    return res(
      ctx.status(200),
      ctx.json({
        success: true,
        data: {
          id: req.params.keyId,
          ...body,
          updatedAt: new Date().toISOString()
        }
      })
    );
  }),

  // Dynamic route for key operations (PATCH /api/keys/:keyId)
  http.patch('/api/keys/:keyId', async (req, res, ctx) => {
    const { keyId } = req.params;
    const body = await req.json();
    
    return res(
      ctx.json({
        success: true,
        data: {
          id: keyId,
          ...body,
          updatedAt: new Date().toISOString()
        }
      })
    );
  })
];

// ============================================================================
// ERROR SCENARIOS
// ============================================================================

/**
 * Handlers for testing error scenarios
 */
export const errorHandlers = {
  networkError: [
    http.get('/api/schemas/available', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    }),
    http.get('/schemas', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    }),
    http.get('/schema/:schemaName', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    }),
    http.get('/schemas/state/:state', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    }),
    http.get('/api/keys', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    }),
    http.patch('/api/keys/:keyId', (req, res, _ctx) => {
      return res.networkError('Failed to connect');
    })
  ],

  serverError: [
    http.get('/api/schemas/available', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Internal server error' }
        })
      );
    }),
    http.get('/schemas', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Internal server error' }
        })
      );
    }),
    http.get('/schema/:schemaName', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Failed to fetch schema' }
        })
      );
    }),
    http.get('/schemas/state/:state', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Failed to fetch schemas by state' }
        })
      );
    }),
    http.get('/api/keys', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Internal server error' }
        })
      );
    }),
    http.post('/api/keys', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Failed to store key' }
        })
      );
    }),
    http.patch('/api/keys/:keyId', (req, res, ctx) => {
      return res(
        ctx.status(500),
        ctx.json({
          success: false,
          error: { message: 'Failed to update key' }
        })
      );
    })
  ],

  timeout: [
    http.get('/api/schemas/available', (req, res, ctx) => {
      return res(ctx.delay(TEST_TIMEOUT_MS + 1000));
    })
  ],

  unauthorized: [
    http.post('/api/mutation', (req, res, ctx) => {
      return res(
        ctx.status(401),
        ctx.json({
          success: false,
          error: { message: 'Unauthorized' }
        })
      );
    })
  ],

  schemaNotApproved: [
    http.post('/api/mutation', (req, res, ctx) => {
      return res(
        ctx.status(403),
        ctx.json({
          success: false,
          error: { message: 'Schema not approved for mutations' }
        })
      );
    })
  ]
};

// ============================================================================
// TEST SERVER SETUP
// ============================================================================

/**
 * Creates a configured MSW server for testing
 * 
 * @param {Array} customHandlers - Additional or override handlers
 * @returns {Object} MSW server instance
 */
export const createMockServer = (customHandlers = []) => {
  const handlers = [...defaultHandlers, ...customHandlers];
  return setupServer(...handlers);
};

/**
 * Default mock server instance
 */
export const mockServer = createMockServer();

// ============================================================================
// MOCK API CLIENT
// ============================================================================

/**
 * Creates a mock API client for testing without network calls
 * 
 * @param {Object} overrides - Override specific methods
 * @returns {Object} Mock API client
 */
export const createMockApiClient = (overrides = {}) => {
  const defaultClient = {
    get: vi.fn().mockResolvedValue({ success: true, data: {} }),
    post: vi.fn().mockResolvedValue({ success: true, data: {} }),
    put: vi.fn().mockResolvedValue({ success: true, data: {} }),
    delete: vi.fn().mockResolvedValue({ success: true, data: {} }),
    patch: vi.fn().mockResolvedValue({ success: true, data: {} }),
    batch: vi.fn().mockResolvedValue([]),
    getMetrics: vi.fn().mockReturnValue({
      averageResponseTime: 100,
      cacheHitRate: 0.8,
      totalRequests: 50
    }),
    getCacheStats: vi.fn().mockReturnValue({
      size: 10,
      hitRate: 0.8
    }),
    clearCache: vi.fn(),
    ...overrides
  };

  return defaultClient;
};

/**
 * Creates mock specialized API clients
 * 
 * @param {Object} baseClient - Base API client to use
 * @returns {Object} Mock specialized clients
 */
export const createMockSpecializedClients = (_baseClient) => {
  return {
    schema: {
      getSchemas: vi.fn().mockResolvedValue(Object.values(mockSchemas)),
      getApprovedSchemas: vi.fn().mockResolvedValue(
        Object.values(mockSchemas).filter(s => s.state === SCHEMA_STATES.APPROVED)
      ),
      getSchema: vi.fn().mockImplementation((name) => 
        Promise.resolve(mockSchemas[name] || null)
      ),
      approveSchema: vi.fn().mockResolvedValue({
        success: true,
        data: { schema: { state: SCHEMA_STATES.APPROVED } }
      }),
      blockSchema: vi.fn().mockResolvedValue({
        success: true,
        data: { schema: { state: SCHEMA_STATES.BLOCKED } }
      })
    },

    mutation: {
      executeMutation: vi.fn().mockResolvedValue(mockMutationResults.create),
      validateMutation: vi.fn().mockReturnValue(null),
      validateSchemaForMutation: vi.fn().mockReturnValue(true)
    },

    query: {
      executeQuery: vi.fn().mockResolvedValue(mockQueryResults.basic)
    },

    security: {
      getSystemPublicKey: vi.fn().mockResolvedValue(mockAuthData),
      verifyMessage: vi.fn().mockResolvedValue({
        success: true,
        data: { valid: true }
      })
    }
  };
};

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Sets up MSW server for tests
 * Call this in test setup
 */
export const setupMockServer = () => {
  beforeAll(() => mockServer.listen({ onUnhandledRequest: 'error' }));
  afterEach(() => mockServer.resetHandlers());
  afterAll(() => mockServer.close());
};

/**
 * Temporarily override handlers for specific tests
 * 
 * @param {Array} handlers - Temporary handlers
 * @param {Function} testFn - Test function to run with handlers
 */
export const withMockHandlers = async (handlers, testFn) => {
  mockServer.use(...handlers);
  try {
    await testFn();
  } finally {
    mockServer.resetHandlers();
  }
};

/**
 * Simulate slow network conditions
 * 
 * @param {number} delay - Delay in milliseconds
 * @returns {Array} Handlers with delay
 */
export const createSlowHandlers = (delay = 2000) => {
  return defaultHandlers.map(handler => {
    return http[handler.info.method.toLowerCase()](handler.info.path, async (req, res, ctx) => {
      await mockDelay(delay);
      return handler.resolver(req, res, ctx);
    });
  });
};

// Export everything
export default {
  mockSchemas,
  mockPersistedStates,
  mockAvailableSchemas,
  mockAuthData,
  mockMutationResults,
  mockQueryResults,
  defaultHandlers,
  errorHandlers,
  createMockServer,
  mockServer,
  createMockApiClient,
  createMockSpecializedClients,
  setupMockServer,
  withMockHandlers,
  createSlowHandlers
};