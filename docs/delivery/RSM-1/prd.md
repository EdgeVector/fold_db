# PBI-RSM-1: Redux State Management for Authentication State Synchronization

[View in Backlog](../backlog.md#user-content-RSM-1)

## Overview

This PBI implements Redux Toolkit state management to resolve the AUTH-003 authentication state synchronization issue identified in the complex database dashboard. The current React Context implementation fails to properly propagate authentication state changes across all components, resulting in inconsistent UI states where authentication succeeds but tabs remain locked and warning banners persist.

## Problem Statement

The existing authentication system has a critical state synchronization issue:

- **Authentication Logic Works**: Ed25519 key generation, registration, and validation succeed
- **UI State Propagation Fails**: React Context updates don't consistently reach all consuming components
- **User Impact**: Successful authentication doesn't unlock UI, creating confusion and blocking user workflows
- **Root Cause**: Multiple useState calls, missing memoization, and global auth instance conflicts in [`useAuth.tsx`](../../src/datafold_node/static-react/src/auth/useAuth.tsx)

For a complex database dashboard with multiple tabs, data views, and real-time updates, Redux provides superior:
- **Predictable state flow** for complex authentication states
- **Debugging capabilities** with Redux DevTools for tracing state issues
- **Centralized state management** for dashboard-wide data synchronization
- **Scalability** for future dashboard complexity

## User Stories

**Primary User Story:**
As a developer, I want to implement Redux state management to resolve authentication state synchronization issues in the complex database dashboard.

**Detailed User Stories:**
- As a user, I want successful authentication to immediately unlock all dashboard tabs and remove warning banners
- As a user, I want consistent authentication state across all components without refresh
- As a developer, I want predictable state management for complex dashboard data flows
- As a developer, I want debugging tools to trace authentication state changes
- As a developer, I want scalable state management for growing dashboard complexity

## Technical Approach

### Redux Toolkit Implementation
Replace the current broken React Context with Redux Toolkit:

1. **Authentication Slice**: Centralized auth state with actions for login, logout, key operations
2. **Store Configuration**: Type-safe store setup with middleware and DevTools
3. **Typed Hooks**: Type-safe Redux hooks for components
4. **Component Migration**: Replace useAuth Context calls with Redux selectors/dispatchers

### Migration Strategy
1. **Install Dependencies**: `@reduxjs/toolkit`, `react-redux`
2. **Create Redux Infrastructure**: Store, slices, typed hooks
3. **Gradual Component Migration**: Replace useAuth calls systematically
4. **Remove React Context**: Clean up broken useAuth.tsx implementation
5. **Testing**: Verify AUTH-003 resolution and state synchronization

### Existing Authentication Infrastructure (Preserved)
- ✅ **Ed25519 Operations**: Client-side cryptography working correctly
- ✅ **Backend Integration**: Security routes and signature verification functional
- ✅ **Key Management**: Generation, registration, and validation working
- ✅ **Session Management**: Cleanup and lifecycle management working

## UX/UI Considerations

- **Immediate State Updates**: Authentication changes reflect instantly across all components
- **Visual Feedback**: Clear indication of authentication status in all dashboard areas
- **Tab Synchronization**: All tabs respond immediately to authentication state changes
- **Error Handling**: Consistent error states managed through Redux
- **Performance**: Optimized selectors prevent unnecessary re-renders

## Acceptance Criteria

1. **AUTH-003 Resolution**: Authentication state propagates correctly to all components
2. **Tab Unlocking**: Successful authentication immediately unlocks all dashboard tabs
3. **Banner Removal**: Warning banners disappear immediately upon successful authentication
4. **State Consistency**: All components show consistent authentication state without manual refresh
5. **Redux DevTools**: Full debugging capability for authentication state changes
6. **Type Safety**: Complete TypeScript integration with Redux Toolkit
7. **Performance**: No regression in component render performance
8. **Testing**: Redux state tested with authentication flows

## Dependencies

### External Dependencies
- `@reduxjs/toolkit` - Modern Redux implementation
- `react-redux` - React bindings for Redux
- TypeScript definitions for type safety

### Internal Dependencies
- Existing authentication logic (preserved)
- Ed25519 cryptography operations (unchanged)
- Backend security routes (unchanged)
- React component structure (minimal changes)

## Open Questions

1. **Migration Timeline**: Gradual vs complete replacement of React Context
2. **State Structure**: Optimal Redux state shape for authentication data
3. **Middleware**: Additional middleware needed for async authentication operations
4. **Testing Strategy**: Integration testing approach for Redux authentication flows

## Related Tasks

See [Tasks for PBI RSM-1](./tasks.md) for detailed implementation tasks.