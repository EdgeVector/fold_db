//! Field alignment validation for iterator stack model
//!
//! Ensures all fields are properly aligned relative to the deepest iterator
//! using 1:1, broadcast, and reduced alignment rules.

use crate::transform::iterator_stack::chain_parser::{FieldAlignment, FieldAlignmentRequirement, ParsedChain};
use crate::transform::iterator_stack::errors::IteratorStackResult;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Validates field alignment across multiple chains in a schema
pub struct FieldAlignmentValidator {
    /// Maximum allowed depth
    max_depth: usize,
    /// Whether to allow reducer functions
    allow_reducers: bool,
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

impl Default for FieldAlignmentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldAlignmentValidator {
    /// Creates a new field alignment validator
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            allow_reducers: true,
        }
    }

    /// Creates a validator with custom configuration
    pub fn with_config(max_depth: usize, allow_reducers: bool) -> Self {
        Self {
            max_depth,
            allow_reducers,
        }
    }

    /// Validates field alignment across multiple parsed chains
    pub fn validate_alignment(
        &self,
        chains: &[ParsedChain],
    ) -> IteratorStackResult<AlignmentValidationResult> {
        if chains.is_empty() {
            return Ok(AlignmentValidationResult {
                valid: true,
                max_depth: 0,
                field_alignments: HashMap::new(),
                errors: Vec::new(),
                warnings: Vec::new(),
            });
        }

        let max_depth = chains.iter().map(|c| c.depth).max().unwrap_or(0);
        let mut field_alignments = HashMap::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        // Validate depth constraints
        self.validate_depth_constraints(chains, &mut errors);

        // Validate branch compatibility
        self.validate_branch_compatibility(chains, &mut errors);

        // Generate field alignment information
        for chain in chains {
            let alignment_info = self.generate_alignment_info(chain, max_depth)?;
            field_alignments.insert(chain.expression.clone(), alignment_info);
        }

        // Check for reducer requirements
        self.validate_reducer_requirements(&field_alignments, &mut errors, &mut warnings);

        // Generate performance warnings
        self.generate_performance_warnings(chains, max_depth, &mut warnings);

        let valid = errors.is_empty();

        Ok(AlignmentValidationResult {
            valid,
            max_depth,
            field_alignments,
            errors,
            warnings,
        })
    }

    /// Validates that all fields respect depth constraints
    fn validate_depth_constraints(
        &self,
        chains: &[ParsedChain],
        errors: &mut Vec<AlignmentError>,
    ) {
        for chain in chains {
            if chain.depth > self.max_depth {
                errors.push(AlignmentError {
                    error_type: AlignmentErrorType::DepthExceeded,
                    message: format!(
                        "Field '{}' depth {} exceeds maximum allowed depth {}",
                        chain.expression, chain.depth, self.max_depth
                    ),
                    fields: vec![chain.expression.clone()],
                });
            }
        }
    }

    /// Validates that branches are compatible (no cartesian products)
    fn validate_branch_compatibility(
        &self,
        chains: &[ParsedChain],
        errors: &mut Vec<AlignmentError>,
    ) {
        // Find the maximum depth for comparison
        let max_depth = chains.iter().map(|c| c.depth).max().unwrap_or(0);

        // Group chains by depth and use proper branch extraction
        let mut depth_branches: HashMap<usize, HashSet<String>> = HashMap::new();
        let mut depth_fields: HashMap<usize, Vec<String>> = HashMap::new();

        // Validate branch compatibility (no cartesian products)
        for chain in chains.iter() {
            if chain.depth > 0 {
                // Extract branch up to the maximum depth for proper comparison
                let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
                match parser.extract_branch_up_to_depth(&chain.operations, max_depth) {
                    Ok(branch_at_max_depth) => {
                        depth_branches
                            .entry(chain.depth)
                            .or_default()
                            .insert(branch_at_max_depth.clone());

                        depth_fields
                            .entry(chain.depth)
                            .or_default()
                            .push(chain.expression.clone());
                    }
                    Err(_) => {
                        // If we can't extract the branch, add an error and skip this chain
                        errors.push(AlignmentError {
                            error_type: AlignmentErrorType::CartesianProduct,
                            message: format!(
                                "Failed to extract branch for field '{}' at depth {}",
                                chain.expression, chain.depth
                            ),
                            fields: vec![chain.expression.clone()],
                        });
                    }
                }
            }
        }


        // Check for incompatible branches at the same depth
        for (depth, branches) in &depth_branches {
            if branches.len() > 1 {
                let branch_list: Vec<String> = branches.iter().cloned().collect();

                // Check if branches diverge at this depth (creating cartesian product)
                // Two branches are incompatible if they have different paths at the same depth level
                let mut has_incompatible_branches = false;
                let mut common_prefix: Option<String> = None;

                for branch in branches {
                    // Find the common prefix among all branches at this depth
                    if common_prefix.is_none() {
                        common_prefix = Some(branch.clone());
                    } else {
                        let current_prefix = common_prefix.as_ref().unwrap();
                        // Find common prefix between current branch and existing prefix
                        let mut new_prefix = String::new();
                        let parts1: Vec<&str> = current_prefix.split('.').collect();
                        let parts2: Vec<&str> = branch.split('.').collect();

                        for (i, (p1, p2)) in parts1.iter().zip(parts2.iter()).enumerate() {
                            if p1 == p2 {
                                if i > 0 {
                                    new_prefix.push('.');
                                }
                                new_prefix.push_str(p1);
                            } else {
                                break;
                            }
                        }

                        common_prefix = Some(new_prefix);
                    }
                }

                // If common prefix is shorter than the depth, branches diverge and are incompatible
                if let Some(prefix) = &common_prefix {
                    let prefix_depth = prefix.split('.').count();
                    if prefix_depth < *depth {
                        has_incompatible_branches = true;
                    }
                }

                if has_incompatible_branches {
                    let field_list = depth_fields.get(depth).unwrap_or(&Vec::new()).clone();

                    // Branches diverge and would create a cartesian product
                    errors.push(AlignmentError {
                        error_type: AlignmentErrorType::CartesianProduct,
                        message: format!(
                            "Incompatible branches at depth {}: {}. Branches diverge and would create a cartesian product.",
                            depth,
                            branch_list.join(", ")
                        ),
                        fields: field_list,
                    });
                } else {
                    // Branches are compatible - no cartesian product
                }
            }
        }
    }

    /// Generates alignment information for a single chain
    fn generate_alignment_info(
        &self,
        chain: &ParsedChain,
        max_depth: usize,
    ) -> IteratorStackResult<FieldAlignmentInfo> {
        let alignment = match chain.depth.cmp(&max_depth) {
            std::cmp::Ordering::Equal => FieldAlignment::OneToOne,
            std::cmp::Ordering::Less => FieldAlignment::Broadcast,
            std::cmp::Ordering::Greater => FieldAlignment::Reduced,
        };


        // Suggest reducers for chains at the depth limit to optimize performance
        // Note: validate_depth_constraints already rejects chains with depth > max_depth
        let requires_reducer = chain.depth == max_depth;
        let suggested_reducer = if requires_reducer {
            Some(self.suggest_reducer_for_chain(chain))
        } else {
            None
        };

        Ok(FieldAlignmentInfo {
            expression: chain.expression.clone(),
            depth: chain.depth,
            alignment,
            branch: chain.branch.clone(),
            requires_reducer,
            suggested_reducer,
        })
    }

    /// Suggests an appropriate reducer function for a chain
    fn suggest_reducer_for_chain(&self, chain: &ParsedChain) -> String {
        // Analyze the chain to suggest the most appropriate reducer
        let operations = &chain.operations;
        
        for operation in operations {
            match operation {
                crate::transform::iterator_stack::chain_parser::ChainOperation::SplitArray => {
                    return "join(',')".to_string();
                }
                crate::transform::iterator_stack::chain_parser::ChainOperation::SplitByWord => {
                    return "join(' ')".to_string();
                }
                crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(field) => {
                    if field.contains("count") || field.contains("size") {
                        return "count()".to_string();
                    }
                    if field.contains("first") {
                        return "first()".to_string();
                    }
                    if field.contains("last") {
                        return "last()".to_string();
                    }
                }
                _ => continue,
            }
        }

        // Default suggestion
        "first()".to_string()
    }

    /// Validates reducer requirements and adds appropriate errors/warnings
    fn validate_reducer_requirements(
        &self,
        field_alignments: &HashMap<String, FieldAlignmentInfo>,
        errors: &mut Vec<AlignmentError>,
        warnings: &mut Vec<AlignmentWarning>,
    ) {
        for (field_name, alignment_info) in field_alignments {
            if alignment_info.requires_reducer && !self.allow_reducers {
                errors.push(AlignmentError {
                    error_type: AlignmentErrorType::DepthExceeded,
                    message: format!(
                        "Field '{}' requires a reducer but reducers are not allowed",
                        field_name
                    ),
                    fields: vec![field_name.clone()],
                });
            } else if alignment_info.requires_reducer {
                warnings.push(AlignmentWarning {
                    warning_type: AlignmentWarningType::OptimizationHint,
                    message: format!(
                        "Field '{}' should use reducer '{}' to improve performance",
                        field_name,
                        alignment_info.suggested_reducer.as_ref().unwrap_or(&"first()".to_string())
                    ),
                    fields: vec![field_name.clone()],
                });
            }
        }
    }

    /// Generates performance-related warnings
    fn generate_performance_warnings(
        &self,
        chains: &[ParsedChain],
        max_depth: usize,
        warnings: &mut Vec<AlignmentWarning>,
    ) {
        // Warn about deep nesting
        if max_depth > 5 {
            let deep_fields: Vec<String> = chains
                .iter()
                .filter(|c| c.depth > 5)
                .map(|c| c.expression.clone())
                .collect();

            if !deep_fields.is_empty() {
                warnings.push(AlignmentWarning {
                    warning_type: AlignmentWarningType::PerformanceConcern,
                    message: format!(
                        "Deep iterator nesting (depth {}) may impact performance",
                        max_depth
                    ),
                    fields: deep_fields,
                });
            }
        }

        // Warn about many broadcast fields
        let broadcast_count = chains
            .iter()
            .filter(|c| c.depth < max_depth)
            .count();

        if broadcast_count > 10 {
            let broadcast_fields: Vec<String> = chains
                .iter()
                .filter(|c| c.depth < max_depth)
                .map(|c| c.expression.clone())
                .collect();

            warnings.push(AlignmentWarning {
                warning_type: AlignmentWarningType::MemoryUsage,
                message: format!(
                    "Many broadcast fields ({}) may increase memory usage",
                    broadcast_count
                ),
                fields: broadcast_fields,
            });
        }
    }

    /// Validates a set of field alignment requirements
    pub fn validate_requirements(
        &self,
        requirements: &[FieldAlignmentRequirement],
    ) -> IteratorStackResult<AlignmentValidationResult> {
        // Convert requirements to chains for validation
        let chains: Vec<ParsedChain> = requirements
            .iter()
            .map(|req| ParsedChain {
                expression: req.field_expression.clone(),
                operations: Vec::new(), // We don't have operations from requirements
                depth: req.depth,
                branch: req.branch.clone(),
                scopes: Vec::new(),
            })
            .collect();

        self.validate_alignment(&chains)
    }

    /// Suggests optimization for a set of field alignments
    pub fn suggest_optimizations(
        &self,
        field_alignments: &HashMap<String, FieldAlignmentInfo>,
    ) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Group fields by alignment type
        let mut one_to_one_fields = Vec::new();
        let mut broadcast_fields = Vec::new();
        let mut reduced_fields = Vec::new();

        for (field_name, info) in field_alignments {
            match info.alignment {
                FieldAlignment::OneToOne => one_to_one_fields.push(field_name.clone()),
                FieldAlignment::Broadcast => broadcast_fields.push(field_name.clone()),
                FieldAlignment::Reduced => reduced_fields.push(field_name.clone()),
            }
        }

        // Suggest reducing broadcast fields if too many
        if broadcast_fields.len() > 5 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationType::ReduceBroadcast,
                message: "Consider reducing the number of broadcast fields to improve memory usage".to_string(),
                affected_fields: broadcast_fields,
                estimated_benefit: "Reduced memory usage and improved cache efficiency".to_string(),
            });
        }

        // Suggest using reducers for fields that exceed optimal depth
        if !reduced_fields.is_empty() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationType::UseReducers,
                message: "Use reducer functions to optimize deep field access".to_string(),
                affected_fields: reduced_fields,
                estimated_benefit: "Improved performance and reduced complexity".to_string(),
            });
        }

        suggestions
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::iterator_stack::chain_parser::ChainParser;

    #[test]
    fn test_simple_alignment_validation() {
        let validator = FieldAlignmentValidator::new();
        let parser = ChainParser::new();

        let chain1 = parser.parse("blogpost.map().content").unwrap();
        let chain2 = parser.parse("blogpost.map().author").unwrap();

        let result = validator.validate_alignment(&[chain1, chain2]).unwrap();

        assert!(result.valid);
        assert_eq!(result.max_depth, 1);
        assert_eq!(result.field_alignments.len(), 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_broadcast_alignment() {
        let validator = FieldAlignmentValidator::new();
        let parser = ChainParser::new();

        let chain1 = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        let chain2 = parser.parse("blogpost.map().publish_date").unwrap();

        let result = validator.validate_alignment(&[chain1, chain2]).unwrap();

        assert!(result.valid);
        assert_eq!(result.max_depth, 2);

        // Check alignments
        let content_alignment = result.field_alignments.get("blogpost.map().content.split_by_word().map()").unwrap();
        assert_eq!(content_alignment.alignment, FieldAlignment::OneToOne);

        let date_alignment = result.field_alignments.get("blogpost.map().publish_date").unwrap();
        assert_eq!(date_alignment.alignment, FieldAlignment::Broadcast);
    }

    #[test]
    fn test_cartesian_product_detection() {
        let validator = FieldAlignmentValidator::new();
        let parser = ChainParser::new();

        let chain1 = parser.parse("blogpost.map().tags.split_array().map()").unwrap();
        let chain2 = parser.parse("blogpost.map().comments.map()").unwrap();

        let result = validator.validate_alignment(&[chain1, chain2]).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert!(matches!(
            result.errors[0].error_type,
            AlignmentErrorType::CartesianProduct
        ));
    }

    #[test]
    fn test_depth_exceeded_validation() {
        let validator = FieldAlignmentValidator::with_config(2, false);
        let parser = ChainParser::new();

        let chain = parser.parse("blogpost.map().content.split_by_word().map().split_array().map()").unwrap();

        let result = validator.validate_alignment(&[chain]).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert!(matches!(
            result.errors[0].error_type,
            AlignmentErrorType::DepthExceeded
        ));
    }

    #[test]
    fn test_reducer_suggestions() {
        let validator = FieldAlignmentValidator::new();
        let parser = ChainParser::new();

        let chain1 = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        let chain2 = parser.parse("blogpost.map().tags.split_array().map().split_by_word().map()").unwrap();

        let result = validator.validate_alignment(&[chain1, chain2]).unwrap();

        assert!(result.valid);

        // With the updated logic, chains at the depth limit should require reducers for optimization
        // - chain1: depth=2, max_depth=3 -> Broadcast (no reducer needed)
        // - chain2: depth=3, max_depth=3 -> OneToOne (reducer suggested for optimization)
        let tags_alignment = result.field_alignments.get("blogpost.map().tags.split_array().map().split_by_word().map()").unwrap();
        assert_eq!(tags_alignment.alignment, FieldAlignment::OneToOne); // Correct based on current logic
        assert!(tags_alignment.requires_reducer); // OneToOne at depth limit should suggest reducer for optimization
        assert!(tags_alignment.suggested_reducer.is_some()); // Reducer suggestion should be provided
    }

    #[test]
    fn test_optimization_suggestions() {
        let validator = FieldAlignmentValidator::new();
        
        let mut field_alignments = HashMap::new();
        
        // Create many broadcast fields
        for i in 0..10 {
            field_alignments.insert(
                format!("field{}", i),
                FieldAlignmentInfo {
                    expression: format!("blogpost.map().field{}", i),
                    depth: 0,
                    alignment: FieldAlignment::Broadcast,
                    branch: "blogpost".to_string(),
                    requires_reducer: false,
                    suggested_reducer: None,
                },
            );
        }

        let suggestions = validator.suggest_optimizations(&field_alignments);
        assert!(!suggestions.is_empty());
        assert!(matches!(
            suggestions[0].suggestion_type,
            OptimizationType::ReduceBroadcast
        ));
    }
}