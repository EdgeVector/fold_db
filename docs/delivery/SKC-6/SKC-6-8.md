# SKC-6-8 Expand universal key regression test coverage

## Description
Create a focused battery of unit and integration tests that validate universal key behavior across AtomManager, MutationService, and downstream producers. The goal is to guard against regressions introduced by the refactors and to cover dotted-path configurations, error scenarios, and backward compatibility cases.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:40:00 | Created | N/A | Proposed | Task created to consolidate new regression coverage | ai-agent |

## Requirements
- Add unit tests for the universal key snapshot helper, MutationService payload builder, and any shared utilities introduced during refactors, focusing on success and failure cases.
- Create or extend integration tests that cover Single, Range, and HashRange workflows end-to-end, ensuring AtomManager storage, mutation context, and event publishing all reflect the normalized metadata.
- Add fixtures for dotted-path key schemas and legacy Range schemas to make tests concise and DRY.
- Ensure tests assert error messaging for invalid configurations (missing key definitions, absent data) to catch silent regressions.
- Document complex test scenarios with concise comments so future maintainers understand the coverage.

## Implementation Plan
1. Organize new unit tests under `tests/unit/field_processing/` and `tests/unit/mutation/`, consolidating shared fixtures in `tests/test_utils.rs`.
2. Extend existing integration suites (`hashrange_end_to_end_workflow_test.rs`, `complete_mutation_query_flow_test.rs`, etc.) with scenarios covering universal key payloads and dotted paths.
3. Add a new integration test if needed to explicitly cover Single schema universal key flows when none exist today.
4. Review coverage to ensure both success and failure paths are exercised, adding negative tests where gaps remain.

## Test Plan
- Run `cargo test --workspace` to execute all new unit and integration tests.
- Run `cargo clippy --all-targets --all-features` to ensure the test additions meet lint expectations.
- Run `npm test` inside `fold_node/src/datafold_node/static-react` if any frontend fixtures or docs contain code snippets referencing the new APIs.

## Verification
- Tests fail before the refactors and pass after they are implemented, demonstrating coverage of new behavior.
- Dotted-path and legacy scenarios remain green, proving backward compatibility.
- Negative tests surface clear error messages when configurations are invalid.

## Files Modified
- `tests/unit/field_processing/*`
- `tests/unit/mutation/*`
- `tests/integration/hashrange_end_to_end_workflow_test.rs`
- `tests/integration/complete_mutation_query_flow_test.rs`
- `tests/test_utils.rs`

[Back to task list](../tasks.md)
