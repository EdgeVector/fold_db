/**
 * Legacy Code Cleanup Constants
 * TASK-007: Legacy Code Removal and Cleanup
 * TASK-008: Duplicate Code Detection and Elimination
 * Required per Section 2.1.12 of .cursorrules
 */

// Cleanup batch configuration
export const CLEANUP_BATCH_SIZE = 10;
export const DEPENDENCY_SCAN_TIMEOUT_MS = 30000;
export const LEGACY_FILE_AGE_DAYS = 30;
export const UNUSED_IMPORT_THRESHOLD = 0;
export const CLEANUP_VALIDATION_TIMEOUT_MS = 60000;

// TASK-008: Duplicate Code Detection Constants
export const CODE_SIMILARITY_THRESHOLD_PERCENT = 80;
export const DUPLICATE_DETECTION_BATCH_SIZE = 20;
export const CONSOLIDATION_VALIDATION_TIMEOUT_MS = 45000;
export const PATTERN_ANALYSIS_DEPTH = 5;
export const DUPLICATE_LINE_THRESHOLD = 10;