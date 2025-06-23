# RSM-1-4 Test and verify AUTH-003 resolution

[Back to task list](./tasks.md)

## Description

Comprehensive testing of Redux authentication flows and end-to-end verification that AUTH-003 authentication state synchronization issue is completely resolved.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-23 09:47:00 | Created | N/A | Proposed | Simplified task combining testing and E2E verification | User |

## Requirements

- Test Redux authentication flows and state synchronization
- Verify AUTH-003 state propagation issue is completely resolved
- Confirm Redux DevTools provide proper debugging capabilities
- Validate all PBI Conditions of Satisfaction are met

## Implementation Plan

### Development Instructions
- **Testing Server**: Use `./run_http_server.sh` and test at `http://localhost:9001/`
- **Code Changes**: Read existing code first, make minimal code changes
- **Quality**: Simplify, remove duplicates, make tests pass, fix linting issues

### Steps
1. **Unit and Integration Testing**
   - Study existing test setup from [`package.json`](../../src/datafold_node/static-react/package.json) lines 10-15
   - Create tests for authentication Redux slice actions and state updates
   - Test component integration with Redux state
   - Test async authentication operations with [`getSystemPublicKey`](../../src/datafold_node/static-react/src/api/securityClient.ts)
   - Run tests using `npm test`

2. **Redux DevTools Verification**
   - Verify DevTools enabled in development mode
   - Test time-travel debugging for authentication state
   - Confirm meaningful action names for debugging clarity

3. **E2E AUTH-003 Resolution Test**
   - Start application: `./run_http_server.sh` → `http://localhost:9001/`
   - Verify warning banners displayed (unauthenticated state)
   - Check [`App.jsx`](../../src/datafold_node/static-react/src/App.jsx) lines 149-168: "Authentication Required" banner visible
   - Verify tabs locked with 🔒 icons (lines 175-253)
   - Generate Ed25519 key pair via [`KeyManagementTab.jsx`](../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx)
   - Register key with backend
   - **Critical Test**: Verify immediate UI state change:
     - Warning banners disappear instantly (no React Context delay)
     - All tabs unlock immediately (proper Redux state propagation)
     - No manual refresh required (immediate re-render)
   - Verify Redux DevTools show correct state transitions
   - Test logout and re-authentication flows
   - Confirm state consistency across all components

## Verification

- [ ] Authentication slice tests created and passing
- [ ] Component integration tests with Redux passing
- [ ] All existing tests continue to pass (`npm test`)
- [ ] Redux DevTools enabled and functional
- [ ] AUTH-003 state synchronization issue completely resolved
- [ ] Warning banners disappear immediately after successful authentication
- [ ] All dashboard tabs unlock immediately after authentication
- [ ] No manual refresh required for UI state updates
- [ ] Redux DevTools show proper authentication state transitions
- [ ] Authentication state consistent across all components
- [ ] Application works correctly using `./run_http_server.sh` at `http://localhost:9001/`
- [ ] All PBI Conditions of Satisfaction verified and met
- [ ] No console errors during complete authentication flow

## Files Modified

- Test files for Redux authentication functionality
- Update existing tests for Redux integration if needed
- Verification documentation of AUTH-003 resolution