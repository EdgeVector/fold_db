# AUTH-004: Integration Testing (SIMPLIFIED SCOPE)

[Back to task list](./tasks.md)

## Dependencies

**🔒 Requires AUTH-001 AND AUTH-002** - Cannot start until both are complete

- **Prerequisites**:
  - AUTH-001 (Global Authentication Context) - provides global access to authentication state
  - AUTH-002 (Authentication Wrapper) - provides the `signedRequest()` functionality to test
- **Reason**: Integration testing requires both the global context and the API signing wrapper to be implemented before testing their interaction
- **Can work in parallel**: No, must wait for both AUTH-001 and AUTH-002 to complete
- **Blocks**: None (final task in dependency chain)

## Description

**Simplified scope** - Extend existing comprehensive testing in [`KeyLifecycle.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx) to cover the NEW functionality from AUTH-001 (global context) and AUTH-002 (API signing integration). The existing 268-line test already covers most authentication scenarios robustly.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-22 11:53:00 | Created | N/A | Proposed | Integration testing for complete authentication flow | User |
| 2025-06-22 12:00:00 | Updated | Proposed | Proposed | Updated to align with memory-only approach (Option 1) | User |
| 2025-06-22 12:51:00 | Simplified | Proposed | Proposed | Scope reduced to extend existing comprehensive tests | User |
| 2025-06-22 16:18:00 | Status Update | Proposed | InProgress | Started implementation of integration testing | User |
| 2025-06-22 16:28:00 | Completed | InProgress | Review | All integration tests passing (23/23) - KeyLifecycle, AuthenticationContext, AuthenticationWrapper | User |
| 2025-06-22 18:02:00 | Status Change | Review | Complete | Integration testing verified complete - all test suites passing and functionality confirmed | AI Agent |

## Current Testing Infrastructure

### ✅ **ALREADY COMPREHENSIVE** - Build On This

- **Framework**: [`vitest.config.js`](../../../src/datafold_node/static-react/vitest.config.js) with Vitest + React Testing Library
- **Existing Tests**: [`KeyLifecycle.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx) (268 lines) covers:
  - Complete key generation and registration workflow
  - Public/private key validation and import
  - Authentication state management
  - API request mocking (`/api/security/system-key`, `/api/mutation`)
  - Error handling and security warnings
  - Full authentication lifecycle
- **Mocking Patterns**: Comprehensive mocks for `@noble/ed25519`, `fetch`, `clipboard`

### 🔍 **TESTING GAPS** - Only Test New Functionality

1. **Global authentication context** integration from AUTH-001
2. **API request signing** with existing [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) from AUTH-002

## Requirements (Simplified)

1. **Extend Existing Framework**: Build on Vitest + React Testing Library configuration
2. **Focus on New Integration**: Only test global context + API signing functionality
3. **Leverage Existing Patterns**: Use established mocking and testing approaches
4. **Minimal New Tests**: Add targeted test cases, not complete new test suites

## Implementation Plan (Realistic)

1. **Extend Existing KeyLifecycle.test.jsx**:
   - Add test cases for global authentication context access
   - Test API request signing integration using existing mocking patterns
   - Validate interaction between existing `useKeyAuthentication` and new global context
   - Keep existing comprehensive test coverage intact

2. **New Test Cases for AUTH-001 Integration**:
   ```javascript
   it('provides global authentication state through context', async () => {
     // Test global context wrapper provides same state as useKeyAuthentication
   })
   ```

3. **New Test Cases for AUTH-002 Integration**:
   ```javascript
   it('automatically signs API requests when authenticated', async () => {
     // Test API calls use createSignedMessage() when authenticated
     // Mock signed request headers and verify backend integration
   })
   
   it('makes unsigned requests when not authenticated', async () => {
     // Test backward compatibility for unauthenticated requests
   })
   ```

4. **Leverage Existing Infrastructure**:
   - Use established `fetch` mocking patterns
   - Extend existing `@noble/ed25519` mocks
   - Build on current authentication state testing
   - Maintain existing security validation patterns

## Verification (Focused)

- [x] Global authentication context provides access to existing authentication state
- [x] API requests automatically signed when authenticated using [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28)
- [x] Unsigned requests work for unauthenticated users (backward compatibility)
- [x] Integration between existing [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) and new global context
- [x] Existing [`KeyLifecycle.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx) continues to pass
- [x] New test cases cover only the integration gaps

## Files Modified (Minimal)

- `src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx` (extend existing)
- `src/datafold_node/static-react/src/test/integration/GlobalAuthContext.test.jsx` (new, focused)
- `src/datafold_node/static-react/src/test/integration/SignedAPIRequests.test.jsx` (new, focused)

## Integration Points (Focused)

**Leverages Existing Infrastructure**:
- [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Already tested comprehensively
- [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) - Function exists, needs integration testing
- [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) - UI already tested
- [`KeyLifecycle.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx) - 268 lines of robust testing

**Tests New Integration Only**:
- Global authentication context wrapper (AUTH-001)
- API request signing integration (AUTH-002)
- Interaction between existing and new components

## Test Plan (Practical)

### Focused Test Scenarios

1. **Global Context Integration**:
   - Verify global context exposes same authentication state as existing hook
   - Test context provides access to authentication data for API signing
   - Ensure no breaking changes to existing authentication flow

2. **API Signing Integration**:
   - Test authenticated API requests automatically use [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28)
   - Verify `X-Signed-Request: true` header added for signed requests
   - Test unauthenticated requests remain unsigned (backward compatibility)
   - Mock backend verification response for signed requests

3. **Integration Validation**:
   - Existing [`KeyLifecycle.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/KeyLifecycle.test.jsx) continues to pass
   - New global context doesn't interfere with existing authentication patterns
   - API signing works with existing key management and validation

### Success Criteria (Realistic)
- Existing comprehensive tests continue to pass without modification
- New integration functionality works as expected with minimal additional test code
- Global context provides seamless access to authentication state for API signing
- Backward compatibility maintained for all existing functionality
- Test approach matches established Vitest + React Testing Library patterns