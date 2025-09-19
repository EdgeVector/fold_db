# SKC-7-3: Document universal key aggregation behavior

[Back to task list](./tasks.md)

## Description

Update delivery and technical documentation so developers understand how the
aggregation utilities leverage universal key configuration. Clarify output
formatting, troubleshooting steps, and relationships to existing project logic
entries governing universal key adoption.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-20 10:10:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

### Content Requirements
- Summarize the new aggregation workflow in `docs/design/iterator_stack_quick_reference.md`
  (or a more appropriate design reference) with emphasis on universal key usage.
- Document result shaping expectations and error handling guidance in the SKC-7 PRD
  and/or supplemental delivery notes as needed.
- Update `docs/project_logic.md` to reflect that aggregation utilities now comply
  with universal key integration (adjust existing logic entry or add a new one).
- Cross-reference related SKC PBIs to maintain a consistent knowledge chain.

### Quality Requirements
- Follow existing documentation style and formatting conventions (headings,
  tables, code fences).
- Avoid duplicating information already stored in canonical sources; link instead.
- Ensure terminology matches code implementation (e.g., `{ hash, range, fields }`).
- Provide troubleshooting tips for common misconfiguration scenarios.

### Dependencies
- Completion of implementation updates from SKC-7-1 so documentation reflects the
  final behavior.
- Coordination with SKC-7-2 test coverage to include references to new tests when
  appropriate.

## Implementation Plan

### Step 1: Inventory affected documentation
- Review existing sections discussing aggregation within `docs/design/iterator_stack_quick_reference.md`
  and other transform design docs to identify update locations.
- Examine prior SKC documentation (SKC-1 through SKC-6) for reusable explanations
  of universal key handling to reference instead of duplicating content.

### Step 2: Draft updated design notes
- Describe how aggregation consumes universal key metadata, including the switch
  to `shape_unified_result()` and dotted key support.
- Outline the expected `{ hash, range, fields }` output shape and how legacy Range
  schemas remain supported.
- Document error handling patterns developers should expect when key configuration
  is incomplete or invalid.

### Step 3: Update project logic and delivery artifacts
- Amend `docs/project_logic.md` entry `SCHEMA-KEY-004` (or create a new logic entry)
  to note that aggregation utilities now satisfy universal key requirements.
- Add a brief summary of documentation changes to `docs/delivery/SKC-7/prd.md`
  under Open Questions or a new Notes section if clarification is helpful.

### Step 4: Review for consistency
- Proofread updates for clarity and alignment with repository documentation
  standards.
- Verify Markdown renders correctly (tables, links, code fences).

## Verification

### Acceptance Criteria
- [ ] Design documentation describes universal key-driven aggregation behavior
      and references `shape_unified_result()`.
- [ ] Project logic table reflects aggregation compliance with universal key
      integration.
- [ ] Delivery documentation links to updated tests and implementation details.
- [ ] Documentation follows established style and avoids duplication.

### Test Plan
1. Build or preview Markdown locally to ensure formatting renders as expected
   (e.g., use VS Code preview or `markdownlint` if available).
2. Cross-check links within updated documents to confirm they resolve to valid
   files/sections.
3. Perform a spellcheck pass or manual proofreading to maintain documentation
   quality.

## Files Modified

- `docs/design/iterator_stack_quick_reference.md`
- `docs/project_logic.md`
- `docs/delivery/SKC-7/prd.md`
- Additional delivery or reference docs as needed for cross-links
