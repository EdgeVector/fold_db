# Tasks for PBI UCR-1: Component Complexity Reduction for UI Maintainability

This document lists all tasks associated with PBI UCR-1.

**Parent PBI**: [PBI UCR-1: Component Complexity Reduction for UI Maintainability](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UCR-1-1 | [Analyze QueryTab component responsibilities and create refactoring plan](./UCR-1-1.md) | **Completed** | **PATTERN ESTABLISHED:** [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) (383 lines) shows good Redux integration. [`QueryTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/QueryTab.jsx) needs analysis. Component extraction pattern seen in [`SchemaActions.jsx`](../../../src/datafold_node/static-react/src/components/schema/SchemaActions.jsx) |
| UCR-1-2 | [Extract custom hooks for query state management with Redux integration](./UCR-1-2.md) | **Completed** | **FOUNDATION EXISTS:** Custom hooks pattern established in [`hooks/`](../../../src/datafold_node/static-react/src/hooks/) with [`useApprovedSchemas.js`](../../../src/datafold_node/static-react/src/hooks/useApprovedSchemas.js) using Redux. Hook index in [`hooks/index.js`](../../../src/datafold_node/static-react/src/hooks/index.js) shows TASK-009 organization |
| UCR-1-3 | [Create QueryBuilder component with Redux schema integration](./UCR-1-3.md) | **Completed** | Extract query building logic using Redis schema state and authentication from existing store |
| UCR-1-4 | [Create QueryForm component for input validation](./UCR-1-4.md) | **Completed** | **FORM PATTERN EXISTS:** Form components in [`components/form/`](../../../src/datafold_node/static-react/src/components/form/) including [`FieldWrapper.jsx`](../../../src/datafold_node/static-react/src/components/form/FieldWrapper.jsx), [`SelectField.jsx`](../../../src/datafold_node/static-react/src/components/form/SelectField.jsx), [`TextField.jsx`](../../../src/datafold_node/static-react/src/components/form/TextField.jsx) show extraction pattern |
| UCR-1-5 | [Create QueryPreview component for query visualization](./UCR-1-5.md) | **Completed** | Extract query preview and visualization logic into dedicated QueryPreview component |
| UCR-1-6 | [Create QueryActions component for execution controls](./UCR-1-6.md) | **Completed** | **PATTERN ESTABLISHED:** [`SchemaActions.jsx`](../../../src/datafold_node/static-react/src/components/schema/SchemaActions.jsx) (207 lines) shows excellent action component pattern with Redux integration, proper JSDoc, constants usage. Template for QueryActions |
| UCR-1-7 | [Refactor parent QueryTab to orchestrate child components and Redux state](./UCR-1-7.md) | **Completed** | **REDUX INTEGRATION READY:** [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) lines 16-21 shows proper Redux hooks usage with [`useAppSelector, useAppDispatch`](../../../src/datafold_node/static-react/src/store/hooks.ts) |
| UCR-1-8 | [Add unit tests for extracted components and hooks](./UCR-1-8.md) | **Completed** | **INFRASTRUCTURE READY:** Testing patterns in [`test/components/`](../../../src/datafold_node/static-react/src/test/components/) and [`hooks/__tests__/`](../../../src/datafold_node/static-react/src/hooks/__tests__/). Test utilities in [`test/utils/testHelpers.tsx`](../../../src/datafold_node/static-react/src/test/utils/testHelpers.tsx) |
| UCR-1-9 | [Add integration tests for QueryTab composition](./UCR-1-9.md) | **Completed** | **PATTERNS ESTABLISHED:** Integration testing in [`test/integration/ComponentIntegration.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/ComponentIntegration.test.jsx), Redux integration testing in [`test/integration/ReduxAuthIntegration.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/ReduxAuthIntegration.test.jsx) |
| UCR-1-10 | [Update component documentation with JSDoc](./UCR-1-10.md) | **Completed** | **TEMPLATE READY:** Excellent JSDoc pattern in [`useApprovedSchemas.js`](../../../src/datafold_node/static-react/src/hooks/useApprovedSchemas.js) lines 1-117 provides comprehensive template for component documentation |

## Cleanup Tasks

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UCR-1-11 | [Remove duplicate code and consolidate similar functions](./UCR-1-11.md) | **Completed** | Identify and remove duplicate code patterns across components |
| UCR-1-12 | [Remove legacy code and unused imports](./UCR-1-12.md) | **Completed** | Clean up unused code, imports, and deprecated patterns |
| UCR-1-13 | [Fix failing tests and ensure all tests pass](./UCR-1-13.md) | **Completed** | Run `npm test` and fix any failing tests after refactoring |
| UCR-1-14 | [Fix ESLint and formatting issues](./UCR-1-14.md) | **Completed** | Run linting tools and fix code style issues |
| UCR-1-15 | [Commit changes and push to repository](./UCR-1-15.md) | **Completed** | Final commit with proper commit message and push changes |

## Development Server

Use `./run_http_server.sh` to start the server and test at http://localhost:9001/