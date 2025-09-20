# SKC-6-3 Refactor HashRange pipeline to use universal key snapshot

## Description
Extend the universal key snapshot adoption to the HashRange molecule flow, including storage updates and event publishing. This task ensures HashRange mutations persist and emit schema-derived hash/range metadata without relying on bespoke JSON parsing or map manipulation.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:10:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-23 09:15:00 | Scope Refined | Proposed | Proposed | Task retargeted to HashRange-specific adoption of the universal snapshot | ai-agent |

## Requirements
- Update `create_hashrange_molecule` to consume the universal key snapshot, storing hash/range values based on schema-derived field names rather than hardcoded `hash_key`/`range_key` JSON lookups.
- Ensure HashRange persistence writes and cache updates reuse shared helpers (for serialization, key formatting) where possible to keep code DRY.
- Modify `publish_field_value_set_event` (and any related event builders) so mutation events include the normalized hash/range fields from the snapshot.
- Adjust `handle_successful_field_value_processing` to propagate the normalized metadata for HashRange responses and notifications.
- Remove `extract_hash_key_from_value` usage within the HashRange branch; any remaining heuristic helpers should be left for the cleanup task.

## Implementation Plan
1. Thread the universal snapshot into `create_hashrange_molecule`, using the resolved hash/range values to construct persistence keys and payloads.
2. Replace manual JSON manipulation with helper-driven updates when storing HashRange BTree data, ensuring dotted-path configurations work.
3. Update event publishing to serialize the snapshot's hash/range fields into `FieldValueSet` events and related telemetry.
4. Validate error handling by ensuring failures within the HashRange branch bubble up with context-rich messages based on the snapshot helper's output.

## Test Plan
- Extend unit tests for HashRange processing to verify persistence keys, stored payloads, and responses use schema-derived names and values.
- Update integration coverage (e.g., `tests/integration/hashrange_end_to_end_workflow_test.rs`) to assert HashRange events expose the normalized metadata.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` to confirm the refactor compiles and passes lint checks.

## Verification
- HashRange storage and event flows rely solely on the universal snapshot for key data.
- Responses and events include accurate hash/range metadata even for dotted-path key definitions.
- No direct JSON probing remains in `create_hashrange_molecule` or event publishing logic.

## Files Modified
- `src/fold_db_core/managers/atom/field_processing.rs`
- `src/fold_db_core/infrastructure/message_bus/atom_events.rs`
- `tests/integration/hashrange_end_to_end_workflow_test.rs`
- `tests/unit/field_processing/hashrange_tests.rs`

[Back to task list](../tasks.md)
