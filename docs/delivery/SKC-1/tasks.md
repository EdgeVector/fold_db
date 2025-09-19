# Tasks for PBI SKC-1: Unify schema key configuration across Single, Range, and HashRange

This document lists all tasks associated with PBI SKC-1.

**Parent PBI**: [PBI SKC-1: Unify schema key configuration across Single, Range, and HashRange](./prd.md)

## Task Summary

| Task ID | Name | Status | Description |
| :------ | :--------------------------------------- | :------- | :--------------------------------- |
| SKC-1-1 | [Define universal KeyConfig and validation rules](./SKC-1-1.md) | Done | Introduce universal KeyConfig applicable to Single, Range, and HashRange with per-type validation. |
| SKC-1-2 | [Parser updates to accept key for all schema types](./SKC-1-2.md) | Done | Update JSON parsing to accept key for all types and preserve legacy Range { range_key } support. |
| SKC-1-3 | [Backend unify key extraction and result shaping](./SKC-1-3.md) | Done | Consolidate key handling and standardize output as hash->range->fields across types. |
| SKC-1-4 | [UI helpers support universal key and consistent detection](./SKC-1-4.md) | Done | Update UI utilities to read optional key on Single/Range and required on HashRange. |
| SKC-1-5 | [Docs and migration guide for universal key](./SKC-1-5.md) | Review | Document new universal key format and provide migration guidance and examples. |
| SKC-1-6 | [E2E CoS test for SKC-1](./SKC-1-6.md) | Review | Add E2E test task verifying CoS across Single, Range, HashRange. |
| SKC-1-7 | [Remove legacy Range { range_key } branching in backend](./SKC-1-7.md) | Proposed | Replace ad-hoc range_key branches with unified key helper; keep parsing compat. |
| SKC-1-8 | [Consolidate JSON readers for key config](./SKC-1-8.md) | Proposed | Remove duplicate key readers; centralize in one module with tests. |
| SKC-1-9 | [Retire redundant UI detection code paths](./SKC-1-9.md) | Proposed | Delete specialized detection in favor of universal key-based helpers. |
| SKC-1-10 | [Delete dead types and constants related to legacy keys](./SKC-1-10.md) | Proposed | Remove unused Range/HashRange split constants, types, and flags. |
