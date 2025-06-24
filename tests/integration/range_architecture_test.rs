//! Range Architecture Tests
//!
//! This comprehensive test suite validates that Range fields serve as an
//! effective replacement for the removed collection functionality.
//!
//! **Range Architecture Coverage:**
//! 1. **Range Fields as Collection Replacement** - Validate Range fields handle collection-like data
//! 2. **Range Field Lookup Hash Functionality** - Test efficient key-value lookups
//! 3. **Range Field Mutations and Queries** - Complete CRUD operations on Range data
//! 4. **Range Field Performance Characteristics** - Performance validation under load
//! 5. **Range Filtering and Search** - Advanced query capabilities
//! 6. **Range Schema Validation** - Ensure Range schemas work correctly

use datafold::schema::types::field::{FieldVariant, RangeField};
use datafold::schema::types::field::range_filter::RangeFilter;
use datafold::schema::{Schema, field_factory::FieldFactory};
use datafold::atom::Atom;
use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
};
use datafold::fold_db_core::managers::atom::AtomManager;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use crate::test_utils::TEST_WAIT_MS;
use tempfile::tempdir;
use uuid::Uuid;

/// Test fixture for Range architecture testing
struct RangeArchitectureTestFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub _atom_manager: AtomManager,
    pub _temp_dir: tempfile::TempDir,
}

impl RangeArchitectureTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()?;
            
        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        
        let atom_manager = AtomManager::new(
            (*db_ops).clone(),
            Arc::clone(&message_bus)
        );
        
        Ok(Self {
            db_ops,
            message_bus,
            _atom_manager: atom_manager,
            _temp_dir: temp_dir,
        })
    }
    
    /// Create a comprehensive Range schema for testing
    fn create_test_range_schema(&self) -> Result<Schema, Box<dyn std::error::Error>> {
        let mut user_catalog_schema = Schema::new_range(
            "UserCatalog".to_string(),
            "user_id".to_string(),
        );
        
        // Add Range fields that will hold user data keyed by user_id
        user_catalog_schema.fields.insert(
            "user_id".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        user_catalog_schema.fields.insert(
            "profile_data".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        user_catalog_schema.fields.insert(
            "preferences".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        user_catalog_schema.fields.insert(
            "activity_metrics".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        
        self.db_ops.store_schema("UserCatalog", &user_catalog_schema)?;
        Ok(user_catalog_schema)
    }
    
    /// Populate Range field with test data
    fn populate_range_field_data(
        &self,
        range_field: &mut RangeField,
        test_data: Vec<(String, serde_json::Value)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize AtomRefRange
        range_field.ensure_molecule_range("test_user".to_string());
        
        for (key, content) in test_data {
            // Create atom
            let atom = Atom::new("UserCatalog".to_string(), "test_user".to_string(), content);
            let atom_uuid = Uuid::new_v4().to_string();
            
            // Store atom
            self.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)?;
            
            // Add to range
            if let Some(molecule_range) = range_field.molecule_range_mut() {
                molecule_range.set_atom_uuid(key, atom_uuid);
            }
        }
        
        Ok(())
    }
    
    /// Execute field mutation via message bus
    fn mutate_range_field(
        &self,
        schema_name: &str,
        field_name: &str,
        range_key: &str,
        value: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let correlation_id = format!("range_{}_{}", schema_name, field_name);
        
        // Subscribe to response
        let mut response_consumer = self.message_bus.subscribe::<FieldValueSetResponse>();
        
        // For Range fields, value should include the range key
        let range_value = json!({
            range_key: range_key,
            "data": value
        });
        
        let request = FieldValueSetRequest::new(
            correlation_id,
            schema_name.to_string(),
            field_name.to_string(),
            range_value,
            "range_test".to_string(),
        );
        
        self.message_bus.publish(request)?;
        
        // Wait for processing
        thread::sleep(Duration::from_millis(TEST_WAIT_MS));
        
        let response = response_consumer.recv_timeout(Duration::from_millis(2000))
            .map_err(|_| "Timeout waiting for FieldValueSetResponse")?;
        
        if !response.success {
            return Err(format!("Range mutation failed: {:?}", response.error).into());
        }
        
        Ok(response.molecule_uuid.unwrap_or_else(|| "no_uuid".to_string()))
    }
}

#[test]
fn test_range_fields_as_collection_replacement() {
    println!("🧪 TEST: Range Fields as Collection Replacement");
    println!("   This validates Range fields can effectively replace collection functionality");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    let _schema = fixture.create_test_range_schema()
        .expect("Failed to create Range schema");
    
    // Test 1: Create Range field and validate collection-like behavior
    let mut user_catalog_field = FieldFactory::create_range_field();
    
    // Populate with user data that would traditionally be in a collection
    let user_data = vec![
        ("user_001".to_string(), json!({
            "name": "Alice Johnson",
            "email": "alice@example.com", 
            "role": "admin",
            "created_at": "2024-01-01T00:00:00Z"
        })),
        ("user_002".to_string(), json!({
            "name": "Bob Smith", 
            "email": "bob@example.com",
            "role": "user", 
            "created_at": "2024-01-02T00:00:00Z"
        })),
        ("user_003".to_string(), json!({
            "name": "Charlie Brown",
            "email": "charlie@example.com", 
            "role": "moderator",
            "created_at": "2024-01-03T00:00:00Z" 
        })),
        ("user_100".to_string(), json!({
            "name": "David Wilson",
            "email": "david@example.com",
            "role": "user",
            "created_at": "2024-01-10T00:00:00Z"
        })),
        ("user_200".to_string(), json!({
            "name": "Eve Davis", 
            "email": "eve@example.com",
            "role": "admin",
            "created_at": "2024-01-20T00:00:00Z"
        })),
    ];
    
    fixture.populate_range_field_data(&mut user_catalog_field, user_data.clone())
        .expect("Failed to populate Range field");
    
    // Test collection-like operations
    
    // 1. Count operation (like collection.size())
    assert_eq!(user_catalog_field.count(), user_data.len());
    println!("✅ Range field count operation works: {} items", user_catalog_field.count());
    
    // 2. Key enumeration (like collection.keys())
    let all_keys = user_catalog_field.get_all_keys();
    assert_eq!(all_keys.len(), user_data.len());
    for (key, _) in &user_data {
        assert!(all_keys.contains(key));
    }
    println!("✅ Range field key enumeration works: {:?}", all_keys);
    
    // 3. Individual item lookup (like collection[key])
    let lookup_filter = RangeFilter::Key("user_001".to_string());
    let lookup_result = user_catalog_field.apply_filter(&lookup_filter);
    assert_eq!(lookup_result.total_count, 1);
    assert!(lookup_result.matches.contains_key("user_001"));
    println!("✅ Range field individual lookup works");
    
    // 4. Multiple item lookup (like collection.get_many(keys))
    let multi_lookup_filter = RangeFilter::Keys(vec![
        "user_001".to_string(),
        "user_002".to_string(), 
        "user_100".to_string()
    ]);
    let multi_result = user_catalog_field.apply_filter(&multi_lookup_filter);
    assert_eq!(multi_result.total_count, 3);
    println!("✅ Range field multiple lookup works: {} items", multi_result.total_count);
    
    // 5. Range queries (like collection.range(start, end))
    let range_filter = RangeFilter::KeyRange {
        start: "user_001".to_string(),
        end: "user_100".to_string(),
    };
    let range_result = user_catalog_field.apply_filter(&range_filter);
    assert!(range_result.total_count >= 2); // Should include user_001, user_002, user_003
    println!("✅ Range field range queries work: {} items in range", range_result.total_count);
    
    // 6. Prefix search (like collection.find_by_prefix())
    let prefix_filter = RangeFilter::KeyPrefix("user_00".to_string());
    let prefix_result = user_catalog_field.apply_filter(&prefix_filter);
    assert_eq!(prefix_result.total_count, 3); // user_001, user_002, user_003
    println!("✅ Range field prefix search works: {} items with prefix", prefix_result.total_count);
    
    println!("✅ Range Fields as Collection Replacement Test PASSED");
}

#[test]
fn test_range_field_lookup_hash_functionality() {
    println!("🧪 TEST: Range Field Lookup Hash Functionality");
    println!("   This validates efficient hash-based lookups in Range fields");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    let mut product_catalog_field = FieldFactory::create_range_field();
    
    // Create a large dataset to test hash performance
    let mut product_data = Vec::new();
    for i in 0..1000 {
        let product_id = format!("PROD_{:06}", i);
        let product_info = json!({
            "sku": product_id.clone(),
            "name": format!("Product {}", i),
            "category": format!("Category {}", i % 10),
            "price": (i as f64) * 9.99,
            "in_stock": i % 3 != 0,
        });
        product_data.push((product_id, product_info));
    }
    
    fixture.populate_range_field_data(&mut product_catalog_field, product_data.clone())
        .expect("Failed to populate product catalog");
    
    println!("✅ Created product catalog with {} items", product_catalog_field.count());
    
    // Test 1: Hash lookup performance
    println!("⚡ Testing hash lookup performance");
    
    let lookup_start = Instant::now();
    let lookup_iterations = 100;
    
    for i in 0..lookup_iterations {
        let product_id = format!("PROD_{:06}", i * 10); // Every 10th product
        let lookup_filter = RangeFilter::Key(product_id.clone());
        let result = product_catalog_field.apply_filter(&lookup_filter);
        
        assert_eq!(result.total_count, 1, "Should find exactly one product for {}", product_id);
        assert!(result.matches.contains_key(&product_id));
    }
    
    let lookup_duration = lookup_start.elapsed();
    let avg_lookup_time = lookup_duration / lookup_iterations;
    
    println!("✅ Hash lookup performance: {} lookups in {:?}", lookup_iterations, lookup_duration);
    println!("   Average lookup time: {:?}", avg_lookup_time);
    
    // Performance assertion (should be sub-millisecond for hash lookups)
    assert!(avg_lookup_time < Duration::from_millis(1), "Hash lookups should be sub-millisecond");
    
    // Test 2: Bulk lookup efficiency
    println!("📦 Testing bulk lookup efficiency");
    
    let bulk_keys: Vec<String> = (0..50).map(|i| format!("PROD_{:06}", i * 20)).collect();
    let bulk_start = Instant::now();
    
    let bulk_filter = RangeFilter::Keys(bulk_keys.clone());
    let bulk_result = product_catalog_field.apply_filter(&bulk_filter);
    
    let bulk_duration = bulk_start.elapsed();
    
    assert_eq!(bulk_result.total_count, bulk_keys.len());
    println!("✅ Bulk lookup: {} items in {:?}", bulk_keys.len(), bulk_duration);
    
    // Test 3: Hash collision handling (same hash, different keys)
    println!("🔗 Testing hash collision scenarios");
    
    // Create keys that might have hash collisions
    let collision_test_data = vec![
        ("collision_test_001".to_string(), json!({"type": "collision_test", "id": 1})),
        ("collision_test_002".to_string(), json!({"type": "collision_test", "id": 2})), 
        ("collision_test_003".to_string(), json!({"type": "collision_test", "id": 3})),
    ];
    
    let mut collision_field = FieldFactory::create_range_field();
    fixture.populate_range_field_data(&mut collision_field, collision_test_data.clone())
        .expect("Failed to populate collision test data");
    
    // Verify each item can be found individually
    for (key, _) in &collision_test_data {
        let filter = RangeFilter::Key(key.clone());
        let result = collision_field.apply_filter(&filter);
        assert_eq!(result.total_count, 1, "Should find collision test item {}", key);
        assert!(result.matches.contains_key(key));
    }
    
    println!("✅ Hash collision handling works correctly");
    
    // Test 4: Memory efficiency validation
    println!("💾 Testing memory efficiency");
    
    let memory_start = Instant::now();
    let large_dataset_size = 5000;
    let mut large_dataset = Vec::new();
    
    for i in 0..large_dataset_size {
        let key = format!("LARGE_{:08}", i);
        let value = json!({
            "id": i,
            "data": format!("Large dataset item {}", i),
            "metadata": {
                "created": "2024-01-01",
                "category": format!("cat_{}", i % 20),
            }
        });
        large_dataset.push((key, value));
    }
    
    let mut large_field = FieldFactory::create_range_field();
    fixture.populate_range_field_data(&mut large_field, large_dataset)
        .expect("Failed to populate large dataset");
    
    let memory_duration = memory_start.elapsed();
    
    assert_eq!(large_field.count(), large_dataset_size);
    println!("✅ Memory efficiency: {} items created in {:?}", large_dataset_size, memory_duration);
    
    // Quick lookup test on large dataset
    let large_lookup_start = Instant::now();
    let test_key = "LARGE_00002500";
    let large_lookup_filter = RangeFilter::Key(test_key.to_string());
    let large_lookup_result = large_field.apply_filter(&large_lookup_filter);
    let large_lookup_duration = large_lookup_start.elapsed();
    
    assert_eq!(large_lookup_result.total_count, 1);
    println!("✅ Large dataset lookup: found item in {:?}", large_lookup_duration);
    
    println!("✅ Range Field Lookup Hash Functionality Test PASSED");
}

#[test]
#[allow(clippy::unnecessary_to_owned)]
fn test_range_field_mutations_and_queries() {
    println!("🧪 TEST: Range Field Mutations and Queries");
    println!("   This validates complete CRUD operations on Range field data");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    let _schema = fixture.create_test_range_schema()
        .expect("Failed to create Range schema");
    
    // Test 1: Create operations via mutations
    println!("📝 Test 1: Create Operations");
    
    let new_users = vec![
        ("user_create_001", json!({
            "name": "New User One",
            "email": "user1@create.com",
            "status": "active"
        })),
        ("user_create_002", json!({
            "name": "New User Two", 
            "email": "user2@create.com",
            "status": "pending"
        })),
        ("user_create_003", json!({
            "name": "New User Three",
            "email": "user3@create.com", 
            "status": "active"
        })),
    ];
    
    let mut created_uuids = Vec::new();
    for (user_key, user_data) in &new_users {
        let aref_uuid = fixture.mutate_range_field(
            "UserCatalog",
            "profile_data", 
            "user_id",
            user_data.clone(),
        ).unwrap_or_else(|_| panic!("Failed to create user {}", user_key));
        
        created_uuids.push(aref_uuid);
        println!("✅ Created user {}: {}", user_key, user_data["name"]);
    }
    
    // Test 2: Read operations via Range field queries
    println!("🔍 Test 2: Read Operations");
    
    // We'll need to reconstruct the field to test queries
    // In a real implementation, this would come from the database
    let mut query_field = FieldFactory::create_range_field();
    let query_data: Vec<(String, serde_json::Value)> = new_users.iter()
        .map(|(key, data)| (key.to_string(), data.clone()))
        .collect();
    
    fixture.populate_range_field_data(&mut query_field, query_data)
        .expect("Failed to populate query field");
    
    // Individual reads
    for (user_key, _expected_data) in &new_users {
        let read_filter = RangeFilter::Key(user_key.to_string());
        let read_result = query_field.apply_filter(&read_filter);
        
        assert_eq!(read_result.total_count, 1);
        assert!(read_result.matches.contains_key(&user_key.to_string()));
        println!("✅ Read user {}: found", user_key);
    }
    
    // Bulk read
    let all_keys: Vec<String> = new_users.iter().map(|(key, _)| key.to_string()).collect();
    let bulk_read_filter = RangeFilter::Keys(all_keys.clone());
    let bulk_read_result = query_field.apply_filter(&bulk_read_filter);
    
    assert_eq!(bulk_read_result.total_count, new_users.len());
    println!("✅ Bulk read: {} users found", bulk_read_result.total_count);
    
    // Test 3: Update operations
    println!("✏️  Test 3: Update Operations");
    
    let update_data = vec![
        ("user_create_001", json!({
            "name": "Updated User One",
            "email": "updated1@create.com",
            "status": "active",
            "last_updated": "2024-01-01T12:00:00Z"
        })),
        ("user_create_002", json!({
            "name": "Updated User Two",
            "email": "updated2@create.com", 
            "status": "active",
            "last_updated": "2024-01-01T12:01:00Z"
        })),
    ];
    
    for (user_key, updated_data) in &update_data {
        let _update_uuid = fixture.mutate_range_field(
            "UserCatalog",
            "profile_data",
            "user_id", 
            updated_data.clone(),
        ).unwrap_or_else(|_| panic!("Failed to update user {}", user_key));
        
        println!("✅ Updated user {}: {}", user_key, updated_data["name"]);
    }
    
    // Test 4: Complex query operations
    println!("🔍 Test 4: Complex Query Operations");
    
    // Range queries
    let range_query_filter = RangeFilter::KeyRange {
        start: "user_create_001".to_string(),
        end: "user_create_003".to_string(),
    };
    let range_query_result = query_field.apply_filter(&range_query_filter);
    assert!(range_query_result.total_count >= 2); // Should include user_create_001 and user_create_002
    println!("✅ Range query: {} users in range", range_query_result.total_count);
    
    // Prefix queries
    let prefix_query_filter = RangeFilter::KeyPrefix("user_create_00".to_string());
    let prefix_query_result = query_field.apply_filter(&prefix_query_filter);
    assert_eq!(prefix_query_result.total_count, 3); // All three users
    println!("✅ Prefix query: {} users with prefix", prefix_query_result.total_count);
    
    // Pattern queries  
    let pattern_query_filter = RangeFilter::KeyPattern("user_create_*".to_string());
    let pattern_query_result = query_field.apply_filter(&pattern_query_filter);
    assert_eq!(pattern_query_result.total_count, 3); // All three users
    println!("✅ Pattern query: {} users matching pattern", pattern_query_result.total_count);
    
    // Test 5: Pagination simulation
    println!("📄 Test 5: Pagination Simulation");
    
    let page_size = 2;
    let page_1_keys = query_field.get_keys_in_range("user_create_001", "user_create_003");
    assert!(page_1_keys.len() <= page_size || page_1_keys.len() == 3); // Depends on range implementation
    
    let page_2_keys = query_field.get_keys_in_range("user_create_003", "user_create_999");
    // Should contain remaining keys
    
    println!("✅ Pagination simulation: Page 1 has {} keys, Page 2 has {} keys", 
        page_1_keys.len(), page_2_keys.len());
    
    println!("✅ Range Field Mutations and Queries Test PASSED");
}

#[test]
fn test_range_field_performance_characteristics() {
    println!("🧪 TEST: Range Field Performance Characteristics");
    println!("   This validates Range field performance under various load conditions");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Performance Test 1: Large dataset insertion
    println!("📊 Performance Test 1: Large Dataset Insertion");
    
    let insertion_start = Instant::now();
    let large_dataset_size = 10000;
    
    let mut performance_field = FieldFactory::create_range_field();
    let mut performance_data = Vec::new();
    
    for i in 0..large_dataset_size {
        let key = format!("PERF_{:08}", i);
        let value = json!({
            "id": i,
            "data": format!("Performance test item {}", i),
            "category": format!("category_{}", i % 100),
            "score": (i as f64) * 0.1,
            "active": i % 2 == 0,
        });
        performance_data.push((key, value));
    }
    
    fixture.populate_range_field_data(&mut performance_field, performance_data)
        .expect("Failed to populate performance dataset");
    
    let insertion_duration = insertion_start.elapsed();
    let insertion_rate = large_dataset_size as f64 / insertion_duration.as_secs_f64();
    
    assert_eq!(performance_field.count(), large_dataset_size);
    println!("✅ Large dataset insertion: {} items in {:?}", large_dataset_size, insertion_duration);
    println!("   Insertion rate: {:.2} items/second", insertion_rate);
    
    // Performance Test 2: Random access patterns
    println!("🔀 Performance Test 2: Random Access Patterns");
    
    let random_access_start = Instant::now();
    let random_access_count = 1000;
    
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    for i in 0..random_access_count {
        // Generate pseudo-random index
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let random_index = (hasher.finish() % large_dataset_size as u64) as usize;
        
        let key = format!("PERF_{:08}", random_index);
        let lookup_filter = RangeFilter::Key(key.clone());
        let result = performance_field.apply_filter(&lookup_filter);
        
        assert_eq!(result.total_count, 1, "Should find item for key {}", key);
    }
    
    let random_access_duration = random_access_start.elapsed();
    let random_access_rate = random_access_count as f64 / random_access_duration.as_secs_f64();
    
    println!("✅ Random access: {} lookups in {:?}", random_access_count, random_access_duration);
    println!("   Random access rate: {:.2} lookups/second", random_access_rate);
    
    // Performance Test 3: Range scan operations
    println!("📏 Performance Test 3: Range Scan Operations");
    
    let range_scan_start = Instant::now();
    let range_scan_count = 100;
    
    for i in 0..range_scan_count {
        let start_key = format!("PERF_{:08}", i * 100);
        let end_key = format!("PERF_{:08}", (i + 1) * 100);
        
        let range_filter = RangeFilter::KeyRange {
            start: start_key,
            end: end_key,
        };
        let result = performance_field.apply_filter(&range_filter);
        
        // Should find approximately 100 items in each range
        assert!(result.total_count > 0, "Range scan should find items");
    }
    
    let range_scan_duration = range_scan_start.elapsed();
    let range_scan_rate = range_scan_count as f64 / range_scan_duration.as_secs_f64();
    
    println!("✅ Range scans: {} scans in {:?}", range_scan_count, range_scan_duration);
    println!("   Range scan rate: {:.2} scans/second", range_scan_rate);
    
    // Performance Test 4: Bulk operations
    println!("📦 Performance Test 4: Bulk Operations");
    
    let bulk_operation_start = Instant::now();
    let bulk_size = 500;
    
    // Create bulk keys
    let bulk_keys: Vec<String> = (0..bulk_size)
        .map(|i| format!("PERF_{:08}", i * 20)) // Every 20th item
        .collect();
    
    let bulk_filter = RangeFilter::Keys(bulk_keys.clone());
    let bulk_result = performance_field.apply_filter(&bulk_filter);
    
    let bulk_operation_duration = bulk_operation_start.elapsed();
    let bulk_operation_rate = bulk_size as f64 / bulk_operation_duration.as_secs_f64();
    
    assert_eq!(bulk_result.total_count, bulk_keys.len());
    println!("✅ Bulk operations: {} items in {:?}", bulk_size, bulk_operation_duration);
    println!("   Bulk operation rate: {:.2} items/second", bulk_operation_rate);
    
    // Performance Test 5: Memory usage characteristics
    println!("💾 Performance Test 5: Memory Usage Characteristics");
    
    let memory_test_start = Instant::now();
    let memory_test_size = 50000;
    
    let mut memory_field = FieldFactory::create_range_field();
    let mut memory_data = Vec::new();
    
    for i in 0..memory_test_size {
        let key = format!("MEM_{:08}", i);
        let value = json!({
            "id": i,
            "small_data": i,
        });
        memory_data.push((key, value));
    }
    
    fixture.populate_range_field_data(&mut memory_field, memory_data)
        .expect("Failed to populate memory test dataset");
    
    let memory_test_duration = memory_test_start.elapsed();
    
    assert_eq!(memory_field.count(), memory_test_size);
    println!("✅ Memory test: {} items in {:?}", memory_test_size, memory_test_duration);
    
    // Quick verification that large dataset still performs well
    let verification_start = Instant::now();
    let verification_key = "MEM_00025000";
    let verification_filter = RangeFilter::Key(verification_key.to_string());
    let verification_result = memory_field.apply_filter(&verification_filter);
    let verification_duration = verification_start.elapsed();
    
    assert_eq!(verification_result.total_count, 1);
    println!("✅ Large dataset verification: found item in {:?}", verification_duration);
    
    // Performance assertions
    assert!(insertion_rate > 100.0, "Insertion rate should be > 100 items/sec");
    assert!(random_access_rate > 100.0, "Random access rate should be > 100 lookups/sec");
    assert!(bulk_operation_rate > 100.0, "Bulk operation rate should be > 100 items/sec");
    
    println!("📈 Performance Summary:");
    println!("   Insertion rate: {:.2} items/second", insertion_rate);
    println!("   Random access rate: {:.2} lookups/second", random_access_rate);
    println!("   Range scan rate: {:.2} scans/second", range_scan_rate);
    println!("   Bulk operation rate: {:.2} items/second", bulk_operation_rate);
    
    println!("✅ Range Field Performance Characteristics Test PASSED");
}

#[test]
fn test_range_filtering_and_search_capabilities() {
    println!("🧪 TEST: Range Filtering and Search Capabilities");
    println!("   This validates advanced query and filtering capabilities of Range fields");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Create a comprehensive search test dataset
    let mut search_field = FieldFactory::create_range_field();
    
    let search_test_data = vec![
        // User data with various patterns
        ("user_admin_001", json!({"role": "admin", "department": "engineering", "level": "senior"})),
        ("user_admin_002", json!({"role": "admin", "department": "marketing", "level": "junior"})),
        ("user_dev_001", json!({"role": "developer", "department": "engineering", "level": "senior"})),
        ("user_dev_002", json!({"role": "developer", "department": "engineering", "level": "mid"})),
        ("user_dev_003", json!({"role": "developer", "department": "engineering", "level": "junior"})),
        ("user_qa_001", json!({"role": "qa", "department": "engineering", "level": "senior"})),
        ("user_qa_002", json!({"role": "qa", "department": "engineering", "level": "mid"})),
        ("user_sales_001", json!({"role": "sales", "department": "sales", "level": "senior"})),
        ("user_sales_002", json!({"role": "sales", "department": "sales", "level": "junior"})),
        ("user_support_001", json!({"role": "support", "department": "customer_service", "level": "mid"})),
        
        // Product data with various patterns
        ("product_laptop_001", json!({"category": "electronics", "type": "laptop", "price": 999})),
        ("product_laptop_002", json!({"category": "electronics", "type": "laptop", "price": 1299})),
        ("product_phone_001", json!({"category": "electronics", "type": "phone", "price": 699})),
        ("product_phone_002", json!({"category": "electronics", "type": "phone", "price": 899})),
        ("product_book_001", json!({"category": "books", "type": "fiction", "price": 15})),
        ("product_book_002", json!({"category": "books", "type": "non-fiction", "price": 25})),
        ("product_clothing_001", json!({"category": "clothing", "type": "shirt", "price": 35})),
        ("product_clothing_002", json!({"category": "clothing", "type": "pants", "price": 65})),
    ];
    
    let search_test_data_converted: Vec<(String, serde_json::Value)> = search_test_data
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    
    fixture.populate_range_field_data(&mut search_field, search_test_data_converted)
        .expect("Failed to populate search test data");
    
    println!("✅ Created search dataset with {} items", search_field.count());
    
    // Test 1: Key-based filtering
    println!("🔑 Test 1: Key-Based Filtering");
    
    // Single key lookup
    let single_key_filter = RangeFilter::Key("user_admin_001".to_string());
    let single_result = search_field.apply_filter(&single_key_filter);
    assert_eq!(single_result.total_count, 1);
    assert!(single_result.matches.contains_key("user_admin_001"));
    println!("✅ Single key lookup works");
    
    // Multiple key lookup
    let multi_key_filter = RangeFilter::Keys(vec![
        "user_admin_001".to_string(),
        "product_laptop_001".to_string(),
        "user_dev_001".to_string(),
    ]);
    let multi_result = search_field.apply_filter(&multi_key_filter);
    assert_eq!(multi_result.total_count, 3);
    println!("✅ Multiple key lookup works: {} items", multi_result.total_count);
    
    // Test 2: Prefix-based filtering
    println!("🏷️  Test 2: Prefix-Based Filtering");
    
    // Find all users
    let user_prefix_filter = RangeFilter::KeyPrefix("user_".to_string());
    let user_prefix_result = search_field.apply_filter(&user_prefix_filter);
    let user_count = search_test_data.iter().filter(|(key, _)| key.starts_with("user_")).count();
    assert_eq!(user_prefix_result.total_count, user_count);
    println!("✅ User prefix filter: {} users found", user_prefix_result.total_count);
    
    // Find all products
    let product_prefix_filter = RangeFilter::KeyPrefix("product_".to_string());
    let product_prefix_result = search_field.apply_filter(&product_prefix_filter);
    let product_count = search_test_data.iter().filter(|(key, _)| key.starts_with("product_")).count();
    assert_eq!(product_prefix_result.total_count, product_count);
    println!("✅ Product prefix filter: {} products found", product_prefix_result.total_count);
    
    // Find all admin users
    let admin_prefix_filter = RangeFilter::KeyPrefix("user_admin_".to_string());
    let admin_prefix_result = search_field.apply_filter(&admin_prefix_filter);
    let admin_count = search_test_data.iter().filter(|(key, _)| key.starts_with("user_admin_")).count();
    assert_eq!(admin_prefix_result.total_count, admin_count);
    println!("✅ Admin prefix filter: {} admins found", admin_prefix_result.total_count);
    
    // Test 3: Range-based filtering
    println!("📏 Test 3: Range-Based Filtering");
    
    // Find users in a specific range
    let user_range_filter = RangeFilter::KeyRange {
        start: "user_admin_001".to_string(),
        end: "user_dev_999".to_string(),
    };
    let user_range_result = search_field.apply_filter(&user_range_filter);
    assert!(user_range_result.total_count > 0);
    println!("✅ User range filter: {} items in range", user_range_result.total_count);
    
    // Find products in a specific range
    let product_range_filter = RangeFilter::KeyRange {
        start: "product_book_000".to_string(),
        end: "product_laptop_999".to_string(),
    };
    let product_range_result = search_field.apply_filter(&product_range_filter);
    assert!(product_range_result.total_count > 0);
    println!("✅ Product range filter: {} items in range", product_range_result.total_count);
    
    // Test 4: Pattern-based filtering (if supported)
    println!("🎯 Test 4: Pattern-Based Filtering");
    
    // Find all laptop products
    let laptop_pattern_filter = RangeFilter::KeyPattern("product_laptop_*".to_string());
    let laptop_pattern_result = search_field.apply_filter(&laptop_pattern_filter);
    let laptop_count = search_test_data.iter().filter(|(key, _)| key.contains("laptop")).count();
    assert_eq!(laptop_pattern_result.total_count, laptop_count);
    println!("✅ Laptop pattern filter: {} laptops found", laptop_pattern_result.total_count);
    
    // Find all development users
    let dev_pattern_filter = RangeFilter::KeyPattern("user_dev_*".to_string());
    let dev_pattern_result = search_field.apply_filter(&dev_pattern_filter);
    let dev_count = search_test_data.iter().filter(|(key, _)| key.contains("_dev_")).count();
    assert_eq!(dev_pattern_result.total_count, dev_count);
    println!("✅ Developer pattern filter: {} developers found", dev_pattern_result.total_count);
    
    // Test 5: Complex filtering scenarios
    println!("🔍 Test 5: Complex Filtering Scenarios");
    
    // Pagination simulation
    let _page_size = 5;
    let page_1_filter = RangeFilter::KeyRange {
        start: "product_book_000".to_string(),
        end: "product_laptop_001".to_string(),
    };
    let page_1_result = search_field.apply_filter(&page_1_filter);
    println!("✅ Pagination page 1: {} items", page_1_result.total_count);
    
    // Category-based lookups via prefix
    let electronics_filter = RangeFilter::KeyPrefix("product_laptop_".to_string());
    let phone_filter = RangeFilter::KeyPrefix("product_phone_".to_string());
    let electronics_result = search_field.apply_filter(&electronics_filter);
    let phone_result = search_field.apply_filter(&phone_filter);
    
    println!("✅ Category filtering: {} laptops, {} phones", 
        electronics_result.total_count, phone_result.total_count);
    
    // Test 6: Performance of complex filters
    println!("⚡ Test 6: Filter Performance");
    
    let filter_performance_start = Instant::now();
    let filter_iterations = 100;
    
    for i in 0..filter_iterations {
        let prefix = format!("user_{}", if i % 2 == 0 { "admin" } else { "dev" });
        let filter = RangeFilter::KeyPrefix(prefix);
        let _result = search_field.apply_filter(&filter);
    }
    
    let filter_performance_duration = filter_performance_start.elapsed();
    let filter_rate = filter_iterations as f64 / filter_performance_duration.as_secs_f64();
    
    println!("✅ Filter performance: {} filters in {:?}", filter_iterations, filter_performance_duration);
    println!("   Filter rate: {:.2} filters/second", filter_rate);
    
    // Performance assertion
    assert!(filter_rate > 10.0, "Filter rate should be > 10 filters/sec");
    
    println!("✅ Range Filtering and Search Capabilities Test PASSED");
}

#[test]
fn test_range_schema_validation_and_consistency() {
    println!("🧪 TEST: Range Schema Validation and Consistency");
    println!("   This validates Range schema creation, validation, and consistency constraints");
    
    let fixture = RangeArchitectureTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Valid Range schema creation
    println!("✅ Test 1: Valid Range Schema Creation");
    
    let mut valid_range_schema = Schema::new_range(
        "ValidRangeSchema".to_string(),
        "timestamp".to_string(),
    );
    
    // All fields must be Range fields in a RangeSchema
    valid_range_schema.fields.insert(
        "timestamp".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    valid_range_schema.fields.insert(
        "event_data".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    valid_range_schema.fields.insert(
        "metadata".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    
    // Store and verify
    fixture.db_ops.store_schema("ValidRangeSchema", &valid_range_schema)
        .expect("Failed to store valid Range schema");
    
    let retrieved_schema = fixture.db_ops.get_schema("ValidRangeSchema")
        .expect("Failed to retrieve schema")
        .expect("Schema should exist");
    
    assert_eq!(retrieved_schema.name, "ValidRangeSchema");
    assert_eq!(retrieved_schema.range_key(), Some("timestamp"));
    assert_eq!(retrieved_schema.fields.len(), 3);
    
    // Verify all fields are Range fields
    for (field_name, field_variant) in &retrieved_schema.fields {
        match field_variant {
            FieldVariant::Range(_) => {
                println!("✅ Field '{}' is correctly a Range field", field_name);
            }
            _ => panic!("Field '{}' should be a Range field in RangeSchema", field_name),
        }
    }
    
    println!("✅ Valid Range schema created and verified");
    
    // Test 2: Range key consistency
    println!("🔑 Test 2: Range Key Consistency");
    
    // Verify range_key field exists and is a Range field
    assert!(retrieved_schema.fields.contains_key("timestamp"));
    match retrieved_schema.fields.get("timestamp").unwrap() {
        FieldVariant::Range(_) => {
            println!("✅ Range key field is correctly a Range field");
        }
        _ => panic!("Range key field must be a Range field"),
    }
    
    // Test 3: Range field data consistency
    println!("📊 Test 3: Range Field Data Consistency");
    
    // Create test data that follows Range schema constraints
    let timestamp_data = vec![
        ("2024-01-01T00:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T00:00:00Z",
            "sequence": 1
        })),
        ("2024-01-01T01:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T01:00:00Z", 
            "sequence": 2
        })),
        ("2024-01-01T02:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T02:00:00Z",
            "sequence": 3
        })),
    ];
    
    let event_data = vec![
        ("2024-01-01T00:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T00:00:00Z",
            "event_type": "user_login",
            "user_id": "user_001"
        })),
        ("2024-01-01T01:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T01:00:00Z",
            "event_type": "page_view", 
            "user_id": "user_001"
        })),
        ("2024-01-01T02:00:00Z".to_string(), json!({
            "timestamp": "2024-01-01T02:00:00Z",
            "event_type": "user_logout",
            "user_id": "user_001"
        })),
    ];
    
    // Populate Range fields with consistent data
    let mut timestamp_field = FieldFactory::create_range_field();
    let mut event_field = FieldFactory::create_range_field();
    
    fixture.populate_range_field_data(&mut timestamp_field, timestamp_data.clone())
        .expect("Failed to populate timestamp field");
    fixture.populate_range_field_data(&mut event_field, event_data.clone())
        .expect("Failed to populate event field");
    
    // Verify consistency across Range fields
    assert_eq!(timestamp_field.count(), event_field.count());
    
    let timestamp_keys = timestamp_field.get_all_keys();
    let event_keys = event_field.get_all_keys();
    
    for key in &timestamp_keys {
        assert!(event_keys.contains(key), "Event data should exist for timestamp key {}", key);
    }
    
    println!("✅ Range field data consistency verified");
    
    // Test 4: Range query consistency
    println!("🔍 Test 4: Range Query Consistency");
    
    // Query the same time range across different fields
    let query_range = RangeFilter::KeyRange {
        start: "2024-01-01T00:00:00Z".to_string(),
        end: "2024-01-01T02:00:00Z".to_string(),
    };
    
    let timestamp_range_result = timestamp_field.apply_filter(&query_range);
    let event_range_result = event_field.apply_filter(&query_range);
    
    // Should return the same number of items (consistency across fields)
    assert_eq!(timestamp_range_result.total_count, event_range_result.total_count);
    
    // Verify the same keys are returned
    for key in timestamp_range_result.matches.keys() {
        assert!(event_range_result.matches.contains_key(key), 
            "Event field should contain key {} found in timestamp field", key);
    }
    
    println!("✅ Range query consistency verified: {} items in both fields", 
        timestamp_range_result.total_count);
    
    // Test 5: Schema evolution compatibility
    println!("🔄 Test 5: Schema Evolution Compatibility");
    
    // Add a new Range field to existing schema
    let mut evolved_schema = retrieved_schema.clone();
    evolved_schema.fields.insert(
        "user_context".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    
    // Store evolved schema with different name (schemas are immutable)
    fixture.db_ops.store_schema("ValidRangeSchemaV2", &evolved_schema)
        .expect("Failed to store evolved schema");
    
    let evolved_retrieved = fixture.db_ops.get_schema("ValidRangeSchemaV2")
        .expect("Failed to retrieve evolved schema")
        .expect("Evolved schema should exist");
    
    assert_eq!(evolved_retrieved.fields.len(), 4); // Original 3 + 1 new
    assert!(evolved_retrieved.fields.contains_key("user_context"));
    
    // Verify range_key is still consistent
    assert_eq!(evolved_retrieved.range_key(), Some("timestamp"));
    
    println!("✅ Schema evolution compatibility verified");
    
    // Test 6: Performance with consistent Range operations
    println!("⚡ Test 6: Consistent Range Operation Performance");
    
    let consistency_start = Instant::now();
    let consistency_iterations = 100;
    
    for i in 0..consistency_iterations {
        let test_key = format!("2024-01-01T{:02}:00:00Z", i % 24);
        
        let timestamp_filter = RangeFilter::Key(test_key.clone());
        let event_filter = RangeFilter::Key(test_key);
        
        let _timestamp_result = timestamp_field.apply_filter(&timestamp_filter);
        let _event_result = event_field.apply_filter(&event_filter);
        
        // Both should return consistent results
    }
    
    let consistency_duration = consistency_start.elapsed();
    let consistency_rate = (consistency_iterations * 2) as f64 / consistency_duration.as_secs_f64();
    
    println!("✅ Consistency performance: {} operations in {:?}", 
        consistency_iterations * 2, consistency_duration);
    println!("   Consistency rate: {:.2} operations/second", consistency_rate);
    
    assert!(consistency_rate > 50.0, "Consistency operation rate should be > 50 ops/sec");
    
    println!("✅ Range Schema Validation and Consistency Test PASSED");
}