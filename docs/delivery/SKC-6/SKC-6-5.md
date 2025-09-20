# SKC-6-5 Implement normalized FieldValueSet payload builder in MutationService

## Description
Introduce a MutationService helper that resolves schema key metadata and assembles a normalized `FieldValueSetRequest` payload containing `hash`, `range`, and `fields` sections. This builder becomes the single entry point for constructing mutation requests, ensuring downstream consumers receive consistent, schema-derived data.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:25:00 | Created | N/A | Proposed | Task file created during SKC-6 task decomposition | ai-agent |

## Requirements
- Add a private builder (e.g., `build_field_value_request`) within `MutationService` that loads schema metadata, resolves universal key information, and constructs the standard request payload structure.
- The builder must support Single, Range, and HashRange schemas, using universal key helpers for key name/value resolution while preserving backward compatibility for legacy Range schemas.
- Return both the serialized `FieldValueSetRequest` and a lightweight context struct (hash, range, fields) so callers can use the data without reparsing JSON.
- Capture detailed error context when schema lookup or key resolution fails; no silent defaults are allowed.
- Keep the builder DRY by reusing existing schema accessor and validation utilities where possible.

## Implementation Plan
1. Define a `NormalizedFieldValueRequest` struct encapsulating the request and resolved key metadata.
2. Implement the builder to fetch schema definitions, call universal key helpers, and assemble the normalized request payload with deterministic ordering.
3. Update internal MutationService modules to expose the builder for subsequent tasks without yet replacing existing call sites.
4. Add targeted logging (trace/debug) to confirm when normalized requests are constructed, omitting verbose println diagnostics.

## Test Plan
- Add unit tests for the builder exercising Single, Range (legacy + universal), and HashRange schemas using fixtures from `tests/test_utils.rs`.
- Include negative tests for missing key configuration and invalid payload inputs to ensure descriptive errors.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` after adding the builder and tests.

## Verification
- The builder returns a well-structured `FieldValueSetRequest` with `fields`, `hash`, and `range` populated from schema metadata.
- Error cases raise detailed messages without falling back to caller-provided key names.
- Existing MutationService behavior remains unchanged until later tasks adopt the builder.

## Files Modified
- `src/fold_db_core/services/mutation.rs`
- `tests/unit/mutation/field_value_request_builder_tests.rs`
- `tests/test_utils.rs`

[Back to task list](../tasks.md)
