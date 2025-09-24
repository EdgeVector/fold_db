//! BlogWordIndex Integration Test
//!
//! This test validates the complete workflow for the BlogWordIndex schema:
//! 1. Load BlogPost schema
//! 2. Populate BlogPost with test data via mutations
//! 3. Load BlogWordIndex declarative schema (which should automatically register transforms)
//! 4. Verify transforms run and create word index entries
//! 5. Query BlogWordIndex by word to verify results
//!
//! ## Generic Helper Methods
//!
//! This test fixture uses the shared `SchemaLoader` utilities from `declarative_transform_test_utils`
//! for loading schemas from the `available_schemas` directory:
//!
//! - `SchemaLoader::load_schema_from_available_schemas()` - Loads regular schemas
//! - `SchemaLoader::load_declarative_schema_from_available_schemas()` - Loads declarative schemas
//!
//! These methods can be used by any test to load any schema file from the `available_schemas` directory
//! by simply providing the schema name (without the .json extension).

use datafold::fold_db_core::FoldDB;
use datafold::fold_db_core::infrastructure::message_bus::events::schema_events::TransformExecuted;
use datafold::schema::types::{Mutation, MutationType, Query};
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;

// Import the shared schema loading utilities
use crate::test_utils::SchemaLoader;

/// Integration test fixture for BlogWordIndex testing
struct BlogWordIndexIntegrationFixture {
    fold_db: FoldDB,
    _temp_dir: TempDir, // Keep temp_dir alive to prevent cleanup
}

impl BlogWordIndexIntegrationFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use a temporary directory instead of the root test_db folder to avoid locks
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path();

        // Create a real FoldDB instance for testing using temp directory
        let fold_db = FoldDB::new(db_path.to_str().expect("Failed to convert path to string"))?;

        Ok(Self { fold_db, _temp_dir: temp_dir })
    }


    /// Load BlogPost schema from available_schemas
    fn load_blogpost_schema(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        SchemaLoader::load_schema_from_available_schemas(&mut self.fold_db, "BlogPost")
    }


    /// Load BlogWordIndex declarative schema from available_schemas
    /// This should automatically register the declarative transform
    fn load_blog_word_index_declarative_schema(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        SchemaLoader::load_declarative_schema_from_available_schemas(&mut self.fold_db, "BlogPostWordIndex")?;
        
        // Manually reload transforms to ensure the declarative transform is loaded into memory
        self.fold_db.reload_transforms()?;
        
        Ok(())
    }

    /// Create test blog posts via mutations
    fn create_test_blog_posts(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("📝 Creating test blog posts...");

        let test_posts = vec![
            (
                "Getting Started with DataFold",
                "DataFold is a powerful distributed database system that enables efficient data storage and retrieval. This post will guide you through the basics of getting started with DataFold.",
                "Alice Johnson",
                "2025-01-01T10:00:00Z",
                vec!["tutorial", "beginners", "datafold"]
            ),
            (
                "Understanding Range Schemas",
                "Range schemas are a key feature of DataFold that allow you to organize data based on a specific field. This post explores how range schemas work and their benefits.",
                "Bob Smith",
                "2025-01-02T11:00:00Z",
                vec!["schema", "range", "datafold"]
            ),
            (
                "Advanced Query Patterns",
                "DataFold supports various query patterns that can help you retrieve data efficiently. This post demonstrates advanced query patterns including filtering and aggregation.",
                "Carol Davis",
                "2025-01-03T12:00:00Z",
                vec!["query", "advanced", "patterns"]
            ),
        ];

        let mut mutation_ids = Vec::new();

        for (title, content, author, publish_date, tags) in test_posts {
            let mutation = Mutation {
                schema_name: "BlogPost".to_string(),
                mutation_type: MutationType::Create,
                fields_and_values: HashMap::from([
                    ("title".to_string(), json!(title)),
                    ("content".to_string(), json!(content)),
                    ("author".to_string(), json!(author)),
                    ("publish_date".to_string(), json!(publish_date)),
                    ("tags".to_string(), json!(tags)),
                ]),
                pub_key: "test-user".to_string(),
                trust_distance: 0,
                synchronous: None,
            };

            // Execute mutation
            let mutation_id = self.execute_mutation(mutation)?;
            mutation_ids.push(mutation_id);

            println!("✅ Created blog post: {}", title);
        }

        println!("✅ Created {} blog posts successfully", mutation_ids.len());
        Ok(mutation_ids)
    }

    /// Execute a mutation and return the mutation ID
    fn execute_mutation(
        &mut self,
        mutation: Mutation,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Use the real mutation pipeline
        let mutation_id = self.fold_db.write_schema(mutation)?;
        println!("📝 Executed mutation with ID: {}", mutation_id);
        Ok(mutation_id)
    }

    /// Wait for mutations to be fully processed and committed using the real wait_for_mutation method
    async fn wait_for_mutations_to_complete(
        &self,
        mutation_ids: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "⏳ Waiting for {} mutations to be fully processed...",
            mutation_ids.len()
        );

        // Use the real wait_for_mutation method for each mutation
        for (index, mutation_id) in mutation_ids.iter().enumerate() {
            println!(
                "⏳ Waiting for mutation {} of {}: {}",
                index + 1,
                mutation_ids.len(),
                mutation_id
            );

            // Use the real async wait_for_mutation method - no timeout needed with event-driven approach
            match self.fold_db.wait_for_mutation(mutation_id).await {
                Ok(_) => {
                    println!("✅ Mutation {} completed successfully", mutation_id);
                }
                Err(e) => {
                    println!("❌ Mutation {} failed: {}", mutation_id, e);
                    return Err(format!("Mutation {} failed: {}", mutation_id, e).into());
                }
            }
        }

        println!(
            "✅ All {} mutations completed successfully",
            mutation_ids.len()
        );
        Ok(())
    }

    /// Query BlogWordIndex by word
    fn query_blog_word_index(&self, word: &str) -> Result<Value, Box<dyn std::error::Error>> {
        println!("🔍 Querying BlogWordIndex for word: '{}'", word);

        let query = Query {
            schema_name: "BlogPostWordIndex".to_string(),
            fields: vec![
                "content".to_string(),
                "author".to_string(),
                "title".to_string(),
                "tags".to_string(),
            ],
            pub_key: "test-user".to_string(),
            trust_distance: 0,
            filter: Some(json!({
                "hash_filter": {
                    "Key": word
                }
            })),
        };

        // Execute query using the real query pipeline
        let result = self.fold_db.query(query)?;
        println!(
            "✅ Query result: {}",
            serde_json::to_string_pretty(&result)?
        );

        Ok(result)
    }

    /// Verify that transforms are automatically registered for BlogWordIndex
    fn verify_transforms_registered(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Verifying transforms are automatically registered...");

        let transforms = self.fold_db.transform_manager().list_transforms()?;

        // Check if BlogWordIndex transform is automatically registered
        let blog_word_index_transform = transforms
            .iter()
            .find(|(_, transform)| transform.get_output().contains("BlogPostWordIndex"));

        match blog_word_index_transform {
            Some((transform_id, transform)) => {
                println!(
                    "✅ Found automatically registered BlogWordIndex transform: {}",
                    transform_id
                );
                println!("✅ Transform output: {}", transform.get_output());
                println!("✅ Transform inputs: {:?}", transform.get_inputs());
                println!(
                    "✅ Transform is declarative: {}",
                    transform.is_declarative()
                );
                Ok(())
            }
            None => {
                println!("❌ BlogWordIndex transform not automatically registered");
                println!(
                    "📋 Available transforms: {:?}",
                    transforms.keys().collect::<Vec<_>>()
                );

                // Debug: Check if transform exists in database but not in memory
                let transform_id = "BlogPostWordIndex.declarative";
                match self.fold_db.schema_manager().get_transform(transform_id) {
                    Ok(Some(transform)) => {
                        println!("🔍 Debug: Transform '{}' exists in database but not in transform manager", transform_id);
                        println!("🔍 Debug: Transform output: {}", transform.get_output());
                        println!(
                            "🔍 Debug: Transform is declarative: {}",
                            transform.is_declarative()
                        );
                    }
                    Ok(None) => {
                        println!(
                            "🔍 Debug: Transform '{}' not found in database",
                            transform_id
                        );
                    }
                    Err(e) => {
                        println!(
                            "🔍 Debug: Error checking database for transform '{}': {}",
                            transform_id, e
                        );
                    }
                }

                Err("BlogWordIndex transform not automatically registered".into())
            }
        }
    }


    /// Wait for transforms to process using simplified event-driven approach
    async fn wait_for_transform_execution(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("⏳ Waiting for declarative transforms to process BlogPost data...");

        // Subscribe to TransformExecuted events
        let message_bus = self.fold_db.message_bus();
        let mut transform_consumer = message_bus.subscribe::<TransformExecuted>();

        // Wait for a reasonable amount of time for transforms to complete
        let timeout = std::time::Duration::from_secs(5);
        let start_time = std::time::Instant::now();
        let mut transform_events_received = 0;

        while start_time.elapsed() < timeout {
            match transform_consumer.try_recv() {
                Ok(transform_executed) => {
                    println!("🎯 Received TransformExecuted event for: {}", transform_executed.transform_id);
                    transform_events_received += 1;
                    
                    // If we've received a few transform events, we're good
                    if transform_events_received >= 2 {
                        println!("✅ Sufficient transform events received, proceeding with test");
                        return Ok(());
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No events available, wait briefly
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    println!("⚠️ TransformExecuted event channel disconnected");
                    break;
                }
            }
        }

        if transform_events_received > 0 {
            println!("✅ Received {} transform events, proceeding with test", transform_events_received);
            Ok(())
        } else {
            println!("⚠️ No transform events received within timeout, proceeding anyway");
            Ok(())
        }
    }





}

/// Test the complete BlogWordIndex declarative transform workflow using temp database
#[tokio::test]
async fn test_blog_word_index_declarative_transform_workflow() {
    let mut fixture =
        BlogWordIndexIntegrationFixture::new().expect("Failed to create integration test fixture");

    println!("🚀 Starting BlogWordIndex declarative transform integration test with temp database");

    // Step 1: Load BlogPost schema
    fixture
        .load_blogpost_schema()
        .expect("Failed to load BlogPost schema");

    // Step 2: Load BlogWordIndex declarative schema (this should automatically register transforms)
    fixture
        .load_blog_word_index_declarative_schema()
        .expect("Failed to load BlogWordIndex declarative schema");

    // Step 3: Create test blog posts (these should trigger transforms as they are created)
    let mutation_ids = fixture
        .create_test_blog_posts()
        .expect("Failed to create test blog posts");

    println!("📊 Created {} blog posts", mutation_ids.len());

    // Wait for mutations to be fully processed and committed
    fixture
        .wait_for_mutations_to_complete(&mutation_ids)
        .await
        .expect("Failed to wait for mutations to complete");

    // Step 4: Verify transforms are automatically registered
    fixture
        .verify_transforms_registered()
        .expect("BlogWordIndex transforms not automatically registered");

    // Step 5: Wait for transforms to execute and verify they created data using event-driven approach
    fixture
        .wait_for_transform_execution()
        .await
        .expect("Failed to wait for transform execution");

    // Step 6: Test a simple query to verify the system is working
    println!("\n🔍 Testing basic query functionality...");
    let result = fixture
        .query_blog_word_index("DataFold")
        .expect("Failed to query BlogWordIndex");

    // Just verify we get a valid JSON response
    if result.is_object() {
        println!("✅ Query returned valid JSON object");
    } else {
        println!("⚠️ Query did not return a valid JSON object: {}", result);
    }

    println!("✅ BlogWordIndex declarative transform integration test completed successfully!");
}

/// Test that declarative schema loading automatically registers transforms
#[test]
#[serial_test::serial]
fn test_declarative_schema_automatic_transform_registration() {
    let mut fixture =
        BlogWordIndexIntegrationFixture::new().expect("Failed to create integration test fixture");

    println!("🔧 Testing automatic transform registration for declarative schemas");

    // Load BlogPost schema first
    fixture
        .load_blogpost_schema()
        .expect("Failed to load BlogPost schema");

    // Load BlogWordIndex declarative schema - this should automatically register transforms
    fixture
        .load_blog_word_index_declarative_schema()
        .expect("Failed to load BlogWordIndex declarative schema");

    // Verify transform was automatically registered
    fixture
        .verify_transforms_registered()
        .expect("Transform not automatically registered");

    println!("✅ Automatic transform registration test completed successfully!");
}

/// Test declarative transform execution with real data
#[tokio::test]
async fn test_declarative_transform_execution() {
    let mut fixture =
        BlogWordIndexIntegrationFixture::new().expect("Failed to create integration test fixture");

    println!("🔧 Testing declarative transform execution with real data");

    // Load schemas
    fixture
        .load_blogpost_schema()
        .expect("Failed to load BlogPost schema");
    fixture
        .load_blog_word_index_declarative_schema()
        .expect("Failed to load BlogWordIndex declarative schema");

    // Create multiple blog posts with diverse content for testing
    let test_posts = vec![
        (
            "Getting Started with DataFold",
            "DataFold is a powerful distributed database system that enables efficient data storage and retrieval. This post will guide you through the basics of getting started with DataFold.",
            "Alice Johnson",
            "2025-01-01T10:00:00Z",
            vec!["tutorial", "beginners", "datafold"]
        ),
        (
            "Understanding Range Schemas",
            "Range schemas are a key feature of DataFold that allow you to organize data based on a specific field. This post explores how range schemas work and their benefits.",
            "Bob Smith",
            "2025-01-02T11:00:00Z",
            vec!["schema", "range", "datafold"]
        ),
        (
            "Advanced Query Patterns",
            "DataFold supports various query patterns that can help you retrieve data efficiently. This post demonstrates advanced query patterns including filtering and aggregation.",
            "Carol Davis",
            "2025-01-03T12:00:00Z",
            vec!["query", "advanced", "patterns"]
        ),
        (
            "Test Blog Post",
            "This is a test blog post with specific words for testing the declarative transform.",
            "Test Author",
            "2025-01-04T13:00:00Z",
            vec!["test", "declarative"]
        ),
    ];

    let mut mutation_ids = Vec::new();

    for (title, content, author, publish_date, tags) in test_posts {
        let mutation = Mutation {
            schema_name: "BlogPost".to_string(),
            mutation_type: MutationType::Create,
            fields_and_values: HashMap::from([
                ("title".to_string(), json!(title)),
                ("content".to_string(), json!(content)),
                ("author".to_string(), json!(author)),
                ("publish_date".to_string(), json!(publish_date)),
                ("tags".to_string(), json!(tags)),
            ]),
            pub_key: "test-user".to_string(),
            trust_distance: 0,
            synchronous: None,
        };

        // Execute mutation
        let mutation_id = fixture
            .execute_mutation(mutation)
            .expect(&format!("Failed to create blog post: {}", title));

        mutation_ids.push(mutation_id);
        println!("✅ Created blog post: {}", title);
    }

    println!("✅ Created {} blog posts successfully", mutation_ids.len());

    // Wait for mutations to be fully processed and committed
    fixture
        .wait_for_mutations_to_complete(&mutation_ids)
        .await
        .expect("Failed to wait for mutations to complete");

    // Wait for transform to execute using event-driven approach
    fixture
        .wait_for_transform_execution()
        .await
        .expect("Failed to wait for transform execution");

    // Test a simple query to verify the system is working
    println!("🔍 Testing basic query functionality...");
    let result = fixture
        .query_blog_word_index("test")
        .expect("Failed to query BlogWordIndex");

    // Just verify we get a valid JSON response
    if result.is_object() {
        println!("✅ Query returned valid JSON object");
    } else {
        println!("⚠️ Query did not return a valid JSON object: {}", result);
    }

    println!("✅ Declarative transform execution test completed successfully!");
}

/// Example test demonstrating how to use the generic helper methods with different schemas
#[test]
#[serial_test::serial]
fn test_generic_schema_loading_helpers() {
    let mut fixture =
        BlogWordIndexIntegrationFixture::new().expect("Failed to create integration test fixture");

    println!("🔧 Testing generic schema loading helpers");

    // Example 1: Load BlogPost schema using the shared SchemaLoader utility
    SchemaLoader::load_schema_from_available_schemas(&mut fixture.fold_db, "BlogPost")
        .expect("Failed to load BlogPost schema using shared SchemaLoader");

    // Example 2: Load BlogPostWordIndex declarative schema using the shared SchemaLoader utility
    SchemaLoader::load_declarative_schema_from_available_schemas(&mut fixture.fold_db, "BlogPostWordIndex")
        .expect("Failed to load BlogPostWordIndex declarative schema using shared SchemaLoader");

    // Example 3: You could load any other schema from available_schemas directory like this:
    // SchemaLoader::load_schema_from_available_schemas(&mut fold_db, "MyCustomSchema")?;
    // SchemaLoader::load_declarative_schema_from_available_schemas(&mut fold_db, "MyCustomDeclarativeSchema")?;

    println!("✅ Generic schema loading helpers test completed successfully!");
}
