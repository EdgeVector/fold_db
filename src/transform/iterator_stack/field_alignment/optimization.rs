//! Optimization suggestions for field alignments
//!
//! Contains logic for analyzing field alignments and suggesting
//! performance optimizations.

use crate::transform::iterator_stack::chain_parser::FieldAlignment;
use crate::transform::iterator_stack::field_alignment::types::{
    FieldAlignmentInfo, FieldAlignmentValidator, OptimizationSuggestion, OptimizationType,
};
use std::collections::HashMap;

impl FieldAlignmentValidator {
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
