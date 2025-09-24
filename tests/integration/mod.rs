//! Integration tests for TransformManager decomposition and system functionality
//!
//! These tests validate that all decomposed modules work together correctly
//! as a cohesive system, ensuring no functionality was lost during decomposition.
//! Also includes tests for system-level functionality like database reset.

pub mod system_routes_tests;
pub mod transform_result_persistence_tests;

// Comprehensive test suites for collection removal and bug fixes
pub mod collection_removal_validation_test;
pub mod range_architecture_test;
pub mod regression_prevention_test;
pub mod stress_performance_test;

// Available schemas test
pub mod available_schemas_test;

// BlogWordIndex integration test
pub mod blog_word_index_integration_test;

// HashRange mutation integration test
pub mod hashrange_mutation_integration_test;