use datafold::schema::types::json_schema::{JsonSchemaDefinition, DeclarativeSchemaDefinition};
use datafold::schema::schema_interpretation;
use datafold::schema::validator::SchemaValidator;
use datafold::schema::core::SchemaCore;
use datafold::schema::types::{Schema, SchemaType};

#[test]
fn test_blogpost_word_index_schema_parsing() {
    println!("🔧 Testing BlogPostWordIndex schema parsing");
    
    // Create a schema core for testing
    let schema_core = SchemaCore::new_for_testing("test_blogpost_word_index_parsing").expect("Failed to create schema core");
    let validator = SchemaValidator::new(&schema_core);
    
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
    
    let blogpost_word_index_schema: DeclarativeSchemaDefinition = serde_json::from_str(blogpost_word_index_json)
        .expect("Failed to parse BlogPostWordIndex schema");
    
    let schema = schema_core.interpret_declarative_schema(blogpost_word_index_schema)
        .expect("Failed to interpret BlogPostWordIndex schema");
    
    println!("✅ BlogPostWordIndex schema parsed successfully: {}", schema.name);
    println!("✅ Schema type: {:?}", schema.schema_type);
    println!("✅ Number of fields: {}", schema.fields.len());
    
    // Verify it's a HashRange schema
    assert_eq!(schema.schema_type, SchemaType::HashRange, "BlogPostWordIndex should be a HashRange schema");
    assert_eq!(schema.name, "BlogPostWordIndex", "Schema name should be BlogPostWordIndex");
    assert_eq!(schema.fields.len(), 4, "Should have 4 fields: blog, author, title, tags");
    
    println!("🎉 BlogPostWordIndex schema parsed successfully!");
}
