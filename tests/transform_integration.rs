use serde_json::json;
use std::collections::HashMap;

// Helper function to convert ExecutionResult to records (copied from transform_runner.rs)
fn convert_execution_result_to_records(
    execution_result: &fold_db::transform::result_types::ExecutionResult,
) -> Result<
    Vec<fold_db::fold_db_core::query::formatter::Record>,
    fold_db::schema::types::SchemaError,
> {
    let mut records = Vec::new();

    // Group entries by row_id
    let mut rows: HashMap<String, HashMap<String, Vec<serde_json::Value>>> = HashMap::new();

    for (field_name, entries) in &execution_result.index_entries {
        for entry in entries {
            let row = rows.entry(entry.row_id.clone()).or_default();
            row.entry(field_name.clone())
                .or_default()
                .push(entry.value.clone());
        }
    }

    // Convert each row to a Record
    for (_, fields_map) in rows {
        let mut record_fields = HashMap::new();
        for (field_name, values) in fields_map {
            // Use single value if only one, otherwise create array
            let value = if values.len() == 1 {
                values[0].clone()
            } else {
                serde_json::Value::Array(values)
            };
            record_fields.insert(field_name, value);
        }
        records.push(fold_db::fold_db_core::query::formatter::Record {
            fields: record_fields,
            metadata: HashMap::new(),
        });
    }

    Ok(records)
}

#[test]
fn execute_engine_and_convert_to_records() {
    // Define transform schema with KeyConfig (HashRange)
    let transform_schema_json = json!({
        "name": "BlogPostWordIndex",
        "schema_type": "HashRange",
        "key": {"hash_field": "word", "range_field": "publish_date"},
        "transform_fields": {
            "word": "BlogPost.content.split_by_word()",
            "publish_date": "BlogPost.publish_date",
            "author": "BlogPost.author",
            "title": "BlogPost.title"
        }
    });
    let transform_schema: fold_db::schema::types::DeclarativeSchemaDefinition =
        serde_json::from_value(transform_schema_json).unwrap();

    // Build expressions and parse chains
    let field_to_hash = transform_schema.get_field_to_hash_code();
    let hash_to_code = transform_schema.hash_to_code();
    let expressions: Vec<(String, String)> = field_to_hash
        .iter()
        .map(|(field, hash)| (field.clone(), hash_to_code.get(hash).unwrap().clone()))
        .collect();
    let parsed =
        fold_db::transform::shared_utilities::parse_expressions_batch(&expressions).unwrap();
    let chains_map: HashMap<String, fold_db::transform::chain_parser::ParsedChain> = parsed
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
        ]),
    );

    // Build typed input per target field used by the chains
    type FV = fold_db::schema::types::field::FieldValue;
    type KV = fold_db::schema::types::key_value::KeyValue;
    let mut typed_input: HashMap<String, HashMap<KV, FV>> = HashMap::new();

    // Two blog posts with stable keys
    let k1 = KV::new(Some("h1".to_string()), Some("r1".to_string()));
    let k2 = KV::new(Some("h2".to_string()), Some("r2".to_string()));

    // Helper to insert a field map
    let mut insert_field = |name: &str, v1: serde_json::Value, v2: serde_json::Value| {
        let mut m: HashMap<KV, FV> = HashMap::new();
        m.insert(
            k1.clone(),
            FV {
                value: v1,
                atom_uuid: "a1".to_string(),
                source_file_name: None,
                metadata: None,
            },
        );
        m.insert(
            k2.clone(),
            FV {
                value: v2,
                atom_uuid: "a2".to_string(),
                source_file_name: None,
                metadata: None,
            },
        );
        typed_input.insert(name.to_string(), m);
    };

    insert_field(
        "BlogPost.content",
        json!("Rust empowers fearless concurrency"),
        json!("Tests validate iterator stacks"),
    );
    insert_field(
        "BlogPost.publish_date",
        json!("2024-12-31"),
        json!("2025-01-05"),
    );
    insert_field("BlogPost.author", json!("Carol"), json!("Dylan"));
    insert_field("BlogPost.title", json!("First"), json!("Second"));

    let exec = fold_db::transform::iterator_stack_typed::adapter::execute_fields_typed(
        &chains_map,
        &typed_input,
    );

    // Convert execution result directly to records (no aggregation needed)
    let records = convert_execution_result_to_records(&exec).expect("convert ok");

    // Validate records shape - records now have simpler structure
    assert!(!records.is_empty());

    // Count records by field type
    let mut word_records = 0;
    let mut other_records = 0;

    for record in &records {
        let fields = &record.fields;
        if fields.contains_key("word") {
            word_records += 1;
            // Validate that word field contains actual words
            if let Some(word_val) = fields.get("word") {
                assert!(word_val.is_string(), "word field should be a string");
            }
        } else {
            other_records += 1;
            // These should be the parent-level fields (title, author, publish_date)
            assert!(
                fields.contains_key("title")
                    || fields.contains_key("author")
                    || fields.contains_key("publish_date"),
                "Record should contain expected fields: {:?}",
                fields.keys().collect::<Vec<_>>()
            );
        }
    }

    // We should have word records (from word splitting) and some parent records
    assert!(
        word_records > 0,
        "Should have word records from word splitting"
    );
    assert!(
        other_records > 0,
        "Should have parent records with other fields"
    );

    // Total should be reasonable (words from both blog posts plus parent records)
    assert!(
        records.len() >= 8,
        "Should have at least 8 records (words from 2 blog posts)"
    );
}
