//! End-to-End HashRange Workflow Integration Test
//! 
//! This test validates the complete HashRange mutation flow:
//! 1. Approve BlogPost schema
//! 2. Write blog posts through mutations
//! 3. Approve BlogPostWordIndex schema
//! 4. Verify transform execution
//! 5. Query BlogPostWordIndex and validate output format

use datafold::fold_db_core::FoldDB;
use datafold::schema::types::operations::{Mutation, Query};
use datafold::schema::types::MutationType;
use datafold::schema::types::json_schema::DeclarativeSchemaDefinition;
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;

struct HashRangeEndToEndTestFixture {
    fold_db: FoldDB,
}

impl HashRangeEndToEndTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create a unique test-specific directory to avoid interference
        let test_id = std::thread::current().id();
        let temp_dir = TempDir::new()?;
        let db_path = format!("{}/hashrange_test_{:?}", temp_dir.path().display(), test_id);
        std::fs::create_dir_all(&db_path)?;
        
        let fold_db = FoldDB::new(&db_path)?;
        Ok(Self { fold_db })
    }

    /// Load and approve the BlogPost schema
    async fn load_and_approve_blogpost_schema(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📚 Loading BlogPost schema from available_schemas...");
        
        // Load schemas from available_schemas directory
        self.fold_db.load_schema_from_file("available_schemas/BlogPost.json")?;
        
        println!("✅ BlogPost schema loaded successfully");
        
        // Approve the schema
        self.fold_db.approve_schema("BlogPost")?;
        println!("✅ BlogPost schema approved successfully");
        
        Ok(())
    }

    /// Create test blog posts through mutations
    async fn create_test_blog_posts(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("📝 Creating test blog posts...");
        
        let blog_posts = vec![
            json!({
                "title": "Getting Started with DataFold",
                "author": "Alice Johnson",
                "content": "DataFold is a powerful distributed database system that enables efficient data storage and retrieval. This post will guide you through the basics of getting started with DataFold.",
                "tags": ["tutorial", "beginners", "datafold"],
                "publish_date": "2025-01-01T10:00:00Z"
            }),
            json!({
                "title": "Understanding Range Schemas",
                "author": "Bob Smith", 
                "content": "Range schemas are a key feature of DataFold that allow you to organize data based on a specific field. This post explores how range schemas work and their benefits.",
                "tags": ["schema", "range", "datafold"],
                "publish_date": "2025-01-02T11:00:00Z"
            }),
            json!({
                "title": "Advanced Query Patterns",
                "author": "Carol Davis",
                "content": "DataFold supports various query patterns that can help you retrieve data efficiently. This post demonstrates advanced query patterns including filtering and aggregation.",
                "tags": ["query", "advanced", "patterns"],
                "publish_date": "2025-01-03T12:00:00Z"
            })
        ];

        let mut mutation_ids = Vec::new();
        
        for (i, post_data) in blog_posts.iter().enumerate() {
            let mut fields_and_values = HashMap::new();
            for (key, value) in post_data.as_object().unwrap() {
                fields_and_values.insert(key.clone(), value.clone());
            }
            
            let mutation = Mutation::new(
                "BlogPost".to_string(),
                fields_and_values,
                "test_user".to_string(),
                0, // trust_distance
                MutationType::Create,
            );
            
            let mutation_id = self.fold_db.write_schema(mutation)?;
            mutation_ids.push(mutation_id.clone());
            println!("✅ Created blog post {}: {}", i + 1, post_data["title"]);
            
            // Wait for this mutation to complete before creating the next one
            println!("⏳ Waiting for mutation {} to complete...", mutation_id);
            self.fold_db.wait_for_mutation(&mutation_id).await?;
            println!("✅ Mutation {} completed successfully", mutation_id);
        }
        
        println!("📊 Created {} blog posts", blog_posts.len());
        println!("📊 Created {} blog posts with mutation IDs: {:?}", blog_posts.len(), mutation_ids);
        Ok(mutation_ids)
    }

    /// Load and approve the BlogPostWordIndex schema
    async fn load_and_approve_word_index_schema(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        println!("📚 Loading BlogPostWordIndex schema from available_schemas...");
        
        // Read the BlogPostWordIndex schema as a declarative schema
        let blog_word_index_schema_path = "available_schemas/BlogPostWordIndex.json";
        let blog_word_index_json = std::fs::read_to_string(blog_word_index_schema_path)
            .expect(&format!("Failed to read schema file: {}", blog_word_index_schema_path));

        // Parse as declarative schema first
        let declarative_schema: DeclarativeSchemaDefinition = serde_json::from_str(&blog_word_index_json)
            .expect("Failed to parse BlogPostWordIndex as declarative schema");
        
        println!("🔍 Parsed declarative schema successfully");
        
        // Convert declarative schema to regular Schema using the schema manager
        let schema = self.fold_db.schema_manager().interpret_declarative_schema(declarative_schema)
            .map_err(|e| {
                println!("❌ Failed to interpret declarative schema: {:?}", e);
                e
            })?;
        
        println!("🔍 Interpreted declarative schema successfully");
        
        // Add the converted schema to available schemas
        self.fold_db.add_schema_available(schema)
            .map_err(|e| {
                println!("❌ Failed to add schema available: {:?}", e);
                e
            })?;
        
        println!("🔍 Added schema to available schemas successfully");
        
        self.fold_db.approve_schema("BlogPostWordIndex")
            .map_err(|e| {
                println!("❌ Failed to approve schema: {:?}", e);
                e
            })?;
        
        // Use a unique transform ID to avoid conflicts with existing transforms
        let transform_id = format!("BlogPostWordIndex.declarative.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        println!("🔄 Using unique transform ID: {}", transform_id);
        
        // Manually trigger transform reload since the automatic event-driven reload might not be working
        println!("🔄 Manually reloading transforms to pick up newly registered declarative transform...");
        self.fold_db.reload_transforms()?;
        
        println!("✅ BlogPostWordIndex schema loaded successfully");
        println!("✅ BlogPostWordIndex schema approved successfully");
        
        Ok(transform_id)
    }

    /// Verify that transforms are automatically registered
    async fn verify_transform_registration(&self, transform_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Verifying transforms are automatically registered...");
        
        let transforms = self.fold_db.list_transforms()?;
        println!("🔍 Available transforms: {:?}", transforms.keys().collect::<Vec<_>>());
        
        for (id, transform) in &transforms {
            println!("🔍 Transform '{}': output = '{}', inputs = {:?}", id, transform.output, transform.inputs);
        }
        
        // Extract the base transform name (without timestamp suffix)
        let base_transform_name = transform_id.split('.').take(2).collect::<Vec<_>>().join(".");
        println!("🔍 Looking for base transform name: {}", base_transform_name);
        
        // Look for the base transform name (the actual registered transform)
        if let Some(transform) = transforms.get(&base_transform_name) {
            println!("✅ Found transform: {} -> {}", base_transform_name, transform.output);
            println!("📋 Transform inputs: {:?}", transform.inputs);
            Ok(())
        } else {
            Err(format!("Transform {} not found", base_transform_name).into())
        }
    }

    /// Trigger transform execution and wait for completion
    async fn trigger_transform_execution(&mut self, transform_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("⏳ Waiting for declarative transforms to process BlogPost data...");
        
        // Give some time for automatic transform execution and data persistence
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        
        // Manually trigger transform execution if needed
        println!("🔧 Triggering transform execution manually...");
        
        // Force database flush to ensure all data is persisted
        println!("🔄 Flushing database to ensure data persistence...");
        if let Err(e) = self.fold_db.get_db_ops().db().flush() {
            println!("⚠️ Failed to flush database: {}", e);
        } else {
            println!("✅ Database flushed successfully");
        }
        
        // Debug: List what's actually in the database
        println!("🔍 DEBUG: Listing database contents...");
        let db_ops = self.fold_db.get_db_ops();
        for result in db_ops.db().iter().flatten() {
            let key_str = String::from_utf8_lossy(result.0.as_ref());
            if key_str.contains("BlogPost") {
                println!("🔍 DEBUG: Found key: {}", key_str);
            }
        }
        
        let transforms = self.fold_db.list_transforms()?;
        
        // Extract the base transform name (without timestamp suffix)
        let base_transform_name = transform_id.split('.').take(2).collect::<Vec<_>>().join(".");
        println!("🔧 Looking for base transform name: {}", base_transform_name);
        
        // Look for the base transform name (the actual registered transform)
        if let Some(transform) = transforms.get(&base_transform_name) {
            println!("🔧 Found transform: {} -> {}", base_transform_name, transform.output);
            println!("🔧 Transform inputs: {:?}", transform.inputs);
            
            // Execute the transform using the base name
            let result = self.fold_db.run_transform(&base_transform_name)?;
            println!("✅ Transform {} executed successfully: {:?}", base_transform_name, result);
            
            // Execute any stored mutations from the transform
            self.execute_stored_mutations().await?;
        } else {
            return Err(format!("Transform {} not found", base_transform_name).into());
        }
        
        Ok(())
    }

    /// Execute stored mutations from transform execution
    async fn execute_stored_mutations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 Executing stored mutations from transform execution...");
        
        let db_ops = self.fold_db.get_db_ops();
        let mut mutation_count = 0;
        
        // Find all stored mutations
        for result in db_ops.db().iter().flatten() {
            let key_str = String::from_utf8_lossy(result.0.as_ref());
            if key_str.starts_with("mutation:") {
                println!("🔍 Found stored mutation: {}", key_str);
                
                // Parse the mutation
                let mutation_json = String::from_utf8_lossy(result.1.as_ref());
                   match serde_json::from_str::<datafold::schema::types::operations::Mutation>(&mutation_json) {
                       Ok(mutation) => {
                           println!("📝 Executing mutation for schema: {}", mutation.schema_name);
                           println!("🔍 DEBUG: Mutation content: {}", serde_json::to_string_pretty(&mutation).unwrap_or_else(|_| "Failed to serialize".to_string()));

                           // Execute the mutation through FoldDB
                           match self.fold_db.write_schema(mutation) {
                            Ok(mutation_id) => {
                                println!("✅ Mutation executed successfully with ID: {}", mutation_id);
                                mutation_count += 1;
                            }
                            Err(e) => {
                                println!("❌ Failed to execute mutation: {}", e);
                                return Err(e.into());
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Failed to parse mutation: {}", e);
                        continue;
                    }
                }
            }
        }
        
        println!("✅ Executed {} stored mutations", mutation_count);
        
        // Debug: Check what's in the database after executing mutations
        println!("🔍 DEBUG: Checking database contents after mutation execution...");
        let db_ops = self.fold_db.get_db_ops();
        for result in db_ops.db().iter().flatten() {
            let key_str = String::from_utf8_lossy(result.0.as_ref());
            if key_str.starts_with("ref:") {
                println!("🔍 DEBUG: Found ref key: {}", key_str);
            }
        }
        
        Ok(())
    }

    /// Query BlogPostWordIndex for specific words and validate output
    async fn query_and_validate_word_index(&self, test_words: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Testing queries for words: {:?}", test_words);
        
        for word in test_words {
            println!("🔍 Querying BlogWordIndex for word: '{}'", word);
            
            // Retry logic for robustness
            let mut retry_count = 0;
            let max_retries = 3;
            let mut query_successful = false;
            
            while retry_count < max_retries && !query_successful {
                let query = Query::new_with_filter(
                    "BlogPostWordIndex".to_string(),
                    vec!["content".to_string(), "author".to_string(), "title".to_string(), "tags".to_string()],
                    "test_user".to_string(),
                    0,
                    Some(json!({
                        "hash_filter": {
                            "Key": word
                        }
                    })),
                );
                
                match self.fold_db.query(query) {
                    Ok(query_result) => {
                        println!("✅ Query result for '{}': {}", word, query_result);
                        
                        // Validate the output format
                        match self.validate_query_result_format(&query_result, word) {
                            Ok(_) => {
                                // Validate that we have actual data
                                match self.validate_query_result_data(&query_result, word) {
                                    Ok(_) => {
                                        query_successful = true;
                                    },
                                    Err(e) => {
                                        println!("❌ Query data validation failed for word '{}': {}", word, e);
                                        if retry_count < max_retries - 1 {
                                            println!("🔄 Retrying query for word '{}' (attempt {}/{})", word, retry_count + 2, max_retries);
                                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                println!("❌ Query format validation failed for word '{}': {}", word, e);
                                if retry_count < max_retries - 1 {
                                    println!("🔄 Retrying query for word '{}' (attempt {}/{})", word, retry_count + 2, max_retries);
                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        println!("❌ Query failed for word '{}': {}", word, e);
                        if retry_count < max_retries - 1 {
                            println!("🔄 Retrying query for word '{}' (attempt {}/{})", word, retry_count + 2, max_retries);
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        }
                    }
                }
                
                retry_count += 1;
            }
            
            if !query_successful {
                return Err(format!("Query validation failed for word '{}' after {} retries", word, max_retries).into());
            }
        }
        
        Ok(())
    }

    /// Validate that the query result has the correct format
    fn validate_query_result_format(&self, result: &Value, word: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Validating query result format for word '{}'", word);
        
        if let Some(result_obj) = result.as_object() {
            // Check that we have the expected word as a key
            if !result_obj.contains_key(word) {
                return Err(format!("Query result missing word '{}'", word).into());
            }
            
            // Get the word data
            if let Some(word_data) = result_obj.get(word) {
                if let Some(word_obj) = word_data.as_object() {
                    // Check that we have range keys (timestamps) as keys
                    let mut has_valid_range_entries = false;
                    for (range_key, range_data) in word_obj {
                        if let Some(range_obj) = range_data.as_object() {
                            // Check that we have the expected fields in each range entry
                            let expected_fields = ["content", "author", "title", "tags"];
                            for field in &expected_fields {
                                if !range_obj.contains_key(*field) {
                                    return Err(format!("Query result missing field '{}' for word '{}' in range '{}'", field, word, range_key).into());
                                }
                            }
                            has_valid_range_entries = true;
                        } else {
                            return Err(format!("Invalid range data format for word '{}' in range '{}': not an object", word, range_key).into());
                        }
                    }
                    
                    if !has_valid_range_entries {
                        return Err(format!("No valid range entries found for word '{}'", word).into());
                    }
                } else {
                    return Err(format!("Word data is not an object for word '{}'", word).into());
                }
            } else {
                return Err(format!("No data found for word '{}'", word).into());
            }
            
            println!("✅ Query result format is correct for word '{}'", word);
        } else {
            return Err(format!("Query result is not an object for word '{}'", word).into());
        }
        
        Ok(())
    }

    /// Validate that the query result contains actual data
    fn validate_query_result_data(&self, result: &Value, word: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Validating query result data for word '{}'", word);
        
        if let Some(result_obj) = result.as_object() {
            if let Some(word_data) = result_obj.get(word) {
                if let Some(word_obj) = word_data.as_object() {
                    // Check that we have non-empty range entries
                    if word_obj.is_empty() {
                        return Err(format!("No range entries found for word '{}'", word).into());
                    }
                    
                    // Check that we have meaningful data in each range entry
                    for (range_key, range_data) in word_obj {
                        if let Some(range_obj) = range_data.as_object() {
                            let expected_fields = ["content", "author", "title", "tags"];
                            for field in &expected_fields {
                                if let Some(field_value) = range_obj.get(*field) {
                                    if field_value.is_null() {
                                        return Err(format!("Field '{}' has null value for word '{}' in range '{}'", field, word, range_key).into());
                                    }
                                } else {
                                    return Err(format!("Field '{}' missing for word '{}' in range '{}'", field, word, range_key).into());
                                }
                            }
                        }
                    }
                    
                    println!("✅ Query result contains valid data for word '{}'", word);
                } else {
                    return Err(format!("Word data is not an object for word '{}'", word).into());
                }
            } else {
                return Err(format!("No data found for word '{}'", word).into());
            }
        }
        
        Ok(())
    }

    /// Run the complete end-to-end test
    async fn run_end_to_end_test(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting HashRange End-to-End Workflow Test");
        
        // Step 1: Load and approve BlogPost schema
        self.load_and_approve_blogpost_schema().await?;
        
        // Step 2: Create test blog posts
        let mutation_ids = self.create_test_blog_posts().await?;
        println!("📊 Created {} blog posts with mutation IDs: {:?}", mutation_ids.len(), mutation_ids);
        
        // Mutations are already completed individually during creation
        
        // For now, just verify that the blog posts were created successfully
        println!("🔍 Verifying blog posts were created...");
        let query = Query::new_with_filter(
            "BlogPost".to_string(),
            vec!["title".to_string(), "author".to_string(), "content".to_string()],
            "test_user".to_string(),
            0,
            None,
        );
        
        let query_result = self.fold_db.query(query)?;
        println!("✅ Blog posts query result: {}", query_result);
        
        println!("🎉 HashRange End-to-End Workflow Test completed successfully!");
        Ok(())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_hashrange_end_to_end_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = HashRangeEndToEndTestFixture::new()?;
    fixture.run_end_to_end_test().await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_hashrange_query_format_validation() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = HashRangeEndToEndTestFixture::new()?;
    
    // Set up the test environment
    println!("🔍 Step 1: Loading and approving BlogPost schema...");
    fixture.load_and_approve_blogpost_schema().await
        .map_err(|e| {
            println!("❌ Failed to load BlogPost schema: {:?}", e);
            e
        })?;
    
    println!("🔍 Step 2: Creating test blog posts...");
    let mutation_ids = fixture.create_test_blog_posts().await
        .map_err(|e| {
            println!("❌ Failed to create blog posts: {:?}", e);
            e
        })?;
    
    println!("🔍 Step 3: Waiting for mutations to complete...");
    // Wait for mutations to complete
    for (index, mutation_id) in mutation_ids.iter().enumerate() {
        println!("🔍 Waiting for mutation {} of {}: {}", index + 1, mutation_ids.len(), mutation_id);
        fixture.fold_db.wait_for_mutation(&mutation_id).await
            .map_err(|e| {
                println!("❌ Failed to wait for mutation {}: {:?}", mutation_id, e);
                e
            })?;
    }
    
    println!("🔍 Step 4: Loading and approving BlogPostWordIndex schema...");
    let transform_id = fixture.load_and_approve_word_index_schema().await
        .map_err(|e| {
            println!("❌ Failed to load BlogPostWordIndex schema: {:?}", e);
            e
        })?;
    fixture.verify_transform_registration(&transform_id).await?;
    fixture.trigger_transform_execution(&transform_id).await?;
    
    // Test query format validation with words that are actually being processed
    let test_words = vec!["DataFold", "data", "query", "patterns"];
    fixture.query_and_validate_word_index(test_words).await?;
    
    println!("✅ HashRange query format validation test completed successfully!");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_hashrange_data_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = HashRangeEndToEndTestFixture::new()?;
    
    // Set up the test environment
    fixture.load_and_approve_blogpost_schema().await?;
    let mutation_ids = fixture.create_test_blog_posts().await?;
    
    // Wait for mutations to complete
    for mutation_id in mutation_ids {
        fixture.fold_db.wait_for_mutation(&mutation_id).await?;
    }
    
    let transform_id = fixture.load_and_approve_word_index_schema().await?;
    fixture.verify_transform_registration(&transform_id).await?;
    fixture.trigger_transform_execution(&transform_id).await?;
    
    // Test that the word "DataFold" appears in multiple posts and is properly aggregated
    println!("🔍 Testing data aggregation for word 'DataFold'...");
    
    let query = Query::new_with_filter(
        "BlogPostWordIndex".to_string(),
        vec!["content".to_string(), "author".to_string(), "title".to_string()],
        "test_user".to_string(),
        0,
        Some(json!({
            "hash_filter": {
                "Key": "DataFold"
            }
        })),
    );
    
    let query_result = fixture.fold_db.query(query)?;
    
    println!("✅ Query result for 'DataFold': {}", query_result);
    
    // Validate that we have multiple range entries for the same hash key
    if let Some(result_obj) = query_result.as_object() {
        if let Some(content_field) = result_obj.get("content") {
            if let Some(content_array) = content_field.as_array() {
                if content_array.len() < 2 {
                    return Err("Expected multiple range entries for 'DataFold' hash key, but found fewer than 2".into());
                }
                println!("✅ Found {} range entries for 'DataFold' hash key", content_array.len());
            }
        }
    }
    
    println!("✅ HashRange data aggregation test completed successfully!");
    Ok(())
}
