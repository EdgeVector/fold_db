# RSM-1-2 Migrate components to Redux

[Back to task list](./tasks.md)

## Description

Replace React Context usage in App.jsx and KeyManagementTab.jsx with Redux state management to resolve AUTH-003 authentication state synchronization issues.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-23 09:47:00 | Created | N/A | Proposed | Simplified task combining component migration | User |

## Requirements

- Replace AuthenticationProvider with Redux Provider in App.jsx
- Update useAuth calls to use Redux hooks in both components
- Maintain all existing UI logic and authentication flows
- Ensure immediate state updates resolve AUTH-003 issues

## Implementation Plan

### Development Instructions
- **Testing Server**: Use `./run_http_server.sh` and test at `http://localhost:9001/`
- **Code Changes**: Read existing code first, make minimal code changes
- **Quality**: Simplify, remove duplicates, make tests pass, fix linting issues

### Steps
1. **Migrate App.jsx**
   - Read [`src/App.jsx`](../../src/datafold_node/static-react/src/App.jsx) thoroughly
   - Replace AuthenticationProvider with Redux Provider (lines 284-286)
   - Update useAuth calls to Redux hooks (lines 15, 22)
   - Test authentication warning banner updates (lines 149-168)
   - Test tab locking/unlocking logic (lines 86-88, 175-253)

2. **Migrate KeyManagementTab.jsx**
   - Read [`src/components/tabs/KeyManagementTab.jsx`](../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) thoroughly
   - Replace useAuth imports with Redux hooks (lines 6, 13)
   - Update authentication state access (lines 22, 40, 54)
   - Update validatePrivateKey calls (lines 117, 195)
   - Update clearAuthentication calls (lines 29, 162)
   - Update refreshSystemKey calls (lines 65, 183)

3. **Test State Synchronization**
   - Verify warning banners disappear immediately after authentication
   - Verify all tabs unlock immediately after authentication
   - Confirm no manual refresh required for UI updates

## Verification

- [ ] Redux Provider wraps application correctly
- [ ] Authentication state accessed via Redux selectors in both components
- [ ] Warning banners respond immediately to auth state changes
- [ ] Tab locking/unlocking works with Redux state
- [ ] Key management authentication flows work correctly
- [ ] Application starts without errors using `./run_http_server.sh`
- [ ] All authentication operations work at `http://localhost:9001/`
- [ ] No console errors during authentication flows
- [ ] Linting passes for updated components

## Files Modified

- `src/App.jsx` - Replace Context with Redux Provider and state access
- `src/components/tabs/KeyManagementTab.jsx` - Replace useAuth with Redux hooks