//! Integration tests for TransformManager decomposition and system functionality
//!
//! These tests validate that all decomposed modules work together correctly
//! as a cohesive system, ensuring no functionality was lost during decomposition.
//! Also includes tests for system-level functionality like database reset.

pub mod transform_result_persistence_tests;
pub mod system_routes_tests;
pub mod complete_mutation_query_flow_test;

// Comprehensive test suites for collection removal and bug fixes
pub mod collection_removal_validation_test;
pub mod range_architecture_test;
pub mod stress_performance_test;
pub mod regression_prevention_test;

// Available schemas test
pub mod available_schemas_test;

// Validation and error handling integration tests
pub mod storage_integration_tests;
pub mod error_handling_tests;

// Transform integration tests
pub mod transform_integration_tests;
pub mod declarative_transform_integration_tests;

// BlogWordIndex integration test
pub mod blog_word_index_integration_test;

// HashRange mutation integration test
pub mod hashrange_mutation_integration_test;

// HashRange end-to-end workflow test
pub mod hashrange_end_to_end_workflow_test;

// Simplified format end-to-end tests
pub mod simplified_format_e2e_tests;
