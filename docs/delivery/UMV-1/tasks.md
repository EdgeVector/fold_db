# Tasks for PBI UMV-1: Magic Values Elimination and Constants Organization

This document lists all tasks associated with PBI UMV-1.

**Parent PBI**: [PBI UMV-1: Magic Values Elimination and Constants Organization](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UMV-1-1 | [Audit component magic values and assess Redux constants integration](./UMV-1-1.md) | **Completed** | **ANALYSIS COMPLETE:** Magic values identified in [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx), [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx). Redux constants in [`constants/redux.js`](../../../src/datafold_node/static-react/src/constants/redux.js) (308 lines) well-structured |
| UMV-1-2 | [Reorganize constants file and integrate with Redux constants](./UMV-1-2.md) | **In Progress** | **PARTIALLY DONE:** Constants already well-organized in [`constants/`](../../../src/datafold_node/static-react/src/constants/) directory with 12 specialized files including [`redux.js`](../../../src/datafold_node/static-react/src/constants/redux.js), [`validation.js`](../../../src/datafold_node/static-react/src/constants/validation.js), [`ui.js`](../../../src/datafold_node/static-react/src/constants/ui.js). **REMAINING:** Component integration |
| UMV-1-3 | [Extract SQL operation constants and query builders](./UMV-1-3.md) | Proposed | Create centralized constants for SQL operations, clauses, and query building patterns |
| UMV-1-4 | [Extract API configuration constants](./UMV-1-4.md) | **In Progress** | **PARTIALLY DONE:** API constants in [`constants/api.ts`](../../../src/datafold_node/static-react/src/constants/api.ts) exist. **REMAINING:** Complete centralization from component magic values |
| UMV-1-5 | [Extract UI and form validation constants](./UMV-1-5.md) | **In Progress** | **LARGELY COMPLETE:** Comprehensive validation constants in [`constants/validation.js`](../../../src/datafold_node/static-react/src/constants/validation.js) (281 lines), UI constants in [`constants/ui.js`](../../../src/datafold_node/static-react/src/constants/ui.js), styling in [`constants/styling.js`](../../../src/datafold_node/static-react/src/constants/styling.js). **REMAINING:** Component adoption |
| UMV-1-6 | [Update components to use centralized constants and Redux state](./UMV-1-6.md) | **In Progress** | **PARTIALLY DONE:** [`SchemaActions.jsx`](../../../src/datafold_node/static-react/src/components/schema/SchemaActions.jsx) uses [`constants/ui.js`](../../../src/datafold_node/static-react/src/constants/ui.js) and [`constants/styling.js`](../../../src/datafold_node/static-react/src/constants/styling.js). **REMAINING:** Update [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) magic values ('approved', 'available', 'blocked') |
| UMV-1-7 | [Add TypeScript definitions for constants](./UMV-1-7.md) | **In Progress** | **PARTIALLY DONE:** [`constants/api.ts`](../../../src/datafold_node/static-react/src/constants/api.ts) already in TypeScript. **REMAINING:** Convert .js constants to .ts with proper typing |
| UMV-1-8 | [Create validation for critical configuration values](./UMV-1-8.md) | **In Progress** | **FUNCTIONS EXIST:** Validation functions in [`constants/validation.js`](../../../src/datafold_node/static-react/src/constants/validation.js) lines 198-268. **REMAINING:** Runtime validation implementation |
| UMV-1-9 | [Add ESLint rules to prevent magic values](./UMV-1-9.md) | Proposed | Configure linting rules to detect and prevent new magic values in code |
| UMV-1-10 | [Document constants organization and usage patterns](./UMV-1-10.md) | Proposed | Create comprehensive documentation for constants structure and usage guidelines |

## Cleanup Tasks

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UMV-1-11 | [Remove duplicate constants and consolidate similar values](./UMV-1-11.md) | Proposed | Identify and remove duplicate constant definitions across files |
| UMV-1-12 | [Remove legacy magic values and unused constant imports](./UMV-1-12.md) | Proposed | Clean up old magic values in components and remove unused constant imports |
| UMV-1-13 | [Fix failing tests after constants refactoring](./UMV-1-13.md) | Proposed | Run `npm test` and fix any tests broken by constants changes |
| UMV-1-14 | [Fix ESLint magic number violations and formatting](./UMV-1-14.md) | Proposed | Apply no-magic-numbers ESLint rules and fix violations |
| UMV-1-15 | [Commit constants refactoring and push changes](./UMV-1-15.md) | Proposed | Final commit with proper commit message for constants organization |

## Development Server

Use `./run_http_server.sh` to start the server and test at http://localhost:9001/