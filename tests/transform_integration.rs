use serde_json::json;
use std::collections::HashMap;

#[test]
fn execute_engine_and_aggregate_rows_with_keyconfig() {
    // Define transform schema with KeyConfig (HashRange)
    let transform_schema_json = json!({
        "name": "BlogPostWordIndex",
        "schema_type": {"HashRange": {"keyconfig": {"hash_field": null, "range_field": null}}},
        "key": {"hash_field": "word", "range_field": "publish_date"},
        "transform_fields": {
            "word": "BlogPost.map().content.split_by_word().map()",
            "publish_date": "BlogPost.map().publish_date",
            "author": "BlogPost.map().author",
            "title": "BlogPost.map().title"
        }
    });
    let transform_schema: datafold::schema::types::DeclarativeSchemaDefinition = serde_json::from_value(transform_schema_json).unwrap();

    // Build expressions and parse chains
    let field_to_hash = transform_schema.get_field_to_hash_code();
    let hash_to_code = transform_schema.hash_to_code();
    let expressions: Vec<(String, String)> = field_to_hash
        .iter()
        .map(|(field, hash)| (field.clone(), hash_to_code.get(hash).unwrap().clone()))
        .collect();
    let parsed = datafold::transform::shared_utilities::parse_expressions_batch(&expressions).unwrap();
    let chains_map: HashMap<String, datafold::transform::iterator_stack::chain_parser::ParsedChain> = parsed
        .iter()
        .map(|(field, chain)| (field.clone(), chain.clone()))
        .collect();

    // Gathered inputs
    let mut input_values: HashMap<String, serde_json::Value> = HashMap::new();
    input_values.insert(
        "BlogPost".to_string(),
        json!([
            {
                "title": "First",
                "content": "Rust empowers fearless concurrency",
                "author": "Carol",
                "publish_date": "2024-12-31"
            },
            {
                "title": "Second",
                "content": "Tests validate iterator stacks",
                "author": "Dylan",
                "publish_date": "2025-01-05"
            }
        ])
    );

    // Execute engine
    let mut engine = datafold::transform::iterator_stack::execution_engine::ExecutionEngine::new();
    let exec = engine.execute_fields(chains_map, input_values.clone()).expect("exec ok");

    // Aggregate into rows
    let all_expressions: Vec<(String, String)> = parsed
        .iter()
        .map(|(field, chain)| (field.clone(), chain.expression.clone()))
        .collect();
    let aggregated = datafold::transform::aggregation::aggregate_results_unified(
        &transform_schema,
        &parsed,
        &exec,
        &input_values,
        &all_expressions,
    ).expect("aggregate ok");

    // Validate rows shape and KeyConfig presence
    let rows = aggregated.as_array().expect("rows array");
    assert!(!rows.is_empty());
    for row in rows {
        let obj = row.as_object().expect("row obj");
        assert!(obj.contains_key("key"));
        assert!(obj.contains_key("fields"));
        let key = obj.get("key").unwrap();
        assert!(key.get("hash").is_some());
        assert!(key.get("range").is_some());
        let fields = obj.get("fields").unwrap().as_object().unwrap();
        assert!(fields.contains_key("word"));
        assert!(fields.contains_key("publish_date"));
        assert!(fields.contains_key("author"));
        assert!(fields.contains_key("title"));
    }
}


