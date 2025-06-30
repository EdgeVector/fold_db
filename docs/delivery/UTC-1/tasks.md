# Tasks for PBI UTC-1: Test Coverage Enhancement for UI Components

This document lists all tasks associated with PBI UTC-1.

**Parent PBI**: [PBI UTC-1: Test Coverage Enhancement for UI Components](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UTC-1-1 | [Assess current test coverage and identify gaps](./UTC-1-1.md) | **Completed** | **COMPREHENSIVE TESTING FOUND:** [`test/`](../../../src/datafold_node/static-react/src/test/) directory with unit tests, integration tests, mocks, fixtures, utils. [`test/setup.js`](../../../src/datafold_node/static-react/src/test/setup.js), [`test/README.md`](../../../src/datafold_node/static-react/src/test/README.md) exist |
| UTC-1-2 | [Set up enhanced testing infrastructure and tools](./UTC-1-2.md) | **Completed** | **INFRASTRUCTURE COMPLETE:** React Testing Library setup in [`test/utils/testHelpers.tsx`](../../../src/datafold_node/static-react/src/test/utils/testHelpers.tsx), [`test/utils/testStore.jsx`](../../../src/datafold_node/static-react/src/test/utils/testStore.jsx), mocks in [`test/mocks/`](../../../src/datafold_node/static-react/src/test/mocks/) |
| UTC-1-3 | [Create unit tests for core application components](./UTC-1-3.md) | **In Progress** | **PARTIALLY DONE:** Tests exist for [`Header.test.jsx`](../../../src/datafold_node/static-react/src/test/components/Header.test.jsx), [`StatusSection.test.jsx`](../../../src/datafold_node/static-react/src/test/components/StatusSection.test.jsx), [`TabNavigation.test.jsx`](../../../src/datafold_node/static-react/src/test/components/TabNavigation.test.jsx). **REMAINING:** App.jsx main test |
| UTC-1-4 | [Create unit tests for form components and validation](./UTC-1-4.md) | **In Progress** | **PARTIALLY DONE:** [`test/components/form/`](../../../src/datafold_node/static-react/src/test/components/form/) with [`FieldWrapper.test.jsx`](../../../src/datafold_node/static-react/src/test/components/form/FieldWrapper.test.jsx), [`TextField.test.jsx`](../../../src/datafold_node/static-react/src/test/components/form/TextField.test.jsx), validation tests in [`constants/__tests__/validation.test.js`](../../../src/datafold_node/static-react/src/constants/__tests__/validation.test.js) |
| UTC-1-5 | [Create unit tests for tab components](./UTC-1-5.md) | **In Progress** | **PARTIALLY DONE:** [`test/components/tabs/SchemaTab.test.jsx`](../../../src/datafold_node/static-react/src/test/components/tabs/SchemaTab.test.jsx) exists. **REMAINING:** Complete tab component coverage |
| UTC-1-6 | [Create tests for custom hooks and utilities with Redux integration](./UTC-1-6.md) | **In Progress** | **PARTIALLY DONE:** Hook tests in [`hooks/__tests__/`](../../../src/datafold_node/static-react/src/hooks/__tests__/) including [`useApprovedSchemas.test.js`](../../../src/datafold_node/static-react/src/hooks/__tests__/useApprovedSchemas.test.js), utils tests in [`utils/__tests__/`](../../../src/datafold_node/static-react/src/utils/__tests__/) |
| UTC-1-7 | [Enhance existing Redux tests and add component integration testing](./UTC-1-7.md) | **In Progress** | **STRONG BASE:** [`store/__tests__/schemaSlice.test.js`](../../../src/datafold_node/static-react/src/store/__tests__/schemaSlice.test.js), integration tests in [`test/integration/`](../../../src/datafold_node/static-react/src/test/integration/) including [`ReduxAuthIntegration.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/ReduxAuthIntegration.test.jsx) (378 lines comprehensive testing) |
| UTC-1-8 | [Create integration tests for Redux-connected workflows](./UTC-1-8.md) | **In Progress** | **EXCELLENT WORK:** [`test/integration/ReduxAuthIntegration.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/ReduxAuthIntegration.test.jsx) tests AUTH-003 synchronization, [`ComponentIntegration.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/ComponentIntegration.test.jsx), [`WorkflowTests.test.jsx`](../../../src/datafold_node/static-react/src/test/integration/WorkflowTests.test.jsx) |
| UTC-1-9 | [Add API integration and error handling tests](./UTC-1-9.md) | **In Progress** | **MOCKS READY:** [`test/mocks/apiMocks.js`](../../../src/datafold_node/static-react/src/test/mocks/apiMocks.js), [`test/utils/authMocks.ts`](../../../src/datafold_node/static-react/src/test/utils/authMocks.ts) exist. **REMAINING:** Complete API error testing coverage |
| UTC-1-10 | [Configure coverage reporting and CI integration](./UTC-1-10.md) | Proposed | Set up coverage metrics, reporting, and continuous integration testing |

## Cleanup Tasks

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| UTC-1-11 | [Remove duplicate test utilities and consolidate mocks](./UTC-1-11.md) | Proposed | Identify and remove duplicate test helpers and consolidate mock functions |
| UTC-1-12 | [Remove legacy test files and unused test imports](./UTC-1-12.md) | Proposed | Clean up old test files and remove unused testing utilities |
| UTC-1-13 | [Fix failing tests and ensure all test suites pass](./UTC-1-13.md) | Proposed | Run `npm test` and fix any failing or broken tests |
| UTC-1-14 | [Fix ESLint testing rule violations and formatting](./UTC-1-14.md) | Proposed | Apply testing ESLint rules and fix test code style issues |
| UTC-1-15 | [Commit test improvements and push changes](./UTC-1-15.md) | Proposed | Final commit with proper commit message for test coverage improvements |

## Development Server

Use `./run_http_server.sh` to start the server and test at http://localhost:9001/