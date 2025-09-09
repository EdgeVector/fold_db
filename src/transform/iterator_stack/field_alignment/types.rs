//! Type definitions for field alignment validation
//!
//! Contains all data structures, enums, and result types used in field alignment
//! validation and optimization.

use crate::transform::iterator_stack::chain_parser::FieldAlignment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validates field alignment across multiple chains in a schema
pub struct FieldAlignmentValidator {
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Whether to allow reducer functions
    pub allow_reducers: bool,
}

/// Result of field alignment validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentValidationResult {
    /// Whether all fields are properly aligned
    pub valid: bool,
    /// Maximum depth across all fields
    pub max_depth: usize,
    /// Field alignment assignments
    pub field_alignments: HashMap<String, FieldAlignmentInfo>,
    /// Validation errors found
    pub errors: Vec<AlignmentError>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<AlignmentWarning>,
}

/// Information about how a field should be aligned
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAlignmentInfo {
    /// Field expression
    pub expression: String,
    /// Iterator depth of this field
    pub depth: usize,
    /// Required alignment type
    pub alignment: FieldAlignment,
    /// Branch identifier
    pub branch: String,
    /// Whether a reducer is required
    pub requires_reducer: bool,
    /// Suggested reducer function if needed
    pub suggested_reducer: Option<String>,
}

/// Alignment validation error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentError {
    /// Error type
    pub error_type: AlignmentErrorType,
    /// Human-readable message
    pub message: String,
    /// Fields involved in the error
    pub fields: Vec<String>,
}

/// Types of alignment errors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentErrorType {
    /// Incompatible fan-out depths
    IncompatibleDepths,
    /// Cartesian product (different branches at same depth)
    CartesianProduct,
    /// Field exceeds max depth without reducer
    DepthExceeded,
    /// Invalid field alignment configuration
    InvalidAlignment,
}

/// Alignment validation warning
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentWarning {
    /// Warning type
    pub warning_type: AlignmentWarningType,
    /// Human-readable message
    pub message: String,
    /// Fields involved in the warning
    pub fields: Vec<String>,
}

/// Types of alignment warnings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentWarningType {
    /// Performance concern with deep nesting
    PerformanceConcern,
    /// Memory usage warning
    MemoryUsage,
    /// Recommended optimization
    OptimizationHint,
}

/// Optimization suggestion for field alignments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Type of optimization
    pub suggestion_type: OptimizationType,
    /// Human-readable message
    pub message: String,
    /// Fields that would be affected
    pub affected_fields: Vec<String>,
    /// Expected benefit from applying this optimization
    pub estimated_benefit: String,
}

/// Types of optimization suggestions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationType {
    /// Reduce number of broadcast fields
    ReduceBroadcast,
    /// Use reducer functions
    UseReducers,
    /// Restructure field access patterns
    RestructureAccess,
    /// Cache expensive computations
    CacheComputations,
}
