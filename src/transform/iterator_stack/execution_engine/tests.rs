//! Tests for the execution engine

#[allow(unused_imports)]
use crate::transform::iterator_stack::{
    ExecutionEngine, ExecutionWarningType
};
#[allow(unused_imports)]
use crate::transform::iterator_stack::chain_parser::{
    ParsedChain, ChainOperation
};
#[allow(unused_imports)]
use crate::transform::iterator_stack::field_alignment::{
    FieldAlignmentInfo, AlignmentValidationResult
};
#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use log::debug;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_execution() {
        let mut engine = ExecutionEngine::new();
        
        // Create a simple chain
        let chain = ParsedChain {
            expression: "user.name".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("user".to_string()),
                ChainOperation::FieldAccess("name".to_string()),
            ],
            depth: 0,
            branch: "main".to_string(),
            scopes: vec![],
        };

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert("user.name".to_string(), FieldAlignmentInfo {
            expression: "user.name".to_string(),
            depth: 0,
            alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne,
            branch: "main".to_string(),
            requires_reducer: false,
            suggested_reducer: None,
        });

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 0,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data
        let input_data = json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com"
            }
        });

        let result = engine.execute_fields(&[chain], &alignment_result, input_data).unwrap();

        assert!(!result.index_entries.is_empty());
        assert_eq!(result.index_entries[0].hash_value, json!("John Doe"));
    }

    #[test]
    fn test_broadcast_execution() {
        let mut engine = ExecutionEngine::new();
        
        // Create a chain that will broadcast across an array
        let chain1 = ParsedChain {
            expression: "users.name".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("users".to_string()),
                ChainOperation::FieldAccess("name".to_string()),
            ],
            depth: 1,
            branch: "main".to_string(),
            scopes: vec![],
        };

        let chain2 = ParsedChain {
            expression: "users.email".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("users".to_string()),
                ChainOperation::FieldAccess("email".to_string()),
            ],
            depth: 1,
            branch: "main".to_string(),
            scopes: vec![],
        };

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert("users.name".to_string(), FieldAlignmentInfo {
            expression: "users.name".to_string(),
            depth: 1,
            alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
            branch: "main".to_string(),
            requires_reducer: false,
            suggested_reducer: None,
        });
        field_alignments.insert("users.email".to_string(), FieldAlignmentInfo {
            expression: "users.email".to_string(),
            depth: 1,
            alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
            branch: "main".to_string(),
            requires_reducer: false,
            suggested_reducer: None,
        });

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data with array
        let input_data = json!({
            "users": [
                {
                    "name": "John Doe",
                    "email": "john@example.com"
                },
                {
                    "name": "Jane Smith",
                    "email": "jane@example.com"
                }
            ]
        });

        let result = engine.execute_fields(&[chain1, chain2], &alignment_result, input_data).unwrap();

        debug!("Broadcast test - Index entries count: {}", result.index_entries.len());
        debug!("Broadcast test - Items per depth: {:?}", result.statistics.items_per_depth);
        debug!("Broadcast test - Alignment result valid: {}", alignment_result.valid);
        debug!("Broadcast test - Max depth: {}", alignment_result.max_depth);

        assert!(!result.index_entries.is_empty());
        assert_eq!(result.statistics.items_per_depth.get(&1), Some(&4)); // 2 users × 2 fields = 4 entries at depth 1
    }

    #[test]
    fn test_execution_warnings() {
        let mut engine = ExecutionEngine::new();
        
        // Use the chain parser to create the chain properly
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
        let chain = parser.parse("items.value").unwrap();

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert("items.value".to_string(), FieldAlignmentInfo {
            expression: "items.value".to_string(),
            depth: 1,
            alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
            branch: "main".to_string(),
            requires_reducer: false,
            suggested_reducer: None,
        });

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data with many items
        let mut items = Vec::new();
        for i in 0..1500 {
            items.push(json!({
                "id": i,
                "value": format!("item_{}", i)
            }));
        }

        let input_data = json!({
            "items": items
        });

        let result = engine.execute_fields(&[chain], &alignment_result, input_data).unwrap();

        debug!("Warnings test - Index entries count: {}", result.index_entries.len());
        debug!("Warnings test - Warnings count: {}", result.warnings.len());
        debug!("Warnings test - Items per depth: {:?}", result.statistics.items_per_depth);

        // Should have warnings due to high entry count
        // Temporarily check entry count first
        assert!(result.index_entries.len() > 1000, "Expected more than 1000 entries, got {}", result.index_entries.len());
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| 
            matches!(w.warning_type, ExecutionWarningType::PerformanceDegradation)
        ));
    }

    #[test]
    fn test_simple_array_iteration() {
        let mut engine = ExecutionEngine::new();
        
        // Use the chain parser to create the chain properly
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
        let chain = parser.parse("items.value").unwrap();
        
        debug!("Chain scopes: {:?}", chain.scopes);
        debug!("Chain depth: {}", chain.depth);
        debug!("Chain operations: {:?}", chain.operations);

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert("items.value".to_string(), FieldAlignmentInfo {
            expression: "items.value".to_string(),
            depth: 1,
            alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
            branch: "main".to_string(),
            requires_reducer: false,
            suggested_reducer: None,
        });

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create simple input data
        let input_data = json!({
            "items": [
                {"value": "item1"},
                {"value": "item2"},
                {"value": "item3"}
            ]
        });

        let result = engine.execute_fields(&[chain], &alignment_result, input_data).unwrap();

        debug!("Simple array test - Index entries count: {}", result.index_entries.len());
        debug!("Simple array test - Items per depth: {:?}", result.statistics.items_per_depth);

        // Should have 3 entries (one for each item)
        assert_eq!(result.index_entries.len(), 3);
    }
}
