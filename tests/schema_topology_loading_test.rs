/// Test to verify that schemas with field_topologies can be loaded successfully
use datafold::schema::SchemaCore;

#[tokio::test]
async fn test_load_blogpost_schema_with_topology() {
    let schema_core = SchemaCore::new_for_testing().await.expect("Failed to create SchemaCore");
    
    // Load the BlogPost schema file
    let result = schema_core.load_schema_from_file("tests/schemas_for_testing/BlogPost.json").await;
    
    match &result {
        Ok(()) => {
            println!("✅ Successfully loaded BlogPost schema");
            
            // Verify the schema was actually loaded
            let schemas = schema_core.get_schemas().expect("Failed to get schemas");
            assert!(schemas.contains_key("BlogPost"), "BlogPost schema not found in loaded schemas");
            
            let blogpost = &schemas["BlogPost"];
            println!("📋 Schema name: {}", blogpost.name);
            println!("📋 Field topologies count: {}", blogpost.field_topologies.len());
            
            // Verify topologies are present
            assert!(blogpost.field_topologies.contains_key("title"), "title topology missing");
            assert!(blogpost.field_topologies.contains_key("content"), "content topology missing");
            assert!(blogpost.field_topologies.contains_key("author"), "author topology missing");
            assert!(blogpost.field_topologies.contains_key("publish_date"), "publish_date topology missing");
            assert!(blogpost.field_topologies.contains_key("tags"), "tags topology missing");
            
            println!("✅ All field topologies present");
        }
        Err(e) => {
            panic!("❌ Failed to load BlogPost schema: {}", e);
        }
    }
}

#[tokio::test]
async fn test_load_blogpost_word_index_transform_with_topology() {
    let schema_core = SchemaCore::new_for_testing().await.expect("Failed to create SchemaCore");
    
    // First load the source schema (BlogPost)
    schema_core.load_schema_from_file("tests/schemas_for_testing/BlogPost.json")
        .await
        .expect("Failed to load BlogPost schema");
    
    // Then load the transform schema (BlogPostWordIndex)
    let result = schema_core.load_schema_from_file("tests/schemas_for_testing/BlogPostWordIndex.json").await;
    
    match &result {
        Ok(()) => {
            println!("✅ Successfully loaded BlogPostWordIndex schema");
            
            let schemas = schema_core.get_schemas().expect("Failed to get schemas");
            assert!(schemas.contains_key("BlogPostWordIndex"), "BlogPostWordIndex schema not found");
            
            let word_index = &schemas["BlogPostWordIndex"];
            println!("📋 Schema name: {}", word_index.name);
            println!("📋 Transform fields: {:?}", word_index.transform_fields.as_ref().map(|tf| tf.len()));
            println!("📋 Field topologies count: {}", word_index.field_topologies.len());
            
            // Verify it has transform fields
            assert!(word_index.transform_fields.is_some(), "Transform fields missing");
            
            // Verify topologies are present for transform output fields
            assert!(word_index.field_topologies.contains_key("word"), "word topology missing");
            assert!(word_index.field_topologies.contains_key("publish_date"), "publish_date topology missing");
            
            println!("✅ Transform schema loaded with topologies");
        }
        Err(e) => {
            panic!("❌ Failed to load BlogPostWordIndex schema: {}", e);
        }
    }
}

#[tokio::test]
async fn test_load_all_available_schemas() {
    let schema_core = SchemaCore::new_for_testing().await.expect("Failed to create SchemaCore");
    
    // Try to load all schemas from tests/schemas_for_testing directory
    let loaded_count = schema_core.load_schemas_from_directory("tests/schemas_for_testing")
        .await
        .expect("Failed to load schemas from directory");
    
    println!("📋 Loaded {} schemas from tests/schemas_for_testing/", loaded_count);
    
    // We should have loaded some schemas
    assert!(loaded_count > 0, "No schemas were loaded from tests/schemas_for_testing directory");
    
    // Verify schemas are accessible
    let schemas = schema_core.get_schemas().expect("Failed to get schemas");
    println!("📋 Total schemas in memory: {}", schemas.len());
    
    // Print all loaded schema names
    for (name, schema) in schemas.iter() {
        println!("  - {} (topologies: {})", name, schema.field_topologies.len());
    }
    
    // Check a few key schemas
    assert!(schemas.contains_key("BlogPost"), "BlogPost not loaded");
    assert!(schemas.contains_key("User"), "User not loaded");
    assert!(schemas.contains_key("Message"), "Message not loaded");
    
    println!("✅ All schemas loaded successfully");
}

#[tokio::test]
async fn test_schema_with_array_topology() {
    let schema_core = SchemaCore::new_for_testing().await.expect("Failed to create SchemaCore");
    
    schema_core.load_schema_from_file("tests/schemas_for_testing/BlogPost.json")
        .await
        .expect("Failed to load BlogPost schema");
    
    let schemas = schema_core.get_schemas().expect("Failed to get schemas");
    let blogpost = &schemas["BlogPost"];
    
    // Check that tags field has Array topology
    let tags_topology = blogpost.field_topologies.get("tags")
        .expect("tags field topology not found");
    
    println!("📋 Tags topology: {:?}", tags_topology);
    
    // The topology should be an Array type
    match &tags_topology.root {
        datafold::schema::types::TopologyNode::Array { .. } => {
            println!("✅ Tags correctly has Array topology");
        }
        other => {
            panic!("❌ Expected Array topology for tags, got: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_schema_json_roundtrip() {
    use datafold::schema::types::DeclarativeSchemaDefinition;
    
    // Read the BlogPost schema file
    let contents = std::fs::read_to_string("tests/schemas_for_testing/BlogPost.json")
        .expect("Failed to read BlogPost.json");
    
    // Parse it
    let parsed: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(&contents);
    
    match parsed {
        Ok(schema) => {
            println!("✅ Successfully parsed BlogPost schema");
            println!("📋 Schema name: {}", schema.name);
            println!("📋 Fields: {:?}", schema.fields);
            println!("📋 Field topologies count: {}", schema.field_topologies.len());
            
            // Verify the structure
            assert_eq!(schema.name, "BlogPost");
            assert!(schema.fields.is_some(), "fields should be present");
            assert!(!schema.field_topologies.is_empty(), "field_topologies should not be empty");
        }
        Err(e) => {
            panic!("❌ Failed to parse BlogPost.json: {}", e);
        }
    }
}

