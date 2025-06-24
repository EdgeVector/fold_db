# Project Logic

This document contains the most up-to-date and condensed information about the project's logical processes, built up over time. Each logic entry tracks which modules it applies to and any conflicts with other logic.

## Logic Table

| Logic_ID | Logic | Module | Updated At | Conflicts with |
|----------|-------|--------|------------|----------------|
| SCHEMA-001 | Schemas cannot go from approved to unloaded. They can only go from approved to blocked, and then from blocked back to approved. | schema_manager, schema_routes | 2025-06-23 19:08:00 | None |

## Logic Details

### SCHEMA-001: Schema State Transition Rules
- **Description**: Enforces valid state transitions for schema lifecycle management
- **Rationale**: Prevents data loss and ensures proper workflow by requiring schemas to go through a blocking state before becoming unloaded
- **Valid Transitions**: 
  - approved → blocked
  - blocked → approved
  - blocked → unloaded (implied, but not approved → unloaded directly)
- **Invalid Transitions**:
  - approved → unloaded (direct transition prohibited)