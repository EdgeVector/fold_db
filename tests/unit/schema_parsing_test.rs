use datafold::schema::core::SchemaCore;
use datafold::schema::types::json_schema::DeclarativeSchemaDefinition;
use datafold::schema::types::SchemaType;
use datafold::schema::validator::SchemaValidator;
use tempfile::TempDir;

#[test]
fn test_blogpost_word_index_schema_parsing() {
    println!("🔧 Testing BlogPostWordIndex schema parsing with temp database");

    // Create a temporary directory for the test database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().to_str().expect("Failed to get temp path");

    // Create a schema core for testing using the temp directory
    let schema_core = SchemaCore::new_for_testing(db_path)
        .expect("Failed to create schema core with temp database");
    let _validator = SchemaValidator::new(&schema_core);

    // Test BlogPostWordIndex schema parsing
    let blogpost_word_index_json = r#"{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "blogpost.map().content.split_by_word().map()",
    "range_field": "blogpost.map().publish_date"
  },
  "fields": {
    "blog": { "atom_uuid": "blogpost.map().$atom_uuid" },
    "author": { "atom_uuid": "blogpost.map().author.$atom_uuid" },
    "title": { "atom_uuid": "blogpost.map().title.$atom_uuid" },
    "tags": { "atom_uuid": "blogpost.map().tags.$atom_uuid" }
  }
}"#;

    let blogpost_word_index_schema: DeclarativeSchemaDefinition =
        serde_json::from_str(blogpost_word_index_json)
            .expect("Failed to parse BlogPostWordIndex schema");

    let schema = schema_core
        .interpret_declarative_schema(blogpost_word_index_schema)
        .expect("Failed to interpret BlogPostWordIndex schema");

    println!(
        "✅ BlogPostWordIndex schema parsed successfully: {}",
        schema.name
    );
    println!("✅ Schema type: {:?}", schema.schema_type);
    println!("✅ Number of fields: {}", schema.fields.len());

    // Verify it's a HashRange schema
    assert_eq!(
        schema.schema_type,
        SchemaType::HashRange,
        "BlogPostWordIndex should be a HashRange schema"
    );
    assert_eq!(
        schema.name, "BlogPostWordIndex",
        "Schema name should be BlogPostWordIndex"
    );
    assert_eq!(
        schema.fields.len(),
        4,
        "Should have 4 fields: blog, author, title, tags"
    );

    println!("🎉 BlogPostWordIndex schema parsed successfully!");
}
