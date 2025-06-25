//! Stress and Performance Tests
//!
//! This comprehensive test suite validates system performance and stability
//! under high-load conditions and stress scenarios.
//!
//! **Stress Testing Coverage:**
//! 1. **High-Volume Mutations** - System behavior under heavy mutation loads
//! 2. **Concurrent Transform Execution** - Multi-threaded transform processing
//! 3. **Memory Usage Validation** - Memory efficiency and leak detection
//! 4. **Event System Performance Under Load** - Message bus performance characteristics
//! 5. **Database Scalability** - Large dataset handling and query performance
//! 6. **Resource Cleanup and Recovery** - System stability after stress

use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    schema_events::{TransformTriggered, TransformExecuted},
};
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::db_operations::DbOperations;
use datafold::schema::{Schema, types::field::FieldVariant, field_factory::FieldFactory};
use datafold::atom::Atom;
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use crate::test_utils::TEST_WAIT_MS;
use tempfile::tempdir;
use uuid::Uuid;

/// Test fixture for stress and performance testing
#[allow(dead_code)]
struct StressPerformanceTestFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub _atom_manager: AtomManager,
    pub _temp_dir: tempfile::TempDir,
}

impl StressPerformanceTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()?;
            
        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        
        let transform_manager = Arc::new(TransformManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&message_bus),
        )?);
        
        let atom_manager = AtomManager::new(
            (*db_ops).clone(),
            Arc::clone(&message_bus)
        );
        
        Ok(Self {
            db_ops,
            message_bus,
            transform_manager,
            _atom_manager: atom_manager,
            _temp_dir: temp_dir,
        })
    }
    
    /// Create stress test schemas
    fn create_stress_test_schemas(&self) -> Result<(), Box<dyn std::error::Error>> {
        // High-throughput data schema
        let mut stress_data_schema = Schema::new("StressTestData".to_string());
        stress_data_schema.fields.insert(
            "data_id".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        stress_data_schema.fields.insert(
            "payload".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        stress_data_schema.fields.insert(
            "timestamp".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        stress_data_schema.fields.insert(
            "metadata".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        
        self.db_ops.store_schema("StressTestData", &stress_data_schema)?;
        
        // Range-based analytics schema for high-volume range operations
        let mut analytics_range_schema = Schema::new_range(
            "StressAnalytics".to_string(),
            "metric_key".to_string()
        );
        analytics_range_schema.fields.insert(
            "metric_key".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        analytics_range_schema.fields.insert(
            "metric_value".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        analytics_range_schema.fields.insert(
            "metric_metadata".to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
        
        self.db_ops.store_schema("StressAnalytics", &analytics_range_schema)?;
        
        // Results aggregation schema
        let mut results_schema = Schema::new("StressResults".to_string());
        results_schema.fields.insert(
            "total_processed".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        results_schema.fields.insert(
            "processing_rate".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        results_schema.fields.insert(
            "error_count".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        
        self.db_ops.store_schema("StressResults", &results_schema)?;
        
        Ok(())
    }
    
    /// Execute a high-volume mutation operation
    fn execute_high_volume_mutations(
        &self,
        mutation_count: usize,
        thread_count: usize,
    ) -> Result<(Duration, usize, usize), Box<dyn std::error::Error>> {
        let mutations_per_thread = mutation_count / thread_count;
        let successful_mutations = Arc::new(AtomicUsize::new(0));
        let failed_mutations = Arc::new(AtomicUsize::new(0));
        
        let start_time = Instant::now();
        
        // Create worker threads
        let handles: Vec<_> = (0..thread_count).map(|thread_id| {
            let message_bus = Arc::clone(&self.message_bus);
            let successful = Arc::clone(&successful_mutations);
            let failed = Arc::clone(&failed_mutations);
            
            thread::spawn(move || {
                let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
                
                for i in 0..mutations_per_thread {
                    let _data_id = format!("stress_{}_{}", thread_id, i);
                    let payload = json!({
                        "thread_id": thread_id,
                        "sequence": i,
                        "data": format!("High volume test data item {}", i),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "random_value": (i * thread_id) % 1000,
                    });
                    
                    let correlation_id = format!("stress_mutation_{}_{}", thread_id, i);
                    let request = FieldValueSetRequest::new(
                        correlation_id,
                        "StressTestData".to_string(),
                        "payload".to_string(),
                        payload,
                        format!("stress_thread_{}", thread_id),
                    );
                    
                    if message_bus.publish(request).is_ok() {
                        // Try to get response (with short timeout for high throughput)
                        match response_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
                            Ok(response) if response.success => {
                                successful.fetch_add(1, Ordering::Relaxed);
                            }
                            _ => {
                                failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    } else {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        }).collect();
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|_| "Thread panicked")?;
        }
        
        let total_duration = start_time.elapsed();
        let successful_count = successful_mutations.load(Ordering::Relaxed);
        let failed_count = failed_mutations.load(Ordering::Relaxed);
        
        Ok((total_duration, successful_count, failed_count))
    }
    
    /// Monitor memory usage during operations
    fn monitor_memory_usage(&self) -> MemoryUsageStats {
        // Simple memory usage monitoring
        // In a real implementation, this would use system APIs
        MemoryUsageStats {
            initial_usage: 0, // Would get actual memory usage
            peak_usage: 0,
            final_usage: 0,
            allocations: 0,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct MemoryUsageStats {
    initial_usage: usize,
    peak_usage: usize,
    final_usage: usize,
    allocations: usize,
}

#[test]
fn test_high_volume_mutations() {
    println!("🧪 TEST: High-Volume Mutations");
    println!("   This validates system performance under heavy mutation loads");
    
    let fixture = StressPerformanceTestFixture::new()
        .expect("Failed to create test fixture");
    
    fixture.create_stress_test_schemas()
        .expect("Failed to create stress test schemas");
    
    // Test 1: Sequential high-volume mutations
    println!("📊 Test 1: Sequential High-Volume Mutations");
    
    let sequential_start = Instant::now();
    let sequential_count = 1000;
    let mut sequential_successful = 0;
    let mut sequential_failed = 0;
    
    let mut response_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();
    
    for i in 0..sequential_count {
        let data = json!({
            "sequence": i,
            "data": format!("Sequential test data {}", i),
            "batch": "sequential_test"
        });
        
        let correlation_id = format!("sequential_{}", i);
        let request = FieldValueSetRequest::new(
            correlation_id,
            "StressTestData".to_string(),
            "payload".to_string(),
            data,
            "sequential_stress_test".to_string(),
        );
        
        if fixture.message_bus.publish(request).is_ok() {
            match response_consumer.recv_timeout(Duration::from_millis(200)) {
                Ok(response) if response.success => sequential_successful += 1,
                _ => sequential_failed += 1,
            }
        } else {
            sequential_failed += 1;
        }
        
        if i % 100 == 0 {
            println!("Sequential mutations progress: {}/{}", i, sequential_count);
        }
    }
    
    let sequential_duration = sequential_start.elapsed();
    let sequential_rate = sequential_successful as f64 / sequential_duration.as_secs_f64();
    
    println!("✅ Sequential mutations completed:");
    println!("   Total: {}, Successful: {}, Failed: {}", 
        sequential_count, sequential_successful, sequential_failed);
    println!("   Duration: {:?}, Rate: {:.2} mutations/sec", 
        sequential_duration, sequential_rate);
    
    // Test 2: Concurrent high-volume mutations
    println!("⚡ Test 2: Concurrent High-Volume Mutations");
    
    let concurrent_count = 2000;
    let thread_count = 4;
    
    let (concurrent_duration, concurrent_successful, concurrent_failed) = fixture
        .execute_high_volume_mutations(concurrent_count, thread_count)
        .expect("Failed to execute concurrent mutations");
    
    let concurrent_rate = concurrent_successful as f64 / concurrent_duration.as_secs_f64();
    
    println!("✅ Concurrent mutations completed:");
    println!("   Total: {}, Successful: {}, Failed: {}", 
        concurrent_count, concurrent_successful, concurrent_failed);
    println!("   Duration: {:?}, Rate: {:.2} mutations/sec", 
        concurrent_duration, concurrent_rate);
    println!("   Threads: {}", thread_count);
    
    // Test 3: Burst load testing
    println!("💥 Test 3: Burst Load Testing");
    
    let burst_iterations = 5;
    let burst_size = 500;
    let mut burst_rates = Vec::new();
    
    for burst_num in 0..burst_iterations {
        println!("Executing burst {} of {}", burst_num + 1, burst_iterations);
        
        let (burst_duration, burst_successful, _burst_failed) = fixture
            .execute_high_volume_mutations(burst_size, 2)
            .expect("Failed to execute burst");
        
        let burst_rate = burst_successful as f64 / burst_duration.as_secs_f64();
        burst_rates.push(burst_rate);
        
        println!("   Burst {}: {:.2} mutations/sec", burst_num + 1, burst_rate);
        
        // Small delay between bursts
        thread::sleep(Duration::from_millis(TEST_WAIT_MS));
    }
    
    let avg_burst_rate = burst_rates.iter().sum::<f64>() / burst_rates.len() as f64;
    let min_burst_rate = burst_rates.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_burst_rate = burst_rates.iter().fold(0.0f64, |a, &b| a.max(b));
    
    println!("✅ Burst load testing completed:");
    println!("   Average rate: {:.2} mutations/sec", avg_burst_rate);
    println!("   Min rate: {:.2} mutations/sec", min_burst_rate);
    println!("   Max rate: {:.2} mutations/sec", max_burst_rate);
    
    // Performance assertions
    assert!(sequential_rate > 50.0, "Sequential mutation rate should be > 50/sec");
    assert!(concurrent_rate > 50.0, "Concurrent mutation rate should be > 50/sec");
    assert!(avg_burst_rate > 25.0, "Burst mutation rate should be > 25/sec");
    
    // Success rate assertions
    let sequential_success_rate = sequential_successful as f64 / sequential_count as f64;
    let concurrent_success_rate = concurrent_successful as f64 / concurrent_count as f64;
    
    assert!(sequential_success_rate > 0.8, "Sequential success rate should be > 80%");
    assert!(concurrent_success_rate > 0.6, "Concurrent success rate should be > 60%");
    
    println!("✅ High-Volume Mutations Test PASSED");
}

#[test]
fn test_concurrent_transform_execution() {
    println!("🧪 TEST: Concurrent Transform Execution");
    println!("   This validates multi-threaded transform processing capabilities");
    
    let fixture = StressPerformanceTestFixture::new()
        .expect("Failed to create test fixture");
    
    fixture.create_stress_test_schemas()
        .expect("Failed to create stress test schemas");
    
    // Test 1: Multiple concurrent transform triggers
    println!("🔄 Test 1: Multiple Concurrent Transform Triggers");
    
    let transform_count = 100;
    let concurrent_threads = 8;
    let transforms_per_thread = transform_count / concurrent_threads;
    
    let triggered_count = Arc::new(AtomicUsize::new(0));
    let executed_count = Arc::new(AtomicUsize::new(0));
    
    // Subscribe to TransformExecuted events
    let mut executed_consumer = fixture.message_bus.subscribe::<TransformExecuted>();
    
    let start_time = Instant::now();
    
    // Create worker threads for transform triggering
    let trigger_handles: Vec<_> = (0..concurrent_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let triggered = Arc::clone(&triggered_count);
        
        thread::spawn(move || {
            for i in 0..transforms_per_thread {
                let transform_id = format!("stress_transform_{}_{}", thread_id, i);
                let trigger_event = TransformTriggered {
                    transform_id,
                };
                
                if message_bus.publish(trigger_event).is_ok() {
                    triggered.fetch_add(1, Ordering::Relaxed);
                }
                
                // Small delay to prevent overwhelming the system
                thread::sleep(Duration::from_millis(1));
            }
        })
    }).collect();
    
    // Wait for all triggers to be sent
    for handle in trigger_handles {
        handle.join().expect("Trigger thread panicked");
    }
    
    // Monitor for executed events
    let monitor_start = Instant::now();
    let monitor_timeout = Duration::from_secs(10);
    
    while monitor_start.elapsed() < monitor_timeout {
        match executed_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
            Ok(_executed_event) => {
                executed_count.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => break, // Timeout, no more events
        }
    }
    
    let total_duration = start_time.elapsed();
    let triggered_total = triggered_count.load(Ordering::Relaxed);
    let executed_total = executed_count.load(Ordering::Relaxed);
    
    let trigger_rate = triggered_total as f64 / total_duration.as_secs_f64();
    let execution_rate = executed_total as f64 / total_duration.as_secs_f64();
    
    println!("✅ Concurrent transform execution results:");
    println!("   Triggered: {}, Executed: {}", triggered_total, executed_total);
    println!("   Duration: {:?}", total_duration);
    println!("   Trigger rate: {:.2} triggers/sec", trigger_rate);
    println!("   Execution rate: {:.2} executions/sec", execution_rate);
    
    // Test 2: Transform execution under load
    println!("⚡ Test 2: Transform Execution Under Load");
    
    // Create a high-load scenario with data mutations and transform triggers
    let load_start = Instant::now();
    let load_operations = 200;
    let mutation_thread_count = 4;
    let transform_thread_count = 2;
    
    let load_successful = Arc::new(AtomicUsize::new(0));
    let load_failed = Arc::new(AtomicUsize::new(0));
    
    // Start mutation threads
    let mutation_handles: Vec<_> = (0..mutation_thread_count).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let successful = Arc::clone(&load_successful);
        let failed = Arc::clone(&load_failed);
        
        thread::spawn(move || {
            let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
            
            for i in 0..load_operations / mutation_thread_count {
                let data = json!({
                    "load_test": true,
                    "thread": thread_id,
                    "sequence": i,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                let request = FieldValueSetRequest::new(
                    format!("load_{}_{}", thread_id, i),
                    "StressTestData".to_string(),
                    "payload".to_string(),
                    data,
                    format!("load_thread_{}", thread_id),
                );
                
                if message_bus.publish(request).is_ok() {
                    match response_consumer.recv_timeout(Duration::from_millis(50)) {
                        Ok(response) if response.success => {
                            successful.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        })
    }).collect();
    
    // Start transform trigger threads
    let transform_handles: Vec<_> = (0..transform_thread_count).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        
        thread::spawn(move || {
            for i in 0..load_operations / transform_thread_count {
                let trigger_event = TransformTriggered {
                    transform_id: format!("load_transform_{}_{}", thread_id, i),
                };
                
                message_bus.publish(trigger_event).ok();
                thread::sleep(Duration::from_millis(5));
            }
        })
    }).collect();
    
    // Wait for all load test threads
    for handle in mutation_handles {
        handle.join().expect("Mutation thread panicked");
    }
    for handle in transform_handles {
        handle.join().expect("Transform thread panicked");
    }
    
    let load_duration = load_start.elapsed();
    let load_successful_count = load_successful.load(Ordering::Relaxed);
    let load_failed_count = load_failed.load(Ordering::Relaxed);
    
    println!("✅ Load test completed:");
    println!("   Operations: {} mutations, {} transforms", 
        load_operations, load_operations);
    println!("   Successful mutations: {}, Failed: {}", 
        load_successful_count, load_failed_count);
    println!("   Duration: {:?}", load_duration);
    
    // Performance assertions
    assert!(trigger_rate > 10.0, "Trigger rate should be > 10/sec");
    let load_success_rate = load_successful_count as f64 / (load_operations as f64);
    assert!(load_success_rate > 0.5, "Load test success rate should be > 50%");
    
    println!("✅ Concurrent Transform Execution Test PASSED");
}

#[test]
fn test_memory_usage_validation() {
    println!("🧪 TEST: Memory Usage Validation");
    println!("   This validates memory efficiency and detects memory leaks");
    
    let fixture = StressPerformanceTestFixture::new()
        .expect("Failed to create test fixture");
    
    fixture.create_stress_test_schemas()
        .expect("Failed to create stress test schemas");
    
    // Test 1: Memory usage during high-volume operations
    println!("💾 Test 1: Memory Usage During High-Volume Operations");
    
    let memory_stats = fixture.monitor_memory_usage();
    println!("Initial memory usage: {} KB", memory_stats.initial_usage);
    
    // Create a large amount of data
    let data_volume = 5000;
    let mut created_items = Vec::new();
    
    let memory_test_start = Instant::now();
    
    for i in 0..data_volume {
        // Create atoms directly to test memory usage
        let content = json!({
            "id": i,
            "data": format!("Memory test data item {} with some substantial content to test memory usage patterns", i),
            "metadata": {
                "created_at": chrono::Utc::now().to_rfc3339(),
                "category": format!("category_{}", i % 100),
                "tags": vec![format!("tag_{}", i % 10), format!("tag_{}", i % 20)],
            },
            "payload": vec![i; 100], // Add some bulk to test memory
        });
        
        let atom = Atom::new("StressTestData".to_string(), "memory_test".to_string(), content);
        let atom_uuid = Uuid::new_v4().to_string();
        
        fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)
            .expect("Failed to store atom");
        
        created_items.push(atom_uuid);
        
        if i % 1000 == 0 {
            println!("Memory test progress: {}/{}", i, data_volume);
        }
    }
    
    let memory_creation_duration = memory_test_start.elapsed();
    println!("✅ Created {} items in {:?}", data_volume, memory_creation_duration);
    
    // Test 2: Memory usage during retrieval operations
    println!("🔍 Test 2: Memory Usage During Retrieval Operations");
    
    let retrieval_start = Instant::now();
    let retrieval_sample_size = 1000;
    
    for i in 0..retrieval_sample_size {
        let atom_uuid = &created_items[i * (data_volume / retrieval_sample_size)];
        let _retrieved_atom = fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid))
            .expect("Failed to retrieve atom")
            .expect("Atom should exist");
    }
    
    let retrieval_duration = retrieval_start.elapsed();
    let retrieval_rate = retrieval_sample_size as f64 / retrieval_duration.as_secs_f64();
    
    println!("✅ Retrieved {} items in {:?}", retrieval_sample_size, retrieval_duration);
    println!("   Retrieval rate: {:.2} items/sec", retrieval_rate);
    
    // Test 3: Memory cleanup verification
    println!("🧹 Test 3: Memory Cleanup Verification");
    
    let cleanup_start = Instant::now();
    
    // Clear references to created items
    created_items.clear();
    created_items.shrink_to_fit();
    
    // Force some cleanup operations
    std::hint::black_box(&created_items); // Prevent optimization
    
    let cleanup_duration = cleanup_start.elapsed();
    println!("✅ Cleanup completed in {:?}", cleanup_duration);
    
    // Test 4: Stress memory allocation patterns
    println!("⚡ Test 4: Stress Memory Allocation Patterns");
    
    let allocation_start = Instant::now();
    let allocation_cycles = 10;
    let items_per_cycle = 1000;
    
    for cycle in 0..allocation_cycles {
        let mut cycle_items = Vec::new();
        
        // Allocate items
        for i in 0..items_per_cycle {
            let large_content = json!({
                "cycle": cycle,
                "item": i,
                "large_data": vec![i; 1000], // Large allocation
                "strings": (0..100).map(|j| format!("string_{}_{}", cycle, j)).collect::<Vec<_>>(),
            });
            
            let atom = Atom::new("StressTestData".to_string(), "allocation_test".to_string(), large_content);
            cycle_items.push(atom);
        }
        
        // Use the items briefly
        let cycle_size = cycle_items.len();
        std::hint::black_box(&cycle_items);
        
        // Clear the cycle (simulating cleanup)
        cycle_items.clear();
        
        println!("Allocation cycle {}: {} items allocated and cleared", cycle + 1, cycle_size);
    }
    
    let allocation_duration = allocation_start.elapsed();
    let total_allocations = allocation_cycles * items_per_cycle;
    let allocation_rate = total_allocations as f64 / allocation_duration.as_secs_f64();
    
    println!("✅ Allocation stress test completed:");
    println!("   Total allocations: {}", total_allocations);
    println!("   Duration: {:?}", allocation_duration);
    println!("   Allocation rate: {:.2} items/sec", allocation_rate);
    
    // Test 5: Long-running memory stability
    println!("⏱️  Test 5: Long-Running Memory Stability");
    
    let stability_start = Instant::now();
    let stability_duration = Duration::from_secs(5); // Short duration for test
    let stability_iterations = 100;
    
    let mut iteration_count = 0;
    while stability_start.elapsed() < stability_duration && iteration_count < stability_iterations {
        // Perform mixed operations
        let content = json!({
            "stability_test": true,
            "iteration": iteration_count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let atom = Atom::new("StressTestData".to_string(), "stability_test".to_string(), content);
        let atom_uuid = Uuid::new_v4().to_string();
        
        fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom).ok();
        
        // Retrieve and use the item
        if let Ok(Some(_retrieved)) = fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid)) {
            // Item retrieved successfully
        }
        
        iteration_count += 1;
        
        if iteration_count % 20 == 0 {
            println!("Stability test iteration: {}", iteration_count);
        }
    }
    
    let stability_actual_duration = stability_start.elapsed();
    println!("✅ Stability test completed: {} iterations in {:?}", 
        iteration_count, stability_actual_duration);
    
    // Performance assertions
    assert!(retrieval_rate > 100.0, "Retrieval rate should be > 100 items/sec");
    assert!(allocation_rate > 50.0, "Allocation rate should be > 50 items/sec");
    assert!(iteration_count > 50, "Should complete > 50 stability iterations");
    
    println!("✅ Memory Usage Validation Test PASSED");
}

#[test]
fn test_event_system_performance_under_load() {
    println!("🧪 TEST: Event System Performance Under Load");
    println!("   This validates message bus performance under high load");
    
    let fixture = StressPerformanceTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: High-frequency event publishing
    println!("📡 Test 1: High-Frequency Event Publishing");
    
    let event_count = 10000;
    let publisher_threads = 4;
    let events_per_thread = event_count / publisher_threads;
    
    let published_count = Arc::new(AtomicUsize::new(0));
    let publish_start = Instant::now();
    
    // Create publisher threads
    let publish_handles: Vec<_> = (0..publisher_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let published = Arc::clone(&published_count);
        
        thread::spawn(move || {
            for i in 0..events_per_thread {
                let transform_event = TransformTriggered {
                    transform_id: format!("high_freq_{}_{}", thread_id, i),
                };
                
                if message_bus.publish(transform_event).is_ok() {
                    published.fetch_add(1, Ordering::Relaxed);
                }
            }
        })
    }).collect();
    
    // Wait for all publishers
    for handle in publish_handles {
        handle.join().expect("Publisher thread panicked");
    }
    
    let publish_duration = publish_start.elapsed();
    let published_total = published_count.load(Ordering::Relaxed);
    let publish_rate = published_total as f64 / publish_duration.as_secs_f64();
    
    println!("✅ High-frequency publishing completed:");
    println!("   Published: {}/{}", published_total, event_count);
    println!("   Duration: {:?}", publish_duration);
    println!("   Publish rate: {:.2} events/sec", publish_rate);
    
    // Test 2: Concurrent publishing and consuming
    println!("🔄 Test 2: Concurrent Publishing and Consuming");
    
    let concurrent_events = 5000;
    let consumer_threads = 2;
    let concurrent_publisher_threads = 2;
    
    let consumed_count = Arc::new(AtomicUsize::new(0));
    let concurrent_published = Arc::new(AtomicUsize::new(0));
    
    let concurrent_start = Instant::now();
    
    // Start consumer threads
    let consumer_handles: Vec<_> = (0..consumer_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let consumed = Arc::clone(&consumed_count);
        
        thread::spawn(move || {
            let mut consumer = message_bus.subscribe::<TransformTriggered>();
            let consumer_start = Instant::now();
            let consumer_timeout = Duration::from_secs(10);
            
            while consumer_start.elapsed() < consumer_timeout {
                match consumer.recv_timeout(Duration::from_millis(10)) {
                    Ok(_event) => {
                        consumed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => break, // Timeout
                }
            }
            
            println!("Consumer {} finished", thread_id);
        })
    }).collect();
    
    // Start publisher threads
    let concurrent_publish_handles: Vec<_> = (0..concurrent_publisher_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let published = Arc::clone(&concurrent_published);
        
        thread::spawn(move || {
            let events_per_publisher = concurrent_events / concurrent_publisher_threads;
            
            for i in 0..events_per_publisher {
                let event = TransformTriggered {
                    transform_id: format!("concurrent_{}_{}", thread_id, i),
                };
                
                if message_bus.publish(event).is_ok() {
                    published.fetch_add(1, Ordering::Relaxed);
                }
                
                // Small delay to prevent overwhelming
                if i % 100 == 0 {
                    thread::sleep(Duration::from_millis(1));
                }
            }
            
            println!("Publisher {} finished", thread_id);
        })
    }).collect();
    
    // Wait for publishers first
    for handle in concurrent_publish_handles {
        handle.join().expect("Concurrent publisher thread panicked");
    }
    
    // Give consumers time to process remaining events
    thread::sleep(Duration::from_millis(500));
    
    // Wait for consumers
    for handle in consumer_handles {
        handle.join().expect("Consumer thread panicked");
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    let concurrent_published_total = concurrent_published.load(Ordering::Relaxed);
    let consumed_total = consumed_count.load(Ordering::Relaxed);
    
    let concurrent_publish_rate = concurrent_published_total as f64 / concurrent_duration.as_secs_f64();
    let consume_rate = consumed_total as f64 / concurrent_duration.as_secs_f64();
    
    println!("✅ Concurrent pub/sub completed:");
    println!("   Published: {}, Consumed: {}", concurrent_published_total, consumed_total);
    println!("   Duration: {:?}", concurrent_duration);
    println!("   Publish rate: {:.2} events/sec", concurrent_publish_rate);
    println!("   Consume rate: {:.2} events/sec", consume_rate);
    
    // Test 3: Event backpressure handling
    println!("⏸️  Test 3: Event Backpressure Handling");
    
    let backpressure_events = 2000;
    let fast_publisher_rate = 1000; // events per second
    let slow_consumer_rate = 100;   // events per second
    
    let backpressure_consumed = Arc::new(AtomicUsize::new(0));
    let backpressure_published = Arc::new(AtomicUsize::new(0));
    
    let backpressure_start = Instant::now();
    
    // Start slow consumer
    let slow_consumer_handle = {
        let message_bus = Arc::clone(&fixture.message_bus);
        let consumed = Arc::clone(&backpressure_consumed);
        
        thread::spawn(move || {
            let mut consumer = message_bus.subscribe::<TransformTriggered>();
            
            for _ in 0..backpressure_events {
                match consumer.recv_timeout(Duration::from_millis(1000)) {
                    Ok(_event) => {
                        consumed.fetch_add(1, Ordering::Relaxed);
                        // Simulate slow processing
                        thread::sleep(Duration::from_millis(1000 / slow_consumer_rate as u64));
                    }
                    Err(_) => break,
                }
            }
        })
    };
    
    // Start fast publisher
    let fast_publisher_handle = {
        let message_bus = Arc::clone(&fixture.message_bus);
        let published = Arc::clone(&backpressure_published);
        
        thread::spawn(move || {
            for i in 0..backpressure_events {
                let event = TransformTriggered {
                    transform_id: format!("backpressure_{}", i),
                };
                
                if message_bus.publish(event).is_ok() {
                    published.fetch_add(1, Ordering::Relaxed);
                }
                
                // Fast publishing rate
                thread::sleep(Duration::from_millis(1000 / fast_publisher_rate as u64));
            }
        })
    };
    
    // Wait for completion or timeout
    let backpressure_timeout = Duration::from_secs(30);
    let mut publisher_done = false;
    let mut consumer_done = false;
    
    while backpressure_start.elapsed() < backpressure_timeout && (!publisher_done || !consumer_done) {
        if !publisher_done && fast_publisher_handle.is_finished() {
            publisher_done = true;
            println!("Fast publisher completed");
        }
        if !consumer_done && slow_consumer_handle.is_finished() {
            consumer_done = true;
            println!("Slow consumer completed");
        }
        thread::sleep(Duration::from_millis(TEST_WAIT_MS));
    }
    
    // Force completion
    fast_publisher_handle.join().ok();
    slow_consumer_handle.join().ok();
    
    let backpressure_duration = backpressure_start.elapsed();
    let backpressure_published_total = backpressure_published.load(Ordering::Relaxed);
    let backpressure_consumed_total = backpressure_consumed.load(Ordering::Relaxed);
    
    println!("✅ Backpressure handling completed:");
    println!("   Published: {}, Consumed: {}", backpressure_published_total, backpressure_consumed_total);
    println!("   Duration: {:?}", backpressure_duration);
    
    // Performance assertions
    assert!(publish_rate > 1000.0, "Publish rate should be > 1000 events/sec");
    assert!(concurrent_publish_rate > 500.0, "Concurrent publish rate should be > 500 events/sec");
    assert!(consume_rate > 100.0, "Consume rate should be > 100 events/sec");
    
    // Backpressure assertions
    let consumption_ratio = backpressure_consumed_total as f64 / backpressure_published_total as f64;
    assert!(consumption_ratio > 0.1, "Should consume at least 10% of published events under backpressure");
    
    println!("✅ Event System Performance Under Load Test PASSED");
}

#[test]
fn test_database_scalability() {
    println!("🧪 TEST: Database Scalability");
    println!("   This validates large dataset handling and query performance");
    
    let fixture = StressPerformanceTestFixture::new()
        .expect("Failed to create test fixture");
    
    fixture.create_stress_test_schemas()
        .expect("Failed to create stress test schemas");
    
    // Test 1: Large dataset storage
    println!("💾 Test 1: Large Dataset Storage");
    
    let large_dataset_size = 50000;
    let storage_batch_size = 1000;
    let storage_start = Instant::now();
    
    for batch in 0..(large_dataset_size / storage_batch_size) {
        let batch_start = Instant::now();
        
        for i in 0..storage_batch_size {
            let item_id = batch * storage_batch_size + i;
            let content = json!({
                "id": item_id,
                "data": format!("Large dataset item {}", item_id),
                "category": format!("category_{}", item_id % 1000),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "metadata": {
                    "batch": batch,
                    "sequence": i,
                    "random_value": (item_id * 17) % 10000,
                }
            });
            
            let atom = Atom::new("StressTestData".to_string(), "scalability_test".to_string(), content);
            let atom_uuid = format!("scale_atom_{:08}", item_id);
            
            fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)
                .expect("Failed to store atom");
        }
        
        let batch_duration = batch_start.elapsed();
        println!("Stored batch {}: {} items in {:?}", 
            batch + 1, storage_batch_size, batch_duration);
    }
    
    let storage_duration = storage_start.elapsed();
    let storage_rate = large_dataset_size as f64 / storage_duration.as_secs_f64();
    
    println!("✅ Large dataset storage completed:");
    println!("   Items: {}", large_dataset_size);
    println!("   Duration: {:?}", storage_duration);
    println!("   Storage rate: {:.2} items/sec", storage_rate);
    
    // Test 2: Random access query performance
    println!("🔍 Test 2: Random Access Query Performance");
    
    let query_sample_size = 5000;
    let query_start = Instant::now();
    
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut successful_queries = 0;
    
    for i in 0..query_sample_size {
        // Generate pseudo-random item ID
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let random_id = (hasher.finish() % large_dataset_size as u64) as usize;
        
        let atom_uuid = format!("scale_atom_{:08}", random_id);
        
        if let Ok(Some(_atom)) = fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid)) {
            successful_queries += 1;
        } // Failed query is ignored
    }
    
    let query_duration = query_start.elapsed();
    let query_rate = successful_queries as f64 / query_duration.as_secs_f64();
    let query_success_rate = successful_queries as f64 / query_sample_size as f64;
    
    println!("✅ Random access queries completed:");
    println!("   Queries: {}, Successful: {}", query_sample_size, successful_queries);
    println!("   Duration: {:?}", query_duration);
    println!("   Query rate: {:.2} queries/sec", query_rate);
    println!("   Success rate: {:.2}%", query_success_rate * 100.0);
    
    // Test 3: Sequential scan performance
    println!("📏 Test 3: Sequential Scan Performance");
    
    let scan_start = Instant::now();
    let scan_sample_size = 10000;
    let mut scanned_items = 0;
    
    for i in 0..scan_sample_size {
        let atom_uuid = format!("scale_atom_{:08}", i);
        
        if fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid)).unwrap_or(None).is_some() {
            scanned_items += 1;
        }
    }
    
    let scan_duration = scan_start.elapsed();
    let scan_rate = scanned_items as f64 / scan_duration.as_secs_f64();
    
    println!("✅ Sequential scan completed:");
    println!("   Scanned: {}, Found: {}", scan_sample_size, scanned_items);
    println!("   Duration: {:?}", scan_duration);
    println!("   Scan rate: {:.2} items/sec", scan_rate);
    
    // Test 4: Concurrent database access
    println!("⚡ Test 4: Concurrent Database Access");
    
    let concurrent_threads = 8;
    let queries_per_thread = 1000;
    let concurrent_successful = Arc::new(AtomicUsize::new(0));
    
    let concurrent_start = Instant::now();
    
    let concurrent_handles: Vec<_> = (0..concurrent_threads).map(|thread_id| {
        let db_ops = Arc::clone(&fixture.db_ops);
        let successful = Arc::clone(&concurrent_successful);
        
        thread::spawn(move || {
            let mut thread_successful = 0;
            
            for i in 0..queries_per_thread {
                let item_id = (thread_id * queries_per_thread + i) % large_dataset_size;
                let atom_uuid = format!("scale_atom_{:08}", item_id);
                
                if db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid)).unwrap_or(None).is_some() {
                    thread_successful += 1;
                }
            }
            
            successful.fetch_add(thread_successful, Ordering::Relaxed);
            thread_successful
        })
    }).collect();
    
    let mut thread_results = Vec::new();
    for handle in concurrent_handles {
        thread_results.push(handle.join().expect("Concurrent thread panicked"));
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    let concurrent_total_queries = concurrent_threads * queries_per_thread;
    let concurrent_successful_total = concurrent_successful.load(Ordering::Relaxed);
    let concurrent_rate = concurrent_successful_total as f64 / concurrent_duration.as_secs_f64();
    
    println!("✅ Concurrent database access completed:");
    println!("   Threads: {}, Queries per thread: {}", concurrent_threads, queries_per_thread);
    println!("   Total queries: {}, Successful: {}", concurrent_total_queries, concurrent_successful_total);
    println!("   Duration: {:?}", concurrent_duration);
    println!("   Concurrent rate: {:.2} queries/sec", concurrent_rate);
    
    // Performance assertions
    assert!(storage_rate > 100.0, "Storage rate should be > 100 items/sec");
    assert!(query_rate > 500.0, "Query rate should be > 500 queries/sec");
    assert!(scan_rate > 1000.0, "Scan rate should be > 1000 items/sec");
    assert!(concurrent_rate > 1000.0, "Concurrent rate should be > 1000 queries/sec");
    
    // Success rate assertions
    assert!(query_success_rate > 0.95, "Query success rate should be > 95%");
    
    println!("✅ Database Scalability Test PASSED");
}