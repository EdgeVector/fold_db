# Project Logic

This document contains the most up-to-date and condensed information about the project's logical processes, built up over time. Each logic entry tracks which modules it applies to and any conflicts with other logic.

## Logic Table

| Logic_ID | Logic | Module | Updated At | Conflicts with |
|----------|-------|--------|------------|----------------|
| SCHEMA-001 | Once approved, schemas cannot be unloaded. They can only transition between approved and blocked states. | schema_manager, schema_routes | 2025-06-23 19:14:00 | None |
| SCHEMA-002 | Only approved schemas can be mutated or queried by the user. | query_routes, mutation_tab, query_tab, schema_manager | 2025-06-23 19:17:00 | None |
| SCHEMA-003 | Transforms can write field values to any schema regardless of state, but cannot modify schema structure. | transform_manager, transform_queue, schema_manager | 2025-06-23 19:23:00 | None |
| SCHEMA-004 | Available schemas are discovered from JSON files only and are not loaded automatically. | schema/discovery, schema/persistence | 2025-06-24 21:49:43 | None |
| LOG-001 | File size strings for log rotation must be numeric with optional KB, MB, or GB suffixes and are parsed by `parse_file_size`. | logging/config, logging/outputs | 2025-06-25 00:28:57 | None |

### SCHEMA-001: Schema State Transition Rules
- **Description**: Enforces valid state transitions for schema lifecycle management
- **Rationale**: Prevents data loss and ensures production schemas remain available; once approved, schemas are considered critical and cannot be unloaded
- **Valid Transitions**:
  - available → approved (initial approval)
  - approved → blocked (temporary blocking)
  - blocked → approved (re-approval)
- **Invalid Transitions**:
  - approved → unloaded (prohibited - approved schemas cannot be unloaded)
  - blocked → unloaded (prohibited - previously approved schemas cannot be unloaded)

### SCHEMA-002: Schema Access Control Rules
- **Description**: Restricts user access to mutation and query operations based on schema state
- **Rationale**: Ensures data integrity and prevents operations on schemas that are not ready for production use
- **Access Rules**:
  - Only schemas with "approved" state can be used for mutations
  - Only schemas with "approved" state can be used for queries
  - Available and blocked schemas are read-only for inspection purposes
  - Transform viewing is allowed for all schemas (inspection purposes only)
- **Enforcement Points**:
  - Query tab should only allow queries against approved schemas
  - Mutation tab should only allow mutations against approved schemas
  - Transforms tab shows all transforms for inspection regardless of schema state
  - API endpoints should validate schema state before processing requests

### SCHEMA-003: Transform Field Write Access
- **Description**: Allows transforms to write field values while protecting schema structure integrity
- **Rationale**: Transforms need to update computed field values as part of their execution, but should not be able to modify schema definitions
- **Access Rules**:
  - Transforms can write/update field values in any schema regardless of approval state
  - Transforms cannot modify schema structure (add/remove fields, change field types)
  - Field value updates are treated as data operations, not schema modifications
  - Schema immutability applies only to structural changes, not field value updates
- **Implementation Notes**:
  - Backend should distinguish between schema structure modifications and field value updates
  - Transform execution should use field value update operations, not schema modification operations
  - Error handling should clearly differentiate between these operation types

### LOG-001: File Size Parsing for Logging
- **Description**: Defines how file size strings are interpreted for log rotation
- **Rationale**: Ensures consistent configuration and prevents invalid log rotation sizes
- **Accepted Formats**:
  - Numeric value only (bytes)
  - Numbers ending with `KB`, `MB`, or `GB`
- **Implementation Notes**:
  - Parsing logic resides in `logging::utils::parse_file_size`
  - Both configuration validation and file output initialization use this shared parser
