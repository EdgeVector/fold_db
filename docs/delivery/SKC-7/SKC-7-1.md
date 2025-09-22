# SKC-7-1: Integrate universal key configuration into aggregation pipeline

[Back to task list](./tasks.md)

## Description

Refactor the aggregation utilities so that result construction derives hash and range
keys from a schema's universal key configuration instead of hardcoded field names.
The pipeline must shape outputs via `shape_unified_result()` so all schema types
return a `{ hash, range, fields }` object that honors dotted key expressions and
backward compatibility expectations.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-20 10:00:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-24 12:45:00 | Status Update | Proposed | In Review/Complete | Universal key aggregation integration completed; awaiting review and documentation sync. | ai-agent |

## Requirements

### Functional Requirements
- Replace `_hash_field` / `_range_field` handling with schema-driven key mapping.
- Support schemas whose key configuration uses dotted path expressions.
- Shape aggregated results using `shape_unified_result()` for consistent output.
- Maintain backward compatibility for legacy Range schemas that rely on `range_key`.
- Provide actionable errors when key configuration is missing or malformed.

### Technical Requirements
- Update `aggregate_results_unified` (and helpers) to accept schema context needed
  for universal key extraction.
- Eliminate duplicated logic for renaming key fields and rely on shared helpers.
- Ensure aggregation works when ExecutionEngine returns zero, one, or many entries.
- Preserve logging and performance characteristics of the aggregation module.

### Dependencies
- Universal key utilities in `schema_operations` (`extract_unified_keys`,
  `shape_unified_result`).
- Transform executor coordination pathways that call aggregation helpers.

## Implementation Plan

### Step 1: Audit existing aggregation flows
- Review `src/transform/aggregation.rs` to catalog where hardcoded field names are
  assumed.
- Trace call sites in `src/transform/executor.rs` and `src/transform/coordination.rs`
  to confirm data passed into aggregation for each schema type.

### Step 2: Thread schema context into aggregation
- Modify `aggregate_results_unified` signature to accept the declarative schema
  definition (or minimal key metadata) alongside the current parameters.
- Update executor and coordination call sites to pass the schema reference without
  cloning large data structures unnecessarily.

### Step 3: Implement universal key-aware result shaping
- Inside aggregation, replace manual map assembly with logic that collects
  intermediate field data and calls `shape_unified_result(schema, &data, hash, range)`.
- Support dotted key expressions by resolving final segment names when populating
  the `fields` object returned by `shape_unified_result`.
- Ensure fallback paths (direct dotted resolution) also rely on schema-aware key
  mapping instead of hardcoded field constants.

### Step 4: Harden error handling and compatibility
- Validate key configuration presence for HashRange schemas and emit descriptive
  `SchemaError`s when requirements are unmet.
- Confirm Range schemas without explicit key config still resolve `range_key`
  correctly.
- Add targeted logging around key extraction outcomes to aid future debugging.

## Verification

### Acceptance Criteria
- [ ] Aggregation utilities no longer reference `_hash_field`, `_range_field`,
      `hash_key`, or `range_key` directly.
- [ ] Results across Single, Range, and HashRange schemas are produced through
      `shape_unified_result()` with consistent `{ hash, range, fields }` structure.
- [ ] Dotted key expressions in universal key config yield correctly named fields.
- [ ] Legacy Range schemas without universal key config continue to succeed.
- [ ] Meaningful errors are returned when key configuration requirements are unmet.

### Test Plan
1. Exercise unit tests covering aggregation with empty, single, and multi-entry
   ExecutionEngine outputs for each schema type.
2. Validate dotted key scenarios (e.g., `metadata.owner.hash`) produce the expected
   field names in the shaped result.
3. Run integration tests through the transform executor to confirm call-site
   compatibility and backward compatibility behavior.
4. Verify error paths by simulating schemas with incomplete key configuration.

## Files Modified

- `src/transform/aggregation.rs`
- `src/transform/executor.rs`
- `src/transform/coordination.rs`
- `src/schema/schema_operations.rs` (if auxiliary helpers are required)
