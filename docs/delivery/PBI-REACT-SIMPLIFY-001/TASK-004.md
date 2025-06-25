# TASK-004: API Client Standardization and Unification

[Back to task list](./tasks.md)

## Description

Create a unified API client architecture to standardize HTTP communications, error handling, and authentication patterns across all frontend API interactions. This task will consolidate the existing API clients ([`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts), [`mutationClient.ts`](../../../src/datafold_node/static-react/src/api/mutationClient.ts), [`securityClient.ts`](../../../src/datafold_node/static-react/src/api/securityClient.ts)) into a cohesive, type-safe API layer.

The current API clients have inconsistent patterns for error handling, response typing, and authentication wrapper usage, leading to maintenance overhead and potential bugs. This consolidation will establish a single, well-tested API interface that all components can rely on.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 17:30:00 | Created | N/A | Proposed | Task file created for API client unification | System |

## Requirements

### Core Requirements
- Create unified `ApiClient` class with consistent interface patterns
- Standardize error handling and response typing across all endpoints
- Implement automatic authentication wrapper integration
- Maintain backward compatibility during migration
- Ensure SCHEMA-002 compliance at the API layer

### Required Constants (Section 2.1.12)
```typescript
const API_REQUEST_TIMEOUT_MS = 30000;
const API_RETRY_ATTEMPTS = 3;
const API_RETRY_DELAY_MS = 1000;
const API_BATCH_REQUEST_LIMIT = 20;
const HTTP_STATUS_CODES = {
  OK: 200,
  CREATED: 201,
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  INTERNAL_SERVER_ERROR: 500
};
```

### DRY Compliance Requirements
- Single implementation of HTTP request logic
- Unified error handling and response transformation
- Centralized authentication integration
- Shared retry and timeout mechanisms
- Common request/response interceptors

### SCHEMA-002 Compliance
- API client must validate schema approval state before operations
- Mutation and query endpoints must enforce approved-only access
- Error responses must clearly indicate schema state violations
- Administrative endpoints (approve/block) must be properly protected

## Implementation Plan

### Phase 1: Create Unified API Client Base
1. **Base ApiClient Class**
   ```typescript
   class ApiClient {
     private baseUrl: string;
     private timeout: number;
     private retryAttempts: number;
     
     constructor(config: ApiClientConfig) {
       this.baseUrl = config.baseUrl || '/api';
       this.timeout = config.timeout || API_REQUEST_TIMEOUT_MS;
       this.retryAttempts = config.retryAttempts || API_RETRY_ATTEMPTS;
     }
     
     async get<T>(endpoint: string, options?: RequestOptions): Promise<ApiResponse<T>>;
     async post<T>(endpoint: string, data?: any, options?: RequestOptions): Promise<ApiResponse<T>>;
     async put<T>(endpoint: string, data?: any, options?: RequestOptions): Promise<ApiResponse<T>>;
     async delete<T>(endpoint: string, options?: RequestOptions): Promise<ApiResponse<T>>;
   }
   ```

2. **Request/Response Types**
   ```typescript
   interface ApiResponse<T = any> {
     success: boolean;
     data?: T;
     error?: string;
     status: number;
     headers?: Record<string, string>;
   }
   
   interface RequestOptions {
     requiresAuth?: boolean;
     timeout?: number;
     retries?: number;
     validateSchema?: boolean;
   }
   
   interface ApiClientConfig {
     baseUrl?: string;
     timeout?: number;
     retryAttempts?: number;
     defaultHeaders?: Record<string, string>;
   }
   ```

### Phase 2: Implement Request Handling
1. **HTTP Request Logic**
   - Implement fetch-based requests with proper error handling
   - Add automatic retry mechanism with exponential backoff
   - Include timeout handling with `API_REQUEST_TIMEOUT_MS`
   - Support request cancellation for component unmounting

2. **Authentication Integration**
   - Integrate with existing [`authenticationWrapper`](../../../src/datafold_node/static-react/src/utils/authenticationWrapper.ts)
   - Automatically apply signing for protected endpoints
   - Handle authentication errors consistently
   - Support both signed and unsigned requests

3. **Error Handling Standardization**
   ```typescript
   class ApiError extends Error {
     constructor(
       message: string,
       public status: number,
       public response?: any,
       public isNetworkError: boolean = false
     ) {
       super(message);
       this.name = 'ApiError';
     }
   }
   
   // Error handling utilities
   function isSchemaStateError(error: ApiError): boolean;
   function isAuthenticationError(error: ApiError): boolean;
   function isNetworkError(error: ApiError): boolean;
   ```

### Phase 3: Create Domain-Specific Clients
1. **Schema API Client**
   - Replace existing [`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) with unified client
   - Implement SCHEMA-002 compliance checks
   - Add schema state validation before operations
   - Include type-safe schema operations

2. **Mutation API Client**
   - Replace existing [`mutationClient.ts`](../../../src/datafold_node/static-react/src/api/mutationClient.ts)
   - Enforce approved schema access for mutations
   - Add range schema validation
   - Implement mutation result typing

3. **Security API Client**
   - Replace existing [`securityClient.ts`](../../../src/datafold_node/static-react/src/api/securityClient.ts)
   - Standardize key management operations
   - Add proper error handling for cryptographic operations
   - Include type-safe key generation and validation

4. **Domain Client Interfaces**
   ```typescript
   interface SchemaApiClient {
     getSchemas(): Promise<ApiResponse<Schema[]>>;
     getSchema(name: string): Promise<ApiResponse<Schema>>;
     approveSchema(name: string): Promise<ApiResponse<void>>;
     blockSchema(name: string): Promise<ApiResponse<void>>;
     getSchemasByState(state: SchemaState): Promise<ApiResponse<Schema[]>>;
   }
   
   interface MutationApiClient {
     executeMutation(mutation: SignedMutation): Promise<ApiResponse<MutationResult>>;
     validateMutation(mutation: Mutation): Promise<ApiResponse<ValidationResult>>;
   }
   ```

### Phase 4: Response Caching and Optimization
1. **Response Caching**
   - Implement in-memory response caching
   - Add cache invalidation strategies
   - Support conditional requests (ETags, Last-Modified)
   - Include cache warming for critical endpoints

2. **Request Optimization**
   - Implement request deduplication
   - Add batch request capabilities
   - Support request prioritization
   - Include background refresh for cached data

### Phase 5: Migration and Integration
1. **Gradual Migration**
   - Replace existing client usage incrementally
   - Maintain backward compatibility during transition
   - Update component imports and usage patterns
   - Verify functionality at each migration step

2. **Component Integration**
   - Update all components to use unified API client
   - Remove direct fetch calls in favor of client methods
   - Standardize error handling across components
   - Update Redux async thunks to use new client

## Verification

### Unit Testing Requirements
- [ ] API client base class tested with mock HTTP responses
- [ ] Error handling tested for all HTTP status codes
- [ ] Authentication integration tested with signed/unsigned requests
- [ ] Retry mechanism tested with network failures
- [ ] Timeout handling tested with delayed responses
- [ ] SCHEMA-002 compliance tested with schema state validation

### Integration Testing Requirements
- [ ] Domain-specific clients tested with real API endpoints
- [ ] Component integration tested with new API client
- [ ] Redux integration tested with unified client
- [ ] Error propagation tested end-to-end
- [ ] Authentication flow tested with API operations

### Performance Requirements
- [ ] Response times maintained or improved compared to existing clients
- [ ] Memory usage optimized with proper cache management
- [ ] Request deduplication reduces redundant API calls
- [ ] Batch operations handle up to `API_BATCH_REQUEST_LIMIT` requests

### Documentation Requirements
- [ ] API client interfaces documented with TypeScript
- [ ] Usage examples provided for each domain client
- [ ] Error handling guide created for component developers
- [ ] Migration guide created for existing API usage

## Files Modified

### Created Files
- `src/datafold_node/static-react/src/api/core/ApiClient.ts`
- `src/datafold_node/static-react/src/api/core/types.ts`
- `src/datafold_node/static-react/src/api/core/errors.ts`
- `src/datafold_node/static-react/src/api/core/cache.ts`
- `src/datafold_node/static-react/src/api/clients/SchemaApiClient.ts`
- `src/datafold_node/static-react/src/api/clients/MutationApiClient.ts`
- `src/datafold_node/static-react/src/api/clients/SecurityApiClient.ts`
- `src/datafold_node/static-react/src/api/index.ts`

### Modified Files
- `src/datafold_node/static-react/src/api/schemaClient.ts` - Deprecated, replaced with unified client
- `src/datafold_node/static-react/src/api/mutationClient.ts` - Deprecated, replaced with unified client
- `src/datafold_node/static-react/src/api/securityClient.ts` - Deprecated, replaced with unified client
- `src/datafold_node/static-react/src/api/endpoints.ts` - Updated for new client structure
- `src/datafold_node/static-react/src/utils/httpClient.ts` - Integrated with unified client

### Component Updates
- `src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx` - Use unified schema client
- `src/datafold_node/static-react/src/components/tabs/MutationTab.jsx` - Use unified mutation client
- `src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx` - Use unified security client
- `src/datafold_node/static-react/src/store/schemaSlice.ts` - Use unified client in async thunks

### Test Files
- `src/datafold_node/static-react/src/api/core/__tests__/ApiClient.test.ts`
- `src/datafold_node/static-react/src/api/core/__tests__/errors.test.ts`
- `src/datafold_node/static-react/src/api/clients/__tests__/SchemaApiClient.test.ts`
- `src/datafold_node/static-react/src/api/clients/__tests__/MutationApiClient.test.ts`
- `src/datafold_node/static-react/src/test/integration/ApiClientIntegration.test.tsx`

## Rollback Plan

If issues arise during API client unification:

1. **Client Isolation**: Temporarily disable unified client and restore original clients
2. **Component Rollback**: Revert components to use original API client imports
3. **Incremental Migration**: Move one domain client at a time back to unified approach
4. **Error Handling Verification**: Ensure error handling remains consistent during rollback
5. **Authentication Preservation**: Maintain authentication flow during rollback process
6. **Performance Monitoring**: Verify API performance is not degraded during rollback