use datafold::transform::iterator_stack::chain_parser::parser::ChainParser;
use datafold::transform::iterator_stack::chain_parser::types::{ChainOperation, FieldAlignment};
use datafold::transform::iterator_stack::errors::IteratorStackError;

#[test]
fn test_simple_chain_parsing() {
    let parser = ChainParser::new();
    let result = parser.parse("blogpost.map()").unwrap();

    assert_eq!(result.depth, 1);
    assert_eq!(result.branch, "blogpost");
    assert_eq!(result.operations.len(), 2);
    assert_eq!(result.scopes.len(), 1);
}

#[test]
fn test_complex_chain_parsing() {
    let parser = ChainParser::new();
    let result = parser
        .parse("blogpost.map().content.split_by_word().map()")
        .unwrap();

    assert_eq!(result.depth, 2);
    assert_eq!(result.branch, "blogpost");
    assert_eq!(result.operations.len(), 5);
    assert_eq!(result.scopes.len(), 2);
}

#[test]
fn test_special_field_parsing() {
    let parser = ChainParser::new();
    let result = parser.parse("blogpost.map().$atom_uuid").unwrap();

    assert_eq!(result.depth, 1);
    assert_eq!(result.branch, "blogpost");
    assert!(matches!(
        result.operations.last(),
        Some(ChainOperation::SpecialField(_))
    ));
}

#[test]
fn test_invalid_chain_syntax() {
    let parser = ChainParser::new();
    let result = parser.parse("map().blogpost");

    assert!(result.is_err());
    if let Err(IteratorStackError::InvalidChainSyntax { .. }) = result {
        // Expected error type
    } else {
        panic!("Expected InvalidChainSyntax error");
    }
}

#[test]
fn test_compatibility_analysis() {
    let parser = ChainParser::new();
    let chain1 = parser
        .parse("blogpost.map().content.split_by_word().map()")
        .unwrap();
    let chain2 = parser.parse("blogpost.map().publish_date").unwrap();

    let analysis = parser.analyze_compatibility(&[chain1, chain2]).unwrap();

    assert_eq!(analysis.max_depth, 2);
    assert!(analysis.compatible);
    assert_eq!(analysis.alignment_requirements.len(), 2);
}

#[test]
fn test_cartesian_fanout_detection() {
    let parser = ChainParser::new();
    let chain1 = parser
        .parse("blogpost.map().tags.split_array().map()")
        .unwrap();
    let chain2 = parser.parse("blogpost.map().comments.map()").unwrap();

    let result = parser.analyze_compatibility(&[chain1, chain2]);

    assert!(result.is_err());
    if let Err(IteratorStackError::AmbiguousFanoutDifferentBranches { .. }) = result {
        // Expected error type
    } else {
        panic!("Expected AmbiguousFanoutDifferentBranches error");
    }
}

#[test]
fn test_reducer_operation_alignment() {
    let parser = ChainParser::new();
    let chain1 = parser
        .parse("blogpost.map().content.map().first()")
        .unwrap();
    let chain2 = parser.parse("blogpost.map().content.map().title").unwrap();

    let analysis = parser.analyze_compatibility(&[chain1, chain2]).unwrap();

    assert_eq!(analysis.max_depth, 2);
    assert!(analysis.compatible);
    assert_eq!(analysis.alignment_requirements.len(), 2);

    // Find the chain with reducer operation
    let reducer_chain = analysis
        .alignment_requirements
        .iter()
        .find(|req| req.field_expression.contains("first"))
        .unwrap();

    // The chain with reducer operation should have Reduced alignment
    assert_eq!(reducer_chain.alignment, FieldAlignment::Reduced);
    assert_eq!(reducer_chain.depth, 2); // depth is 2 for the chain with .map().map().first()

    // The chain without reducer should have OneToOne alignment (max depth)
    let non_reducer_chain = analysis
        .alignment_requirements
        .iter()
        .find(|req| req.field_expression.contains("title"))
        .unwrap();
    assert_eq!(non_reducer_chain.alignment, FieldAlignment::OneToOne);
    assert_eq!(non_reducer_chain.depth, 2); // depth is 2 for the chain with .map().map()
}
