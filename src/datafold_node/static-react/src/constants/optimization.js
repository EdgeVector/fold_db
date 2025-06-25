/**
 * Optimization Constants
 * TASK-009: Additional Simplification Opportunities  
 * Required per Section 2.1.12 of .cursorrules
 */

// Component complexity analysis thresholds
export const COMPLEXITY_SCORE_THRESHOLD = 15;
export const PROP_INTERFACE_MAX_PROPS = 8;
export const COMPONENT_MAX_LINES = 200;

// Abstraction analysis thresholds  
export const ABSTRACTION_JUSTIFICATION_THRESHOLD = 3;

// Validation and timeout configurations
export const OPTIMIZATION_VALIDATION_TIMEOUT_MS = 30000;

// Complexity scoring weights for analysis
export const COMPLEXITY_WEIGHT_CYCLOMATIC = 1;
export const COMPLEXITY_WEIGHT_CONDITIONAL_RENDER = 2;
export const COMPLEXITY_WEIGHT_NESTED_DEPTH = 1.5;
export const COMPLEXITY_WEIGHT_PROP_COUNT = 0.5;

// Code quality metrics
export const MAX_FUNCTION_LINES = 50;
export const MAX_JSX_NESTING_DEPTH = 5;
export const MAX_USEEFFECT_DEPENDENCIES = 5;
export const MAX_CONDITIONAL_BRANCHES = 3;

// Performance optimization thresholds
export const BUNDLE_SIZE_WARNING_THRESHOLD_KB = 500;
export const RENDER_PERFORMANCE_WARNING_MS = 16; // One frame at 60fps
export const MEMORY_LEAK_DETECTION_THRESHOLD = 10;