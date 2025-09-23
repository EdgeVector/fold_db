# Tasks for PBI SKC-6: Update Field Processing Utilities for Universal Key Configuration

This document lists all tasks associated with PBI SKC-6.

**Parent PBI**: [PBI SKC-6: Update Field Processing Utilities for Universal Key Configuration](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :------------------------------------------------------------ | :------- | :-------------------------------------------------------------- |
| SKC-6-1 | [Introduce schema-driven key snapshot helper for field processing](./SKC-6-1.md) | Done | Add a universal key helper that returns normalized hash/range metadata for AtomManager. |
| SKC-6-2 | [Refactor Single and Range molecule creation to use universal key snapshot](./SKC-6-2.md) | Done | Adopt the helper for Single & Range flows so molecules and responses rely on schema-derived keys. |
| SKC-6-3 | [Refactor HashRange pipeline to use universal key snapshot](./SKC-6-3.md) | Done | Extend the helper to HashRange storage and events. |
| SKC-6-4 | [Retire legacy key heuristics and tighten error reporting](./SKC-6-4.md) | Done | Remove obsolete key extraction helpers and unify error handling. |
| SKC-6-5 | [Implement normalized FieldValueSet payload builder in MutationService](./SKC-6-5.md) | Done | Create a builder that assembles schema-derived mutation payloads. |
| SKC-6-6 | [Adopt normalized payload builder in mutation workflows](./SKC-6-6.md) | Done | Update MutationService flows to publish normalized payloads. |
| SKC-6-7 | [Align downstream producers with normalized mutation payloads](./SKC-6-7.md) | Done | Refactor transform/message bus producers to use the shared payload shape. |
| SKC-6-8 | [Expand universal key regression test coverage](./SKC-6-8.md) | Done | Add comprehensive unit and integration tests for universal key workflows. |
| SKC-6-9 | [Document universal key field processing behavior](./SKC-6-9.md) | Review | Refresh documentation to describe the new helpers and payload structure. |
| SKC-6-10 | [Remove legacy fallback logic from universal key resolution](./SKC-6-10.md) | Done | Remove create_legacy_resolved_keys fallback introduced in SKC-6-2 to enforce strict schema-driven key extraction. |

**Documentation References**

- [Universal Key Migration Guide workflow](../../guides/operations/universal-key-migration-guide.md#universal-key-processing-workflow)
- [MutationService reference](../../reference/fold_db_core/mutation_service.md)
- [Field processing reference](../../reference/fold_db_core/field_processing.md)
