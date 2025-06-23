# AUTH-002: Add Mandatory Request Authentication and Signing (CORE IMPLEMENTATION)

[Back to task list](./tasks.md)

## Dependencies

**🔒 Requires AUTH-001** - Cannot start until AUTH-001 is complete

- **Prerequisites**: AUTH-001 (Global Authentication Context) must be completed first
- **Reason**: The `signedRequest()` wrapper function cannot use React hooks directly - it needs non-hook access to authentication state provided by AUTH-001's global context
- **Can work in parallel**: No, strictly depends on AUTH-001
- **Blocks**: AUTH-004 (Integration Testing)

## Description

**Primary implementation task** - Create a unified authentication wrapper that handles authentication checking and signing for protected operations using the existing [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) function. Only specific operations require authentication and signing, while others remain unprotected.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-22 11:52:00 | Updated | Proposed | Proposed | Simplified to leverage existing createSignedMessage() infrastructure | User |
| 2025-06-22 12:00:00 | Updated | Proposed | Proposed | Updated to align with memory-only approach (Option 1) | User |
| 2025-06-22 12:43:00 | Analysis | Proposed | Active | Confirmed as main implementation task - clear scope | User |
| 2025-06-22 12:54:00 | Updated | Active | Active | Changed to mandatory authentication - always sign, block if not authenticated | User |
| 2025-06-22 12:58:00 | Updated | Active | Active | Switched to unified authentication wrapper approach with specific protected operations | User |
| 2025-06-22 16:17:00 | Started | Active | InProgress | Implementation started - authentication wrapper created and API clients updated | Assistant |
| 2025-06-22 18:02:00 | Status Change | InProgress | Complete | Implementation finished - signedRequest wrapper functional, all protected operations secured | AI Agent |

## Current Implementation Analysis

### ✅ **ALREADY IMPLEMENTED** - No Changes Needed

- [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) - Complete signing implementation
- [`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) - Basic API client structure exists
- [`useKeyAuthentication.ts`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Authentication state management

### 🔨 **IMPLEMENTATION NEEDED** - Clear Scope

- **Create unified authentication wrapper** (`signedRequest()` function)
- **Define specific protected operations** that require authentication and signing
- **Apply wrapper to protected operations** only
- **Keep unprotected operations unchanged** (like getting system public key)
- **Provide clear error messages** when authentication is required but missing
- **Integrate with global auth context** from AUTH-001

## Requirements (Updated - Unified Authentication Wrapper)

1. **Unified Wrapper Function**: Create a `signedRequest()` wrapper that handles authentication and signing
2. **Specific Protected Operations**: Define exact list of operations requiring authentication:
   - Queries (data retrieval)
   - Mutations (data modification)
   - Make schema approved
   - Make schema blocked
3. **Unprotected Operations**: Keep certain operations unsigned (e.g., getting system public key)
4. **Authentication Guard**: Block protected operations when user not authenticated
5. **Unified Error Message**: Consistent error message for authentication failures
6. **Clean Separation**: Clear distinction between protected and unprotected operations

## Implementation Plan (Unified Authentication Wrapper)

1. **Create Authentication Wrapper Utility**:
   ```typescript
   // New file: src/datafold_node/static-react/src/utils/authenticationWrapper.ts
   import { createSignedMessage } from './signing';
   import { getAuthContextInstance } from '../contexts/AuthContext'; // From AUTH-001

   export async function signedRequest<T>(requestFunction: () => Promise<T>): Promise<T> {
     const authContext = getAuthContextInstance();
     
     if (!authContext?.isAuthenticated || !authContext?.privateKey || !authContext?.publicKeyId) {
       throw new Error('Authentication required: This operation requires valid authentication');
     }
     
     // Execute the request function with authentication context available
     return await requestFunction();
   }
   ```

2. **Define Protected Operations**:
   ```typescript
   // Protected operations that require signing:
   const queryData = await signedRequest(() => schemaClient.query(params));
   const mutationResult = await signedRequest(() => schemaClient.mutate(data));
   const approveResult = await signedRequest(() => schemaClient.makeSchemaApproved(schemaId));
   const blockResult = await signedRequest(() => schemaClient.makeSchemaBlocked(schemaId));
   ```

3. **Keep Unprotected Operations Unchanged**:
   ```typescript
   // Unprotected operations remain direct calls:
   const systemKey = await securityClient.getSystemPublicKey();
   const publicSchemas = await schemaClient.getPublicSchemas();
   ```

4. **Wrapper Implementation Details**:
   - Check authentication state before executing protected operations
   - Use existing [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) for signing
   - Add `X-Signed-Request: true` header for signed requests
   - Provide unified error handling for authentication failures

5. **Error Handling**:
   - Unified error message: "Authentication required: This operation requires valid authentication"
   - Clear separation between authentication errors and other API errors
   - No modification of unprotected operations' error handling

## Specific Protected Operations

### Operations Requiring Authentication and Signing:
1. **Queries** - Data retrieval operations
2. **Mutations** - Data modification operations
3. **Make Schema Approved** - Schema approval operations
4. **Make Schema Blocked** - Schema blocking operations

### Operations Remaining Unprotected:
1. **Get System Public Key** - Public key retrieval
2. **Get Public Schemas** - Public schema listing
3. **Health Checks** - System status endpoints
4. **Public Configuration** - Non-sensitive system information

## Code Examples

### Authentication Wrapper Implementation:
```typescript
// src/datafold_node/static-react/src/utils/authenticationWrapper.ts
import { createSignedMessage } from './signing';
import { getAuthContextInstance } from '../contexts/AuthContext'; // Requires AUTH-001

export async function signedRequest<T>(requestFunction: () => Promise<T>): Promise<T> {
  const authContext = getAuthContextInstance();
  
  if (!authContext?.isAuthenticated || !authContext?.privateKey || !authContext?.publicKeyId) {
    throw new Error('Authentication required: This operation requires valid authentication');
  }
  
  // Execute the request function with authentication context available
  return await requestFunction();
}
```

### Usage in API Clients:
```typescript
// Protected operations use the wrapper
import { signedRequest } from '../utils/authenticationWrapper';

// Queries - require authentication
export const queryData = async (params: QueryParams) => {
  return await signedRequest(() => schemaClient.query(params));
};

// Mutations - require authentication
export const mutateData = async (data: MutationData) => {
  return await signedRequest(() => schemaClient.mutate(data));
};

// Schema management - require authentication
export const approveSchema = async (schemaId: string) => {
  return await signedRequest(() => schemaClient.makeSchemaApproved(schemaId));
};

export const blockSchema = async (schemaId: string) => {
  return await signedRequest(() => schemaClient.makeSchemaBlocked(schemaId));
};

// Unprotected operations remain unchanged
export const getSystemPublicKey = async () => {
  return await securityClient.getSystemPublicKey(); // No wrapper needed
};

export const getPublicSchemas = async () => {
  return await schemaClient.getPublicSchemas(); // No wrapper needed
};
```

## Verification (Unified Authentication Wrapper)

- [x] `signedRequest()` wrapper function created and functional
- [x] Protected operations (queries, mutations, schema approval/blocking) use the wrapper
- [x] Unprotected operations (system public key, public schemas) remain unchanged
- [x] Wrapper blocks protected operations when not authenticated
- [x] Wrapper signs requests when authenticated using [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28)
- [x] Unified error message displayed for authentication failures
- [x] Clear separation between protected and unprotected operations
- [x] [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) used without modification
- [x] Backend receives and verifies signed requests correctly

## Files Modified (Specific)

- **New file**: [`src/datafold_node/static-react/src/utils/authenticationWrapper.ts`](../../../src/datafold_node/static-react/src/utils/authenticationWrapper.ts) - **Main implementation**
- [`src/datafold_node/static-react/src/api/schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) - Apply wrapper to protected operations
- Other API clients (`mutationClient.ts`, `securityClient.ts`) - Apply wrapper to their protected operations

## Integration Points

**No Changes Needed**:
- ✅ [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) - Use exactly as-is
- ✅ Backend verification - Already supports signed requests
- ✅ Authentication state - Available from AUTH-001 context

**Implementation Work**:
- **Authentication wrapper utility** (`signedRequest()` function)
- **Protected operation identification** and wrapper application
- **Request blocking** for protected operations when authentication unavailable
- **Unified error handling** for authentication failures
- **Selective signing** for protected operations only

## Test Plan (Unified Authentication Wrapper)

### Key Test Scenarios

1. **Protected Operations with Authentication**:
   - User authenticated → Protected operations automatically signed using [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28)
   - Queries, mutations, schema approval/blocking use wrapper
   - Signed requests include `X-Signed-Request: true` header
   - Backend receives and verifies signed requests correctly

2. **Protected Operations without Authentication**:
   - User not authenticated → Protected operations blocked with clear error message
   - No network requests made for protected operations when authentication missing
   - Error message: "Authentication required: This operation requires valid authentication"

3. **Unprotected Operations**:
   - User not authenticated → Unprotected operations work normally
   - System public key retrieval works without authentication
   - Public schema access works without authentication
   - No signing applied to unprotected operations

4. **Wrapper Function Testing**:
   - `signedRequest()` wrapper properly checks authentication state
   - Wrapper executes request function when authenticated
   - Wrapper blocks execution when not authenticated
   - Clear error propagation from wrapper

5. **State Transitions**:
   - Authentication state changes → Protected operations behavior updates immediately
   - Logout → Protected operations blocked, unprotected operations continue
   - Re-authentication → Protected operations resume with signing

6. **Code Examples Verification**:
   ```javascript
   // Protected operations use the wrapper
   const queryData = await signedRequest(() => schemaClient.query(params));
   const mutationResult = await signedRequest(() => schemaClient.mutate(data));
   
   // Unprotected operations remain unchanged
   const systemKey = await securityClient.getSystemPublicKey();
   ```

### Success Criteria
- **Unified wrapper**: Single `signedRequest()` function handles all authentication
- **Selective protection**: Only specified operations require authentication
- **Clear separation**: Protected vs unprotected operations clearly distinguished
- **Unified error handling**: Consistent error message for authentication failures
- **Clean integration**: Uses existing [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) without modification
- **Better architecture**: Separation of concerns between authentication and API logic