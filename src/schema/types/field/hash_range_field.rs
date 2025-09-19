//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;
use crate::transform::iterator_stack::{
    chain_parser::{ChainParser, ParsedChain},
    errors::IteratorStackResult,
    execution_engine::ExecutionEngine,
    field_alignment::FieldAlignmentValidator,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeField {
    pub inner: FieldCommon,
    /// Expression for the hash field (used for indexing)
    pub hash_field: String,
    /// Expression for the range field (used for sorting/filtering)
    pub range_field: String,
    /// Expression for the atom UUID field
    pub atom_uuid: String,
    /// Cached parsed chains for performance
    #[serde(skip)]
    pub cached_chains: Option<HashRangeChains>,
}

/// Cached parsed chains for a HashRange field
#[derive(Debug, Clone)]
pub struct HashRangeChains {
    hash_chain: ParsedChain,
    range_chain: ParsedChain,
    atom_uuid_chain: ParsedChain,
}

/// Configuration for HashRange field indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeConfig {
    /// Maximum iterator depth allowed
    pub max_depth: usize,
    /// Whether to enable caching of parsed chains
    pub enable_caching: bool,
}

impl Default for HashRangeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            enable_caching: true,
        }
    }
}

impl HashRangeField {
    /// Creates a new HashRange field
    #[must_use]
    pub fn new(
        permission_policy: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        field_mappers: HashMap<String, String>,
        hash_field: String,
        range_field: String,
        atom_uuid: String,
    ) -> Self {
        Self {
            inner: FieldCommon::new(permission_policy, payment_config, field_mappers),
            hash_field,
            range_field,
            atom_uuid,
            cached_chains: None,
        }
    }

    /// Gets the hash field expression
    pub fn hash_field(&self) -> &str {
        &self.hash_field
    }

    /// Gets the range field expression
    pub fn range_field(&self) -> &str {
        &self.range_field
    }

    /// Gets the atom UUID expression
    pub fn atom_uuid_field(&self) -> &str {
        &self.atom_uuid
    }

    /// Sets the hash field expression
    pub fn set_hash_field(&mut self, hash_field: String) {
        self.hash_field = hash_field;
        self.cached_chains = None; // Invalidate cache
    }

    /// Sets the range field expression
    pub fn set_range_field(&mut self, range_field: String) {
        self.range_field = range_field;
        self.cached_chains = None; // Invalidate cache
    }

    /// Sets the atom UUID field expression
    pub fn set_atom_uuid_field(&mut self, atom_uuid: String) {
        self.atom_uuid = atom_uuid;
        self.cached_chains = None; // Invalidate cache
    }

    /// Validates the field expressions using the iterator stack model
    pub fn validate_expressions(&mut self) -> IteratorStackResult<()> {
        let chains = self.get_or_parse_chains()?;

        let validator = FieldAlignmentValidator::new();
        let all_chains = vec![
            chains.hash_chain.clone(),
            chains.range_chain.clone(),
            chains.atom_uuid_chain.clone(),
        ];

        let alignment_result = validator.validate_alignment(&all_chains)?;

        if !alignment_result.valid {
            return Err(
                crate::transform::iterator_stack::errors::IteratorStackError::FieldAlignmentError {
                    field: "HashRange".to_string(),
                    reason: format!(
                        "Field alignment validation failed: {:?}",
                        alignment_result.errors
                    ),
                },
            );
        }

        Ok(())
    }

    /// Executes the field expressions and generates index entries
    pub fn execute_indexing(&mut self, input_data: Value) -> IteratorStackResult<Vec<IndexEntry>> {
        let chains = self.get_or_parse_chains()?;

        let validator = FieldAlignmentValidator::new();
        let all_chains = vec![
            chains.hash_chain.clone(),
            chains.range_chain.clone(),
            chains.atom_uuid_chain.clone(),
        ];

        let alignment_result = validator.validate_alignment(&all_chains)?;

        if !alignment_result.valid {
            return Err(
                crate::transform::iterator_stack::errors::IteratorStackError::ExecutionError {
                    message: "Cannot execute with invalid field alignment".to_string(),
                },
            );
        }

        let mut engine = ExecutionEngine::new();
        let execution_result = engine.execute_fields(&all_chains, &alignment_result, input_data)?;

        // Convert execution result to index entries
        let index_entries: Vec<IndexEntry> = execution_result
            .index_entries
            .into_iter()
            .map(|entry| IndexEntry {
                hash_value: entry.hash_value,
                range_value: entry.range_value,
                atom_uuid: entry.atom_uuid,
                metadata: entry.metadata,
            })
            .collect();

        Ok(index_entries)
    }

    /// Gets or parses the field chains, using cache when available
    fn get_or_parse_chains(&mut self) -> IteratorStackResult<&HashRangeChains> {
        if self.cached_chains.is_none() {
            let parser = ChainParser::new();

            let hash_chain = parser.parse(&self.hash_field)?;
            let range_chain = parser.parse(&self.range_field)?;
            let atom_uuid_chain = parser.parse(&self.atom_uuid)?;

            self.cached_chains = Some(HashRangeChains {
                hash_chain,
                range_chain,
                atom_uuid_chain,
            });
        }

        Ok(self.cached_chains.as_ref().unwrap())
    }

    /// Gets information about the field's iterator stack structure
    pub fn get_stack_info(&mut self) -> IteratorStackResult<HashRangeStackInfo> {
        let chains = self.get_or_parse_chains()?;

        let validator = FieldAlignmentValidator::new();
        let all_chains = vec![
            chains.hash_chain.clone(),
            chains.range_chain.clone(),
            chains.atom_uuid_chain.clone(),
        ];

        let alignment_result = validator.validate_alignment(&all_chains)?;

        Ok(HashRangeStackInfo {
            max_depth: alignment_result.max_depth,
            hash_depth: chains.hash_chain.depth,
            range_depth: chains.range_chain.depth,
            atom_uuid_depth: chains.atom_uuid_chain.depth,
            hash_alignment: alignment_result
                .field_alignments
                .get(&chains.hash_chain.expression)
                .map(|info| info.alignment.clone()),
            range_alignment: alignment_result
                .field_alignments
                .get(&chains.range_chain.expression)
                .map(|info| info.alignment.clone()),
            atom_uuid_alignment: alignment_result
                .field_alignments
                .get(&chains.atom_uuid_chain.expression)
                .map(|info| info.alignment.clone()),
            compatible: alignment_result.valid,
            warnings: alignment_result.warnings,
        })
    }

    /// Analyzes the performance characteristics of the field
    pub fn analyze_performance(&mut self) -> IteratorStackResult<PerformanceAnalysis> {
        let chains = self.get_or_parse_chains()?;

        let max_depth = chains
            .hash_chain
            .depth
            .max(chains.range_chain.depth)
            .max(chains.atom_uuid_chain.depth);

        let estimated_complexity = Self::estimate_complexity_for_chains(chains);
        let memory_usage = Self::estimate_memory_usage_for_chains(chains);
        let recommendations = Self::generate_performance_recommendations_for_chains(chains);

        Ok(PerformanceAnalysis {
            max_depth,
            estimated_complexity,
            memory_usage_category: if memory_usage < 1000 {
                MemoryUsageCategory::Low
            } else if memory_usage < 10000 {
                MemoryUsageCategory::Medium
            } else {
                MemoryUsageCategory::High
            },
            recommendations,
        })
    }

    /// Static version of estimate_complexity for avoiding borrowing conflicts
    fn estimate_complexity_for_chains(chains: &HashRangeChains) -> ComplexityEstimate {
        let hash_ops = Self::count_operations_static(&chains.hash_chain);
        let range_ops = Self::count_operations_static(&chains.range_chain);
        let uuid_ops = Self::count_operations_static(&chains.atom_uuid_chain);

        let total_ops = hash_ops + range_ops + uuid_ops;
        let max_depth = chains
            .hash_chain
            .depth
            .max(chains.range_chain.depth)
            .max(chains.atom_uuid_chain.depth);

        ComplexityEstimate {
            operation_count: total_ops,
            depth_factor: max_depth,
            estimated_big_o: if max_depth <= 1 {
                "O(n)".to_string()
            } else if max_depth <= 2 {
                "O(n²)".to_string()
            } else {
                format!("O(n^{})", max_depth)
            },
        }
    }

    /// Static version of count_operations
    fn count_operations_static(chain: &ParsedChain) -> usize {
        chain.operations.len()
    }

    /// Static version of estimate_memory_usage for avoiding borrowing conflicts
    fn estimate_memory_usage_for_chains(chains: &HashRangeChains) -> usize {
        // Simple estimation based on depth and operation count
        let base_usage = 100; // Base memory per field
        let depth_multiplier = chains
            .hash_chain
            .depth
            .max(chains.range_chain.depth)
            .max(chains.atom_uuid_chain.depth);

        base_usage * (depth_multiplier + 1) * chains.hash_chain.operations.len()
    }

    /// Static version of generate_performance_recommendations for avoiding borrowing conflicts
    fn generate_performance_recommendations_for_chains(chains: &HashRangeChains) -> Vec<String> {
        let mut recommendations = Vec::new();

        let max_depth = chains
            .hash_chain
            .depth
            .max(chains.range_chain.depth)
            .max(chains.atom_uuid_chain.depth);

        if max_depth > 3 {
            recommendations
                .push("Consider reducing iterator depth for better performance".to_string());
        }

        if chains.range_chain.depth < chains.hash_chain.depth - 1 {
            recommendations
                .push("Range field will be heavily broadcast; consider restructuring".to_string());
        }

        if chains.atom_uuid_chain.depth < chains.hash_chain.depth - 1 {
            recommendations
                .push("Atom UUID field will be heavily broadcast; consider caching".to_string());
        }

        recommendations
    }
}

/// Index entry produced by HashRange field execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Hash field value (used for indexing)
    pub hash_value: Value,
    /// Range field value (used for sorting/filtering)
    pub range_value: Value,
    /// Atom UUID reference
    pub atom_uuid: String,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Information about the iterator stack structure of a HashRange field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeStackInfo {
    /// Maximum depth across all field expressions
    pub max_depth: usize,
    /// Iterator depth of hash field
    pub hash_depth: usize,
    /// Iterator depth of range field
    pub range_depth: usize,
    /// Iterator depth of atom UUID field
    pub atom_uuid_depth: usize,
    /// Alignment type for hash field
    pub hash_alignment: Option<crate::transform::iterator_stack::chain_parser::FieldAlignment>,
    /// Alignment type for range field
    pub range_alignment: Option<crate::transform::iterator_stack::chain_parser::FieldAlignment>,
    /// Alignment type for atom UUID field
    pub atom_uuid_alignment: Option<crate::transform::iterator_stack::chain_parser::FieldAlignment>,
    /// Whether all fields are compatible
    pub compatible: bool,
    /// Validation warnings
    pub warnings: Vec<crate::transform::iterator_stack::field_alignment::AlignmentWarning>,
}

/// Performance analysis for a HashRange field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    /// Maximum iterator depth
    pub max_depth: usize,
    /// Estimated computational complexity
    pub estimated_complexity: ComplexityEstimate,
    /// Memory usage category
    pub memory_usage_category: MemoryUsageCategory,
    /// Performance optimization recommendations
    pub recommendations: Vec<String>,
}

/// Estimate of computational complexity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityEstimate {
    /// Number of operations
    pub operation_count: usize,
    /// Depth multiplication factor
    pub depth_factor: usize,
    /// Big-O notation estimate
    pub estimated_big_o: String,
}

/// Categories of memory usage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryUsageCategory {
    Low,
    Medium,
    High,
}

impl_field!(HashRangeField);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::types::TrustDistanceScaling;
    use crate::permissions::types::policy::TrustDistance;

    fn create_test_hash_range_field() -> HashRangeField {
        let permission_policy =
            PermissionsPolicy::new(TrustDistance::Distance(0), TrustDistance::Distance(0));
        let payment_config =
            FieldPaymentConfig::new(1.0, TrustDistanceScaling::None, None).unwrap();

        HashRangeField::new(
            permission_policy,
            payment_config,
            HashMap::new(),
            "blogpost.map().content.split_by_word().map()".to_string(),
            "blogpost.map().publish_date".to_string(),
            "blogpost.map().$atom_uuid".to_string(),
        )
    }

    #[test]
    fn test_hash_range_field_creation() {
        let field = create_test_hash_range_field();

        assert_eq!(
            field.hash_field(),
            "blogpost.map().content.split_by_word().map()"
        );
        assert_eq!(field.range_field(), "blogpost.map().publish_date");
        assert_eq!(field.atom_uuid_field(), "blogpost.map().$atom_uuid");
    }

    #[test]
    fn test_expression_validation() {
        let mut field = create_test_hash_range_field();

        // This should validate successfully
        let result = field.validate_expressions();
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_expressions() {
        let permission_policy =
            PermissionsPolicy::new(TrustDistance::Distance(0), TrustDistance::Distance(0));
        let payment_config =
            FieldPaymentConfig::new(1.0, TrustDistanceScaling::None, None).unwrap();

        let mut field = HashRangeField::new(
            permission_policy,
            payment_config,
            HashMap::new(),
            "blogpost.map().tags.split_array().map()".to_string(), // Different branch
            "blogpost.map().comments.map()".to_string(), // Different branch - should cause error
            "blogpost.map().$atom_uuid".to_string(),
        );

        // This should fail validation due to incompatible branches
        let result = field.validate_expressions();
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_info() {
        let mut field = create_test_hash_range_field();

        let stack_info = field.get_stack_info().unwrap();
        assert_eq!(stack_info.max_depth, 2);
        assert_eq!(stack_info.hash_depth, 2);
        assert_eq!(stack_info.range_depth, 1);
        assert_eq!(stack_info.atom_uuid_depth, 1);
        assert!(stack_info.compatible);
    }

    #[test]
    fn test_performance_analysis() {
        let mut field = create_test_hash_range_field();

        let analysis = field.analyze_performance().unwrap();
        assert_eq!(analysis.max_depth, 2);
        assert!(analysis.estimated_complexity.operation_count > 0);
        assert_eq!(analysis.estimated_complexity.estimated_big_o, "O(n²)");
    }

    #[test]
    fn test_field_updates() {
        let mut field = create_test_hash_range_field();

        // Modify field expressions
        field.set_hash_field("blogpost.map().title".to_string());
        field.set_range_field("blogpost.map().created_at".to_string());

        // Should parse new expressions
        let stack_info = field.get_stack_info().unwrap();
        assert_eq!(stack_info.hash_depth, 1);
        assert_eq!(stack_info.range_depth, 1);
    }

    #[test]
    fn test_indexing_execution() {
        let mut field = create_test_hash_range_field();

        let input_data = serde_json::json!({
            "blogpost": [
                {
                    "content": "hello world test",
                    "publish_date": "2024-01-01",
                    "$atom_uuid": "uuid1"
                }
            ]
        });

        let result = field.execute_indexing(input_data);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_stack_info_alignment_lookups() {
        let mut field = create_test_hash_range_field();

        let stack_info = field.get_stack_info().unwrap();

        // Verify that all alignments are successfully retrieved
        assert!(
            stack_info.hash_alignment.is_some(),
            "Hash alignment should be retrieved successfully"
        );
        assert!(
            stack_info.range_alignment.is_some(),
            "Range alignment should be retrieved successfully"
        );
        assert!(
            stack_info.atom_uuid_alignment.is_some(),
            "Atom UUID alignment should be retrieved successfully"
        );

        // Verify the specific alignment types
        let hash_alignment = stack_info.hash_alignment.unwrap();
        let range_alignment = stack_info.range_alignment.unwrap();
        let atom_uuid_alignment = stack_info.atom_uuid_alignment.unwrap();

        // Hash field should be OneToOne (depth 2, max depth 2)
        assert_eq!(
            hash_alignment,
            crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne
        );

        // Range field should be Broadcast (depth 1, max depth 2)
        assert_eq!(
            range_alignment,
            crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast
        );

        // Atom UUID field should be Broadcast (depth 1, max depth 2)
        assert_eq!(
            atom_uuid_alignment,
            crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast
        );

        // Verify that the field is compatible
        assert!(stack_info.compatible, "Field should be compatible");
    }
}
