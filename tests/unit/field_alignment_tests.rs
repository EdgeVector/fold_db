use datafold::transform::iterator_stack::chain_parser::{ChainParser, FieldAlignment};
use datafold::transform::iterator_stack::field_alignment::types::{
    AlignmentErrorType, FieldAlignmentValidator,
    OptimizationType,
};
use std::collections::HashMap;

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
            datafold::transform::iterator_stack::field_alignment::types::FieldAlignmentInfo {
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
