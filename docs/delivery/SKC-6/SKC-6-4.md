# SKC-6-4 Document universal key field processing behavior

## Description

Update developer-facing documentation so field processing workflows with universal key configuration are fully described. Capture new helper usage, payload structure, and troubleshooting guidance for Range and HashRange schemas.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:15:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

- Document the normalized FieldValueSet payload format and key extraction flow in `docs/guides/operations/universal-key-migration-guide.md` (or a new referenced section).
- Update API/reference docs describing `FieldValueSetRequest`, `MutationService`, and AtomManager field processing to reflect universal key usage.
- Capture troubleshooting steps for common error scenarios (missing key config, dotted path resolution failures, inconsistent payloads).
- Ensure documentation links from the PBI PRD and related tasks reference the updated sections.
- Maintain consistency with existing terminology and style guides across docs.

## Implementation Plan

1. Add a dedicated subsection in the universal key migration guide covering field processing utilities, including diagrams or flow descriptions as needed.
2. Update API or reference documentation (e.g., `docs/reference/fold_db_core/` or equivalent) to describe the normalized payload structure and helper functions.
3. Refresh inline documentation in source files (`field_processing.rs`, `mutation.rs`) where helpful, ensuring comments reference the updated docs.
4. Cross-link the documentation updates from the SKC-6 PRD and task list so developers can find the new material easily.
5. Perform proofreading and consistency checks to ensure formatting, terminology, and links comply with documentation standards.

## Test Plan

- Proofread rendered Markdown locally to ensure formatting and links are correct.
- Run `cargo test --workspace` to verify documentation changes do not break doctests or code snippets.
- Run `cargo clippy --all-targets --all-features` if source comments are updated alongside docs.

## Verification

- Documentation clearly explains how universal key extraction is used within field processing workflows.
- All links resolve correctly, and doctest snippets (if any) compile.
- PRD references and related tasks point to the updated documentation sections.

## Files Modified

- `docs/guides/operations/universal-key-migration-guide.md`
- `docs/reference/...` describing field processing APIs
- Inline doc comments within relevant Rust source files (as needed)
- `docs/delivery/SKC-6/prd.md` (link updates, if required)

[Back to task list](../tasks.md)
