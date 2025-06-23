# RSM-1-3 Remove React Context implementation

[Back to task list](./tasks.md)

## Description

Clean up the broken React Context implementation after Redux migration is complete, removing useAuth.tsx and all related Context code that caused AUTH-003 state synchronization issues.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-23 09:47:00 | Created | N/A | Proposed | Simplified cleanup task after Redux migration | User |

## Requirements

- Remove useAuth.tsx file containing broken Context implementation
- Clean up any remaining useAuth imports in components
- Verify no broken references remain in codebase
- Maintain clean codebase without duplicate state management

## Implementation Plan

### Development Instructions
- **Testing Server**: Use `./run_http_server.sh` and test at `http://localhost:9001/`
- **Code Changes**: Read existing code first, make minimal code changes
- **Quality**: Simplify, remove duplicates, make tests pass, fix linting issues

### Steps
1. **Verify Migration Complete**
   - Confirm RSM-1-2 components successfully migrated to Redux
   - Test all authentication flows work with Redux

2. **Remove Context Implementation**
   - Remove [`src/auth/useAuth.tsx`](../../src/datafold_node/static-react/src/auth/useAuth.tsx)
   - File contains AUTH-003 problematic multiple useState calls (lines 27-33)
   - File contains broken KeyAuthenticationState, globalAuthInstance, AuthenticationProvider

3. **Clean Up References**
   - Search for any remaining useAuth imports
   - Remove any leftover AuthProvider references
   - Verify no broken imports or references

4. **Final Verification**
   - Test application works without Context implementation
   - Confirm authentication flows use only Redux
   - Verify no console errors about missing Context

## Verification

- [ ] useAuth.tsx file completely removed
- [ ] No remaining useAuth imports in any components
- [ ] No AuthProvider references remain
- [ ] Application starts without errors using `./run_http_server.sh`
- [ ] All authentication flows work correctly at `http://localhost:9001/`
- [ ] No console errors about missing Context
- [ ] TypeScript compilation passes
- [ ] Linting passes with no unused imports

## Files Modified

- `src/auth/useAuth.tsx` - File removed
- Other component files - Remove any unused useAuth imports if found