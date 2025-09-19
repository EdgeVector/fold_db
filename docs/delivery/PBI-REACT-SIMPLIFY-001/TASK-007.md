# TASK-007: Legacy Code Removal and Cleanup

[Back to task list](./tasks.md)

## Description

Remove all legacy files and unused imports that are no longer needed after the React simplification initiative. This task focuses on identifying and safely removing deprecated components, utilities, patterns, API client files, and configuration files that have been replaced by the new modular architecture.

The cleanup will target deprecated files identified during the refactoring process, unused utility functions, obsolete API endpoints, and configuration files that are no longer relevant to the simplified architecture.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 19:24:00 | Created | N/A | Proposed | Task file created for legacy code cleanup | System |

## Requirements

### Core Requirements
- Identify and remove all unused legacy files from the React application
- Remove deprecated components that have been replaced by new modular components
- Clean up unused imports and dependencies throughout the codebase
- Remove obsolete API client files and unused endpoint configurations
- Maintain SCHEMA-002 compliance throughout the cleanup process

### Required Constants (Section 2.1.12)
```typescript
const CLEANUP_BATCH_SIZE = 10;
const DEPENDENCY_SCAN_TIMEOUT_MS = 30000;
const LEGACY_FILE_AGE_DAYS = 30;
const UNUSED_IMPORT_THRESHOLD = 0;
const CLEANUP_VALIDATION_TIMEOUT_MS = 60000;
```

### DRY Compliance Requirements
- Remove all duplicate utility functions that have been consolidated
- Eliminate redundant API client patterns replaced by unified client
- Remove duplicate constants that have been centralized
- Clean up repeated validation logic replaced by hooks

### SCHEMA-002 Compliance
- Verify that legacy schema access patterns are completely removed
- Ensure no deprecated schema validation logic remains
- Confirm all schema operations use approved-only access patterns
- Validate that legacy schema state management is eliminated

## Implementation Plan

### Phase 1: Legacy File Identification
1. **Automated Dependency Analysis**
   - Scan entire React codebase for unused imports using `DEPENDENCY_SCAN_TIMEOUT_MS`
   - Identify files with no references using static analysis
   - Generate list of potentially obsolete files for manual review
   - Document files that were replaced during refactoring

2. **Legacy Component Identification**
   - Identify components replaced by new modular components
   - Locate utility functions superseded by custom hooks
   - Find API clients replaced by unified client
   - Mark deprecated patterns for removal

### Phase 2: Safe Removal Process
1. **Pre-Removal Validation**
   - Verify files are truly unused through comprehensive grep search
   - Check git history to understand file purpose and replacement
   - Validate no dynamic imports or runtime references exist
   - Confirm test coverage for remaining functionality

2. **Batch Removal Implementation**
   - Remove files in batches of `CLEANUP_BATCH_SIZE` for safe validation
   - Run full test suite after each batch removal
   - Validate application functionality with `CLEANUP_VALIDATION_TIMEOUT_MS`
   - Document removal reasons and replacement components

### Phase 3: Import and Dependency Cleanup
1. **Unused Import Removal**
   - Use ESLint unused-imports rule to identify unused imports
   - Remove imports with `UNUSED_IMPORT_THRESHOLD` usage count
   - Clean up package.json dependencies no longer needed
   - Update import paths to use new modular structure

2. **API Endpoint Cleanup**
   - Remove unused API endpoint configurations
   - Clean up obsolete authentication wrapper implementations
   - Remove deprecated response type definitions
   - Update API documentation to reflect current endpoints

### Phase 4: Configuration and Constants Cleanup
1. **Legacy Configuration Removal**
   - Remove obsolete configuration files
   - Clean up environment variables no longer used
   - Remove deprecated build configurations
   - Update deployment scripts to remove legacy references

2. **Constants Consolidation Verification**
   - Verify all magic numbers have been properly extracted
   - Remove duplicate constant definitions
   - Ensure consistent naming conventions
   - Validate constant usage across components

## Verification

### Cleanup Validation Requirements
- [ ] No unused files remain in the React application directory
- [ ] All imports resolve correctly and are actually used
- [ ] No deprecated components or utilities are referenced
- [ ] Package.json contains only necessary dependencies
- [ ] Application builds and runs without errors after cleanup

### Functional Testing Requirements
- [ ] All existing features continue to work after cleanup
- [ ] No broken links or missing modules
- [ ] API clients function correctly with current endpoints
- [ ] Authentication flows work with updated configurations
- [ ] Schema operations maintain SCHEMA-002 compliance

### Performance Requirements
- [ ] Bundle size reduced by at least 15% after cleanup
- [ ] Build time improved or maintained after dependency cleanup
- [ ] Application startup time not negatively impacted
- [ ] Memory usage reduced with removal of unused code

### Documentation Requirements
- [ ] Document all removed files and their replacements
- [ ] Update README to reflect cleaned architecture
- [ ] Remove references to deprecated components from docs
- [ ] Update API documentation for current endpoints only

## Files Modified

### Removed Files (Examples)
- `src/datafold_node/static-react/src/utils/legacySchemaUtils.js` - Replaced by useApprovedSchemas hook
- `src/datafold_node/static-react/src/api/legacyMutationClient.ts` - Replaced by unified API client
- `src/datafold_node/static-react/src/components/legacy/` - Entire directory of deprecated components
- `src/datafold_node/static-react/src/constants/deprecated.js` - Obsolete constants

### Modified Files
- `src/datafold_node/static-react/package.json` - Remove unused dependencies
- `src/datafold_node/static-react/src/App.jsx` - Remove legacy imports
- `src/datafold_node/static-react/.eslintrc.js` - Update rules for cleaned codebase
- `src/datafold_node/static-react/webpack.config.js` - Remove legacy build configurations

### Documentation Updates
- `docs/ui/static-react/overview.md` - Remove references to deleted files
- `docs/ui/static-react/cleanup-log.md` - Document cleanup decisions

## Rollback Plan

If issues arise during legacy code cleanup:

1. **Immediate Restoration**: Use git to restore accidentally removed files
2. **Batch Rollback**: Revert cleanup batches one at a time to identify issues
3. **Dependency Restoration**: Restore package.json if dependency removal causes issues
4. **Configuration Rollback**: Restore configuration files if deployment issues occur
5. **Validation Testing**: Run comprehensive test suite after any rollback operation