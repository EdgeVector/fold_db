use datafold::datafold_node::DataFoldNode;
use datafold::datafold_node::config::NodeConfig;
use datafold::datafold_node::OperationProcessor;
use datafold::schema::types::field::HashRangeFilter;
use datafold::schema::types::key_value::KeyValue;
use datafold::schema::types::operations::Query;
use datafold::MutationType;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test that querying by an exact range key returns only records matching that range key
/// This test uses the existing BlogPost schema and creates test data to verify filtering
#[tokio::test]
async fn test_exact_range_key_filtering_with_blogpost() {
    // Set up temporary database
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize node with temporary database
    let config = NodeConfig::new(temp_db_path.into());
    let node = DataFoldNode::new(config)
        .expect("Failed to create DataFoldNode");
    
    // Wrap node in Arc<Mutex<>> for OperationProcessor
    let node_arc = Arc::new(tokio::sync::Mutex::new(node));
    let processor = OperationProcessor::new(node_arc);
    
    // Create multiple test blog posts with different publish_date values (range keys)
    let test_posts = vec![
        ("2024-01-01", "First Post", "Alice", vec!["tech", "rust"]),
        ("2024-01-02", "Second Post", "Bob", vec!["programming", "web"]),
        ("2024-01-03", "Third Post", "Charlie", vec!["database", "sql"]),
    ];
    
    // Insert test data using the BlogPost schema
    for (publish_date, title, author, tags) in &test_posts {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("title".to_string(), json!(title));
        fields_and_values.insert("author".to_string(), json!(author));
        fields_and_values.insert("publish_date".to_string(), json!(publish_date));
        fields_and_values.insert("tags".to_string(), json!(tags));
        fields_and_values.insert("content".to_string(), json!("This is test content for the post"));
        
        
        processor.execute_mutation(
            "BlogPost".to_string(),
            fields_and_values,
            KeyValue::new(None, Some(publish_date.to_string())),
            MutationType::Create,
        ).await
        .expect("Failed to execute mutation");
    }
    
    
    // Test 1: Query with exact range key filter for "2024-01-02"
    let target_date = "2024-01-02";
    let query = Query::new_with_filter(
        "BlogPost".to_string(),
        vec!["title".to_string(), "author".to_string(), "publish_date".to_string(), "tags".to_string()],
        Some(HashRangeFilter::HashKey(target_date.to_string())),
    );
    let result_with_filter = processor.execute_query_map(query).await
    .expect("Failed to execute query with filter");
    
    
    // Verify that only the post with publish_date "2024-01-02" is returned
    assert_eq!(result_with_filter.len(), 4, "Should have 4 fields in result");
    
    // Check that the title field contains only "Second Post"
    let title_field_results = result_with_filter.get("title").unwrap();
    assert_eq!(title_field_results.len(), 1, "Should have exactly 1 record for title field");
    
    let (key, field_value) = title_field_results.iter().next().unwrap();
    assert_eq!(key.range, Some(target_date.to_string()), "Range key should be '{}'", target_date);
    assert_eq!(field_value.value, json!("Second Post"), "Title should be 'Second Post'");
    
    // Check that the author field contains only "Bob"
    let author_field_results = result_with_filter.get("author").unwrap();
    assert_eq!(author_field_results.len(), 1, "Should have exactly 1 record for author field");
    
    let (key, field_value) = author_field_results.iter().next().unwrap();
    assert_eq!(key.range, Some(target_date.to_string()), "Range key should be '{}'", target_date);
    assert_eq!(field_value.value, json!("Bob"), "Author should be 'Bob'");
    
    // Check that the publish_date field contains only "2024-01-02"
    let publish_date_field_results = result_with_filter.get("publish_date").unwrap();
    assert_eq!(publish_date_field_results.len(), 1, "Should have exactly 1 record for publish_date field");
    
    let (key, field_value) = publish_date_field_results.iter().next().unwrap();
    assert_eq!(key.range, Some(target_date.to_string()), "Range key should be '{}'", target_date);
    assert_eq!(field_value.value, json!(target_date), "Publish date should be '{}'", target_date);
    
    // Test 2: Query with exact range key filter for "2024-01-01"
    let target_date_2 = "2024-01-01";
    let query = Query::new_with_filter(
        "BlogPost".to_string(),
        vec!["title".to_string(), "author".to_string(), "publish_date".to_string()],
        Some(HashRangeFilter::HashKey(target_date_2.to_string())),
    );
    let result_for_first_post = processor.execute_query_map(query).await
    .expect("Failed to execute query for first post");
    
    // Verify that only "First Post" is returned
    let title_field_results = result_for_first_post.get("title").unwrap();
    assert_eq!(title_field_results.len(), 1, "Should have exactly 1 record for title field");
    
    let (key, field_value) = title_field_results.iter().next().unwrap();
    assert_eq!(key.range, Some(target_date_2.to_string()), "Range key should be '{}'", target_date_2);
    assert_eq!(field_value.value, json!("First Post"), "Title should be 'First Post'");
    
    // Test 3: Query with non-existent range key should return empty results
    let non_existent_date = "2024-12-31";
    let query = Query::new_with_filter(
        "BlogPost".to_string(),
        vec!["title".to_string(), "author".to_string(), "publish_date".to_string()],
        Some(HashRangeFilter::HashKey(non_existent_date.to_string())),
    );
    let result_non_existent = processor.execute_query_map(query).await
    .expect("Failed to execute query for non-existent record");
    
    // Verify that no results are returned
    for (field_name, field_results) in &result_non_existent {
        assert_eq!(field_results.len(), 0, "Field '{}' should have no results for non-existent key '{}'", field_name, non_existent_date);
    }
    
    // Test 4: Query without filter should return all records
    let query = Query::new(
        "BlogPost".to_string(),
        vec!["title".to_string(), "author".to_string(), "publish_date".to_string()],
    );
    let result_all = processor.execute_query_map(query).await
    .expect("Failed to execute query for all records");
    
    // Verify that all 3 records are returned
    for (field_name, field_results) in &result_all {
        assert_eq!(field_results.len(), 3, "Field '{}' should have 3 results", field_name);
    }
    
    println!("✅ All exact range key filtering tests passed!");
}

/// Test that rangeKey is properly set in the query object structure
#[tokio::test]
async fn test_range_key_set_in_query_object() {
    // Set up temporary database
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize node with temporary database
    let config = NodeConfig::new(temp_db_path.into());
    let node = DataFoldNode::new(config)
        .expect("Failed to create DataFoldNode");
    
    // Wrap node in Arc<Mutex<>> for OperationProcessor
    let node_arc = Arc::new(tokio::sync::Mutex::new(node));
    let processor = OperationProcessor::new(node_arc);
    
    // Create test blog post
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("title".to_string(), json!("Test Post"));
    fields_and_values.insert("author".to_string(), json!("Test Author"));
    fields_and_values.insert("publish_date".to_string(), json!("2024-01-15"));
    fields_and_values.insert("tags".to_string(), json!(vec!["test", "example"]));
    fields_and_values.insert("content".to_string(), json!("This is test content"));
    
    processor.execute_mutation(
        "BlogPost".to_string(),
        fields_and_values,
        KeyValue::new(None, Some("2024-01-15".to_string())),
        MutationType::Create,
    ).await
    .expect("Failed to execute mutation");
    
    // Create query with exact range key filter
    let target_date = "2024-01-15";
    let query = Query::new_with_filter(
        "BlogPost".to_string(),
        vec!["title".to_string(), "author".to_string(), "publish_date".to_string()],
        Some(HashRangeFilter::HashKey(target_date.to_string())),
    );
    let result = processor.execute_query_map(query).await
    .expect("Failed to execute query");
    
    // Verify that the range key is properly set in the returned key structure
    for (field_name, field_results) in &result {
        assert_eq!(field_results.len(), 1, "Field '{}' should have exactly 1 result", field_name);
        
        let (key, _field_value) = field_results.iter().next().unwrap();
        assert_eq!(key.range, Some(target_date.to_string()), 
                  "Range key should be '{}' for field '{}'", target_date, field_name);
        assert!(key.hash.is_none(), "Hash should be None for Range schema in field '{}'", field_name);
    }
    
    println!("✅ Range key properly set in query object test passed!");
}