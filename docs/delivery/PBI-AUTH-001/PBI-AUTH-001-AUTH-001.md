# AUTH-001: Create Global Authentication Context (REDUCED SCOPE)

[Back to task list](./tasks.md)

## Dependencies

**❌ No Dependencies** - This task can be started immediately

- **Prerequisites**: None
- **Can work in parallel**: Yes, independent task
- **Blocks**: AUTH-002 (Authentication Wrapper) and AUTH-004 (Integration Testing)

## Description

Create a lightweight React context wrapper around the existing [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) hook to provide global access for API request signing. **Minimal scope** - the authentication logic is already robust and complete.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-22 11:52:00 | Updated | Proposed | Proposed | Simplified to leverage existing useKeyAuthentication infrastructure | User |
| 2025-06-22 12:00:00 | Updated | Proposed | Proposed | Updated to use memory-only approach (Option 1) removing encrypted storage | User |
| 2025-06-22 12:42:00 | Analysis | Proposed | Reduced | Scope reduced - authentication logic already complete | User |
| 2025-06-22 16:09:00 | Status Change | Proposed | InProgress | Started implementation of global authentication context | AI Agent |
| 2025-06-22 16:14:00 | Status Change | InProgress | Review | Implementation complete - global context working, all tests pass | AI Agent |
| 2025-06-22 18:02:00 | Status Change | Review | Complete | Task verified complete - all requirements fulfilled and tested | AI Agent |

## Current Implementation Analysis

### ✅ **ALREADY IMPLEMENTED** - No Changes Needed

- [`useKeyAuthentication.ts`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Complete authentication state management
- [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) - Full authentication UI and controls
- [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx) - Tab-level authentication integration

### 🔍 **MINIMAL SCOPE** - Only Global Access Needed

The **only** gap is global access to authentication state for API request signing (AUTH-002).

## Requirements (Reduced)

1. **Simple Context Wrapper**: Wrap existing [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) for global access
2. **No Logic Changes**: Keep all existing authentication logic unchanged
3. **Non-Hook Access**: Provide `getAuthContextInstance()` function for AUTH-002's `signedRequest()` wrapper
4. **API Integration**: Enable access for automatic request signing (AUTH-002)
5. **Zero Breaking Changes**: Maintain full compatibility with existing [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx)

## Implementation Plan (Minimal)

1. **Create Lightweight AuthContext**:
   - Create simple React context that exposes existing `useKeyAuthentication` hook globally
   - No changes to authentication logic - pure wrapper
   - Maintain 100% compatibility with existing implementation

2. **Add Global Access Hook**:
   - Create `useAuth()` hook that provides global access to authentication state
   - Expose `isAuthenticated`, `validatePrivateKey`, and any needed signing data
   - No new authentication logic - just global access

3. **Non-Hook Access for AUTH-002**:
   - Create `getAuthContextInstance()` function that returns current authentication state
   - Enable AUTH-002's `signedRequest()` wrapper to access authentication without React hooks
   - Essential for utility functions that can't use hooks directly

4. **Integration for API Signing**:
   - Ensure private key and authentication state accessible for request signing
   - Support AUTH-002 requirement for automatic request signing
   - Zero impact on existing UI components

## Verification (Minimal)

- [x] Simple AuthProvider wraps app without breaking existing functionality
- [x] [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) works unchanged
- [x] Global `useAuth()` hook provides authentication state for API clients
- [x] No changes to authentication logic or security model

## Files Modified (Minimal)

- [x] `src/datafold_node/static-react/src/context/AuthenticationContext.tsx` (new - simple wrapper)
- [x] `src/datafold_node/static-react/src/hooks/useAuth.ts` (new - global access hook)
- [x] `src/datafold_node/static-react/src/App.jsx` (modified - add AuthProvider wrapper)
- [x] `src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx` (modified - use global context)

## Integration Points

**No Changes to Existing Code**:
- ✅ [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Used as-is
- ✅ [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) - No modifications
- ✅ All existing authentication logic and security

**Minimal New Additions**:
- Simple context wrapper for global access
- Hook for API clients to check authentication state
- Zero breaking changes to existing implementation

## Test Plan (Minimal)

### Key Test Scenarios

1. **Zero Impact Integration**:
   - Existing authentication functionality works unchanged
   - [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) operates normally
   - No regression in existing features

2. **Global Access**:
   - API clients can access authentication state via `useAuth()`
   - Request signing can access private key when authenticated
   - Authentication state updates globally when changed in KeyManagementTab

### Success Criteria
- **Zero breaking changes** to existing authentication functionality
- **Minimal implementation** - pure wrapper around existing logic
- **Global access** for API request signing (AUTH-002)
- **Perfect compatibility** with current user experience