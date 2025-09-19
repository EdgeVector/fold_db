//! Unit tests for individual TransformManager modules
//!
//! These tests validate that each decomposed module functions correctly
//! in isolation and maintains its specific responsibilities.
//!
//! ## Universal Key Configuration Testing
//!
//! The universal key configuration system allows schemas to define their key fields
//! in a unified way across all schema types (Single, Range, HashRange). This provides
//! consistent key management and eliminates the need for hardcoded field names.
//!
//! ### Test Patterns
//!
//! When testing universal key functionality:
//! 1. **Schema Creation**: Use test fixtures to create schemas with universal key configuration
//! 2. **Field Processing**: Test that field processing utilities work with universal key extraction
//! 3. **Error Handling**: Test error scenarios for invalid universal key configurations
//! 4. **Validation**: Test that schemas validate correctly with universal key configuration
//!
//! ### Example Usage
//!
//! ```rust
//! // Create a HashRange schema with universal key configuration
//! let schema = fixture.create_hashrange_schema_with_universal_key(
//!     "TestSchema", 
//!     "user_id", 
//!     "timestamp"
//! );
//! 
//! // Test field name extraction
//! let (hash_field, range_field) = mutation_service
//!     .get_hashrange_key_field_names(&schema)?;
//! assert_eq!(hash_field, "user_id");
//! assert_eq!(range_field, "timestamp");
//! ```

pub mod mutation_completion_tests;
pub mod range_filter_tests;
pub mod schema;
pub mod schema_parsing_test;
pub mod transform;
pub mod transform_manager_module_tests;
pub mod transform_utils_helper_tests;
pub mod declarative_transform_tests;
pub mod hashrange_schema_tests;
pub mod schema_declarative_transform_interpretation_tests;
pub mod iterator_stack_tests;
pub mod field_alignment_tests;
pub mod chain_parser_tests;
pub mod hashrange_mutation_core_test;
pub mod schema_universal_key_validation_tests;
pub mod schema_universal_key_parsing_tests;
pub mod unified_key_extraction_tests;
pub mod hashrange_query_processor_tests;
pub mod mutation_processor_universal_key_tests;
pub mod mutation_service_universal_key_tests;
pub mod universal_key_transform_tests;
