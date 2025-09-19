# PBI-SKC-1: Unify schema key configuration across Single, Range, and HashRange

[View in Backlog](../backlog.md#user-content-SKC-1)

## Overview
Unify the schema-level key configuration by introducing a universal `key` config structure applicable to all schema types (Single, Range, HashRange). This enables consistent parsing, storage, and query formatting, simplifying code paths and reducing conditional logic.

## Problem Statement
- Current behavior treats `key` as required only for `HashRange`, optional/ignored for `Single`, and separate `range_key` for `Range`.
- This leads to divergent parsing and execution branches, duplicated logic, and inconsistent query output formatting.
- We want a single, explicit key configuration for all types to improve consistency and reduce complexity while maintaining backward compatibility.

## User Stories
- As a developer, I want a universal `key` config so I can handle all schema types with a unified code path.
- As a developer, I want consistent query formatting (hash->range->fields) so downstream consumers have predictable results.
- As a maintainer, I want fewer conditionals and less duplication so the system is easier to evolve.

## Technical Approach
- Define/extend a canonical `KeyConfig { hash_field?: string, range_field?: string }` usable by all schema types.
  - Single: `key` optional; if present may include `hash_field` or `range_field` (used for indexing/ordering hints); defaults to none.
  - Range: `key.range_field` determines the range dimension; `hash_field` optional (enables sharding if needed).
  - HashRange: both `hash_field` and `range_field` required (unchanged).
- Parsing:
  - Update JSON schema parsing to accept `key` for all schema types and validate per-type rules.
  - Preserve backward compatibility: continue supporting `SchemaType::Range { range_key }` and legacy schemas without `key`.
- Execution & Query Formatting:
  - Standardize result shape as hash->range->fields for all types:
    - Single: `hash` and `range` may be null/missing; fields returned under consistent `fields` object.
    - Range: `range` present; `hash` optional if provided in `key`.
    - HashRange: both present.
  - Centralize key extraction in one helper used by executor and query APIs.
- Documentation & Migration:
  - Document the universal `key` format with examples.
  - Recommend using `key` for new schemas; keep legacy formats working.

## UX/UI Considerations
- UI helpers that detect schema type should read `schema_type` and optional `key` consistently.
- Avoid additional UI complexity; rely on existing range helpers with minor updates to support optional `hash_field` on Range and optional `key` on Single.

## Acceptance Criteria
- Universal `key` supported across Single, Range, HashRange with type-appropriate validation.
- Backward compatibility retained for existing schemas (no breaking changes).
- Query result formatting is consistent as hash->range->fields for all types.
- One consolidated code path for key extraction/handling in backend.
- Docs updated with examples and guidance.

## Dependencies
- Parser and validation in `src/schema/types/json_schema.rs` and `src/schema/types/schema.rs`.
- Executor and schema operations in `src/transform/executor.rs` and `src/schema/schema_operations.rs`.
- UI utilities in `src/datafold_node/static-react/src/utils/*SchemaHelpers.js`.

## Open Questions
- Should Single schemas allow `range_field` in `key`, and how should ordering be exposed if present?
- Do we expose null vs omit for missing `hash`/`range` consistently across APIs?

## Documentation
- **Schema Management Guide**: Updated with universal key configuration examples and migration guidance
- **Migration Guide**: Comprehensive [Universal Key Migration Guide](../../universal-key-migration-guide.md) with step-by-step instructions
- **Migration Examples**: See [Schema Management Documentation](../../schema-management.md#universal-key-configuration) for detailed examples and best practices

## Related Tasks
To be created in `docs/delivery/SKC-1/tasks.md` following the tasks framework.
