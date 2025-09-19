# SKC-7-2: Add aggregation test coverage for universal key scenarios

[Back to task list](./tasks.md)

## Description

Expand automated test coverage so aggregation utilities are validated against
universal key configuration scenarios, including dotted key expressions and
legacy fallback behavior. Tests must exercise both direct aggregation helpers
and end-to-end execution flows in the transform executor.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-20 10:05:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

### Functional Requirements
- Cover Single, Range, and HashRange schemas that rely on universal key config.
- Validate dotted key expressions resolve to the correct output field names.
- Ensure aggregation gracefully handles empty ExecutionEngine results.
- Confirm legacy Range schemas without key config retain expected formatting.
- Capture regression tests for error cases (e.g., missing key fields, mismatched
  schema metadata).

### Technical Requirements
- Extend unit tests in `src/transform/aggregation.rs` or move them to a dedicated
  test module for broader scenarios.
- Add integration tests that execute transforms via `TransformExecutor` using
  fixture schemas configured with universal keys.
- Reuse shared test utilities for schema loading instead of duplicating setup
  logic.
- Keep tests deterministic and independent so they run reliably in CI.

### Dependencies
- Updated aggregation implementation from task SKC-7-1.
- Universal key fixtures introduced in earlier SKC PBIs (schemas/tests utilities).

## Implementation Plan

### Step 1: Assess current coverage gaps
- Review existing unit tests within `src/transform/aggregation.rs` and identify
  missing universal key cases (dotted paths, multi-entry arrays, error paths).
- Inspect integration tests such as `tests/integration/hashrange_end_to_end_workflow_test.rs`
  to understand current coverage and insertion points.

### Step 2: Author focused unit tests
- Create new cases that invoke aggregation directly with mocked ExecutionEngine
  entries representing multi-row HashRange outputs.
- Add tests verifying `shape_unified_result()` output when aggregation receives
  dotted key expressions and when ExecutionEngine returns no entries.
- Validate error handling by simulating incomplete key configuration.

### Step 3: Build integration/E2E style scenarios
- Define fixture schemas (Single, Range, HashRange) that rely on universal keys
  and dotted expressions; leverage shared schema factory utilities if available.
- Execute transforms through `TransformExecutor` to assert final shaped result
  structure matches `{ hash, range, fields }` expectations for each schema type.
- Include regression coverage for legacy Range schemas that should still pass.

### Step 4: Finalize test harness updates
- Ensure new tests are referenced by the workspace test runner (`cargo test`).
- Document any new fixtures or helpers to keep future maintenance simple.

## Verification

### Acceptance Criteria
- [ ] Unit tests validate aggregation for Single, Range, and HashRange universal
      key scenarios, including dotted paths and empty results.
- [ ] Integration tests confirm transform execution outputs use the universal
      key-shaped result structure.
- [ ] Regression tests exist for legacy Range schemas without key config.
- [ ] Tests assert meaningful errors when key configuration is incomplete.
- [ ] CI test suite passes without flaky behavior introduced by new cases.

### Test Plan
1. Run `cargo test --workspace` to execute unit and integration tests.
2. Focus on new test modules by running targeted commands (e.g.,
   `cargo test aggregation_universal_key` once implemented) to iterate quickly.
3. Validate integration scenarios with `cargo test --test hashrange_end_to_end_workflow_test`
   or equivalent targeted suites.
4. Confirm error-focused tests fail when key configuration handling regresses.

## Files Modified

- `src/transform/aggregation.rs` (unit tests section)
- `tests/integration/hashrange_end_to_end_workflow_test.rs`
- `tests/unit/transform/universal_key_aggregation_tests.rs` (new or updated)
- Supporting fixtures under `tests/fixtures/` as required
