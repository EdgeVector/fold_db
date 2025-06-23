# RSM-1-1 Setup Redux infrastructure

[Back to task list](./tasks.md)

## Description

Complete Redux infrastructure setup including dependencies, store configuration, authentication slice, typed hooks, and DevTools to replace the broken React Context causing AUTH-003 state synchronization issues.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-23 09:47:00 | Created | N/A | Proposed | Simplified task combining Redux infrastructure setup | User |

## Requirements

- Install Redux Toolkit and React Redux dependencies
- Create TypeScript-configured store with middleware
- Implement authentication slice based on existing useAuth state
- Create typed hooks (useAppDispatch, useAppSelector)
- Enable Redux DevTools for debugging AUTH-003 issues

## Implementation Plan

### Development Instructions
- **Testing Server**: Use `./run_http_server.sh` and test at `http://localhost:9001/`
- **Code Changes**: Read existing code first, make minimal code changes
- **Quality**: Simplify, remove duplicates, make tests pass, fix linting issues

### Steps
1. **Install Dependencies**
   - Review existing [`package.json`](../../src/datafold_node/static-react/package.json) dependencies
   - Install: `npm install @reduxjs/toolkit react-redux`
   - Install types: `npm install --save-dev @types/react-redux`

2. **Create Store Infrastructure**
   - Create `src/datafold_node/static-react/src/store/store.ts`
   - Follow existing TypeScript patterns from [`src/types/cryptography.ts`](../../src/datafold_node/static-react/src/types/cryptography.ts)
   - Configure with Redux DevTools for development

3. **Implement Authentication Slice**
   - Read [`src/auth/useAuth.tsx`](../../src/datafold_node/static-react/src/auth/useAuth.tsx) thoroughly
   - Create `src/store/authSlice.ts` based on KeyAuthenticationState interface (lines 6-17)
   - Replace problematic multiple useState calls (lines 27-33) with Redux state
   - Include: validatePrivateKey, clearAuthentication, refreshSystemKey actions

4. **Create Typed Hooks**
   - Create `src/store/hooks.ts`
   - Study existing hook patterns from [`useKeyGeneration.ts`](../../src/datafold_node/static-react/src/hooks/useKeyGeneration.ts)
   - Export useAppDispatch and useAppSelector with proper typing

## Verification

- [ ] Dependencies installed successfully in package.json
- [ ] Store configures with TypeScript support
- [ ] Authentication slice implements all current useAuth functionality
- [ ] Typed hooks provide IntelliSense and type safety
- [ ] Redux DevTools enabled in development
- [ ] Application starts without errors using `./run_http_server.sh`
- [ ] No console errors at `http://localhost:9001/`
- [ ] All linting passes

## Files Modified

- `package.json` - Add Redux dependencies
- `src/store/store.ts` - New Redux store configuration
- `src/store/authSlice.ts` - New authentication Redux slice
- `src/store/hooks.ts` - New typed Redux hooks