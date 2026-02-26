use fold_db::schema::types::key_value::KeyValue;
use fold_db::transform::chain_parser::parser::ChainParser;
use fold_db::transform::iterator_stack_typed::adapter::map_chain_to_specs;
use fold_db::transform::iterator_stack_typed::engine::TypedEngine;
use fold_db::transform::iterator_stack_typed::types::TypedInput;
use serde_json::json;
use std::collections::HashMap;

/// Test full flow: Parse -> Spec -> Execute
/// With implicit cardinality (no .map())
#[test]
fn test_implicit_cardinality_execution() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();

    // 1. Setup Input Data
    let mut input_data: TypedInput = HashMap::new();

    // Field "content" with 2 atoms
    let mut content_atoms = HashMap::new();
    content_atoms.insert(
        KeyValue::new(Some("row1".to_string()), None),
        fold_db::schema::types::field::FieldValue {
            value: json!("Hello world"),
            atom_uuid: "atom1".to_string(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
        },
    );
    content_atoms.insert(
        KeyValue::new(Some("row2".to_string()), None),
        fold_db::schema::types::field::FieldValue {
            value: json!("Another test"),
            atom_uuid: "atom2".to_string(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
        },
    );
    input_data.insert("content".to_string(), content_atoms);

    // 2. Define Chains to Test
    let test_cases = vec![
        // Identity: implicit 1:1
        ("content", 2, "1:1 mapping"),
        // Split: implicit 1:N (2 rows * 2 words each = 4 total)
        ("content.split_by_word()", 4, "1:N split"),
        // Count: implicit N:1 (per row count) -> 2 rows
        ("content.split_by_word().count()", 2, "N:1 count per row"),
    ];

    for (expr, expected_count, desc) in test_cases {
        println!("Testing: {}", desc);
        let parsed = parser.parse(expr).expect("Failed to parse");
        let specs = map_chain_to_specs(&parsed);
        let result = engine.execute_chain(&specs, &input_data, "output");

        let emitted = result.get("output").expect("No output");
        assert_eq!(emitted.len(), expected_count, "Failed: {}", desc);
    }
}
