# SKC-6-9 Document universal key field processing behavior

## Description
Update developer-facing documentation so the universal key field processing flow is fully described. Capture the new helper architecture, normalized mutation payload structure, and troubleshooting guidance for schemas using universal key configuration.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:45:00 | Created | N/A | Proposed | Documentation task created alongside the SKC-6 task split | ai-agent |
| 2025-09-24 14:30:00 | Status Update | Proposed | Done | Published normalized workflow docs and cross-links. | ai-agent |

## Requirements
- Document the universal key snapshot helper workflow in `docs/universal-key-migration-guide.md`, including how `{hash, range, fields}` travel through AtomManager and MutationService.
- Update API/reference docs describing `FieldValueSetRequest`, `MutationService`, and field processing so they reference the normalized payload structure and helper utilities in `docs/reference/fold_db_core/`.
- Add troubleshooting notes for common error scenarios (missing key configuration, dotted-path resolution failures, inconsistent payloads).
- Ensure documentation cross-links from the SKC-6 PRD and task list to the updated sections for discoverability.
- Maintain consistency with existing terminology and style guides; remove obsolete references to legacy heuristic helpers.

## Implementation Plan
1. Add a dedicated subsection to the universal key migration guide that diagrams the new helper interactions between MutationService, AtomManager, and downstream events.
2. Refresh API/reference documentation to describe the normalized payload structure, including code snippets that match the new implementations.
3. Update inline doc comments in `field_processing.rs` and `mutation.rs` where appropriate to point developers to the new documentation sections.
4. Review all links and formatting for accuracy, ensuring Markdown renders cleanly.

## Test Plan
- Proofread the rendered Markdown locally to validate formatting, anchors, and diagrams.
- Run `cargo test --workspace` to ensure documentation changes do not break doctests or Rust code snippets.
- Run `cargo clippy --all-targets --all-features` if inline source documentation is updated as part of this task.
- Run `npm test` inside `fold_node/src/datafold_node/static-react` if any UI documentation snippets involve runnable code.

## Verification
- Developers can follow the documentation to understand how universal key metadata flows through field processing and mutation pipelines.
- Troubleshooting steps cover the primary failure scenarios observed during implementation.
- Links from PRD and task documents resolve to the new sections.

## Files Modified
- `docs/universal-key-migration-guide.md`
- `docs/reference/fold_db_core/mutation_service.md`
- `docs/reference/fold_db_core/field_processing.md`
- `src/fold_db_core/managers/atom/field_processing.rs`
- `src/fold_db_core/services/mutation.rs`
- `docs/delivery/SKC-6/prd.md`
- `docs/delivery/SKC-6/tasks.md`

[Back to task list](../tasks.md)
