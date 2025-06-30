# Tasks for PBI UTS-1: Type Safety Implementation for UI Components

This document lists all tasks associated with PBI UTS-1.

**Parent PBI**: [PBI UTS-1: Type Safety Implementation for UI Components](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UTS-1-1 | [Audit existing TypeScript implementation and plan component migration](./UTS-1-1.md) | **In Progress** | **COMPLETED:** Redux TypeScript fully implemented in [`store/`](../../../src/datafold_node/static-react/src/store/), [`types/`](../../../src/datafold_node/static-react/src/types/) exists with api.ts, cryptography.ts, schema.ts. **REMAINING:** Component .jsx to .tsx migration |
| UTS-1-2 | [Define component prop interfaces and API response types](./UTS-1-2.md) | **In Progress** | **PARTIALLY DONE:** API types exist in [`types/api.ts`](../../../src/datafold_node/static-react/src/types/api.ts), cryptography types in [`types/cryptography.ts`](../../../src/datafold_node/static-react/src/types/cryptography.ts), schema types in [`types/schema.ts`](../../../src/datafold_node/static-react/src/types/schema.ts). **REMAINING:** Component prop interfaces |
| UTS-1-3 | [Integrate components with existing typed Redux store](./UTS-1-3.md) | **In Progress** | **PARTIALLY DONE:** [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) uses typed hooks [`useAppSelector, useAppDispatch`](../../../src/datafold_node/static-react/src/store/hooks.ts). **REMAINING:** Complete component integration |
| UTS-1-4 | [Convert component props to TypeScript interfaces](./UTS-1-4.md) | Proposed | Add TypeScript prop interfaces for all React components |
| UTS-1-5 | [Add type safety to custom hooks and utilities](./UTS-1-5.md) | **In Progress** | **PARTIALLY DONE:** [`useKeyGeneration.ts`](../../../src/datafold_node/static-react/src/hooks/useKeyGeneration.ts), [`useKeyLifecycle.ts`](../../../src/datafold_node/static-react/src/hooks/useKeyLifecycle.ts), [`utils/ed25519.ts`](../../../src/datafold_node/static-react/src/utils/ed25519.ts), [`utils/signing.ts`](../../../src/datafold_node/static-react/src/utils/signing.ts) already in TypeScript. **REMAINING:** Convert remaining .js hooks |
| UTS-1-6 | [Migrate core components from JSX to TSX with Redux integration](./UTS-1-6.md) | Proposed | Convert App, Header, and form components to TypeScript with proper Redux state typing |
| UTS-1-7 | [Migrate tab components from JSX to TSX with Redux state](./UTS-1-7.md) | Proposed | Convert all tab components to TypeScript with Redux schema and auth state integration |
| UTS-1-8 | [Enhance build process for component TypeScript compilation](./UTS-1-8.md) | Proposed | Ensure component TypeScript compilation works with existing Redux TypeScript setup |
| UTS-1-9 | [Create type definitions for constants and configurations](./UTS-1-9.md) | Proposed | Add TypeScript definitions for constants, configurations, and enums |
| UTS-1-10 | [Validate type safety with comprehensive testing](./UTS-1-10.md) | Proposed | Verify type safety implementation and ensure no regression in functionality |

## Cleanup Tasks

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UTS-1-11 | [Remove duplicate type definitions and consolidate interfaces](./UTS-1-11.md) | Proposed | Identify and remove duplicate TypeScript interfaces and type definitions |
| UTS-1-12 | [Remove legacy JavaScript files and unused type imports](./UTS-1-12.md) | Proposed | Clean up old .js files after .tsx migration and remove unused imports |
| UTS-1-13 | [Fix TypeScript compilation errors and ensure tests pass](./UTS-1-13.md) | Proposed | Run `npm test` and `tsc --noEmit` to fix compilation issues |
| UTS-1-14 | [Fix ESLint TypeScript rules and formatting](./UTS-1-14.md) | Proposed | Apply TypeScript ESLint rules and fix code style issues |
| UTS-1-15 | [Commit TypeScript migration and push changes](./UTS-1-15.md) | Proposed | Final commit with proper commit message for TypeScript migration |

## Development Server

Use `./run_http_server.sh` to start the server and test at http://localhost:9001/