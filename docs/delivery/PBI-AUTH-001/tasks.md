# Tasks for PBI AUTH-001: Authenticated Request Signing and Tab Unlocking

This document lists all tasks associated with PBI AUTH-001.

**Parent PBI**: [PBI AUTH-001: Authenticated Request Signing and Tab Unlocking](./prd.md)

## Task Summary

**Note**: Analysis shows most functionality already exists. Reduced from 4 tasks to 3 tasks with significantly reduced scope.

| Task ID | Name | Status | Dependencies | Scope | Description |
| :------ | :--------------------------------------- | :------- | :----------- | :------- | :--------------------------------- |
| AUTH-001 | [Create Global Authentication Context](./PBI-AUTH-001-AUTH-001.md) | **Minimal** | **None** | **Reduced** | Simple context wrapper around existing `useKeyAuthentication` for global access |
| AUTH-002 | [Add Automatic Request Signing](./PBI-AUTH-001-AUTH-002.md) | **Active** | **AUTH-001** | **Core** | Create `signedRequest()` wrapper using global context from AUTH-001 |
| ~~AUTH-003~~ | ~~[Implement Tab Locking/Unlocking UI](./PBI-AUTH-001-AUTH-003.md)~~ | **Eliminated** | **N/A** | **N/A** | ~~Complete UI already implemented in current codebase~~ |
| AUTH-004 | [Integration Testing](./PBI-AUTH-001-AUTH-004.md) | **Unchanged** | **AUTH-001 + AUTH-002** | **Standard** | Test integration between global context and API signing |

## Implementation Order (Dependency Chain)

```
AUTH-001 (Global Context)
    ↓
AUTH-002 (API Signing Wrapper)
    ↓
AUTH-004 (Integration Testing)
```

**Critical Path**: AUTH-001 → AUTH-002 → AUTH-004

- **Phase 1**: AUTH-001 must be completed first (no dependencies)
- **Phase 2**: AUTH-002 requires AUTH-001's global context for non-hook access to authentication state
- **Phase 3**: AUTH-004 tests the integration between AUTH-001 and AUTH-002

## Why AUTH-002 Depends on AUTH-001

The key technical reason: AUTH-002's `signedRequest()` wrapper function cannot use React hooks directly (like [`useKeyAuthentication()`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts)) because:

1. **React Hook Rules**: Hooks can only be called inside React components, not utility functions
2. **API Client Architecture**: The `signedRequest()` wrapper needs to be a pure JavaScript function that API clients can use
3. **Global Access Required**: AUTH-001 provides `getAuthContextInstance()` for non-hook access to authentication state

Without AUTH-001's global context, AUTH-002 cannot access authentication state outside of React components.

## Implementation Scope Analysis

### 🟢 AUTH-001: **MINIMAL IMPLEMENTATION**
- **Existing**: [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) has all logic
- **Needed**: Simple React context wrapper for global access
- **Files**: 3 small files (context, hook, types)

### 🔴 AUTH-002: **MAIN IMPLEMENTATION WORK**
- **Existing**: [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) complete
- **Needed**: Modify [`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) for conditional signing
- **Files**: Primarily 1 file modification

### ❌ AUTH-003: **ELIMINATED - ALREADY COMPLETE**
**All functionality already exists**:
- ✅ Tab conditional rendering: [`App.jsx:80-85`](../../../src/datafold_node/static-react/src/App.jsx:80)
- ✅ Authentication controls: [`KeyManagementTab.jsx:210-292`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx:210)
- ✅ Visual feedback: [`App.jsx:178-259`](../../../src/datafold_node/static-react/src/App.jsx:178) (🔒/✓ icons)
- ✅ Authentication status: [`KeyManagementTab.jsx:199-201`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx:199)

### 🟡 AUTH-004: **UNCHANGED SCOPE**
- Standard integration testing for completed features

## Existing Infrastructure (Leveraged)

- ✅ **Authentication Hook**: [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Memory-only private key validation and auth state
- ✅ **Request Signing**: [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) - Ed25519 signing implementation
- ✅ **API Client**: [`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) - Ready for signing integration
- ✅ **Key Management UI**: [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) - User interface for key operations
- ✅ **Ed25519 Utils**: Cryptographic operations and utilities
- ✅ **Security Client**: Backend verification infrastructure

## Security Benefits of Memory-Only Approach

- **No Persistent Attack Surface**: Private keys never touch disk, localStorage, or sessionStorage
- **Process Isolation**: Keys destroyed automatically when tab/browser closes
- **Proven Security Model**: Leverages existing secure implementation without introducing new storage vulnerabilities
- **Reduced Complexity**: No encryption/decryption overhead or key derivation dependencies