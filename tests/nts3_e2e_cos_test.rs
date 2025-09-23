/*!
 * NTS-3-8: E2E CoS Test - End-to-End Chain of Success Validation
 *
 * This module provides comprehensive end-to-end validation tests for the complete
 * Native Transform System (NTS-3). The tests validate the entire pipeline from
 * input to output, ensuring all components integrate correctly and handle complex
 * real-world scenarios with proper type safety and error handling.
 *
 * Test Coverage:
 * - Complete transform pipeline validation
 * - Integration between all NTS-3 components (Executor, Function Registry, Expression Evaluator, Schema Registry)
 * - Complex real-world scenarios with realistic data
 * - Type safety and error handling throughout the pipeline
 * - Performance characteristics with large datasets (10K+ records)
 * - Backward compatibility and migration scenarios
 * - Comprehensive assertions and validation
 */

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    use datafold::transform::function_registry::FunctionRegistry;
    use datafold::transform::native::transform_spec::{
        FieldMapping, FilterCondition, FilterTransform, MapTransform, ReduceTransform,
        ReducerType, TransformSpec, TransformType,
    };
    use datafold::transform::native::types::FieldValue;
    use datafold::transform::native_executor::{NativeTransformExecutor, NativeTransformInput};
    use datafold::transform::native_schema_registry::{NativeSchemaRegistry, DatabaseOperationsTrait};
    use datafold::schema::types::errors::SchemaError;

    /// Test utilities and fixtures for NTS-3 E2E testing
    mod test_utils {
        use super::*;

        /// Create a test NativeTransformExecutor with all built-in functions
        pub fn create_test_executor() -> NativeTransformExecutor {
            let schema_registry = Arc::new(NativeSchemaRegistry::new(Arc::new(MockDatabaseOperations)));
            let function_registry = Arc::new(FunctionRegistry::with_built_ins());
            NativeTransformExecutor::new_with_functions(schema_registry, function_registry)
        }

        /// Create a test field definition
        pub fn create_test_field_definition(name: &str, field_type: datafold::transform::native::types::FieldType) -> datafold::transform::native::field_definition::FieldDefinition {
            datafold::transform::native::field_definition::FieldDefinition::new(name, field_type)
        }

        /// Create a test transform spec with minimal field definitions
        pub fn create_test_transform_spec(
            name: &str,
            transform_type: TransformType,
        ) -> TransformSpec {
            let inputs = vec![
                create_test_field_definition("id", datafold::transform::native::types::FieldType::Integer),
                create_test_field_definition("name", datafold::transform::native::types::FieldType::String),
                create_test_field_definition("age", datafold::transform::native::types::FieldType::Integer),
                create_test_field_definition("active", datafold::transform::native::types::FieldType::Boolean),
                create_test_field_definition("score", datafold::transform::native::types::FieldType::Number),
            ];
            let output = create_test_field_definition("output", datafold::transform::native::types::FieldType::String);

            TransformSpec::new(name, inputs, output, transform_type)
        }

        /// Create a test transform spec with custom field definitions
        pub fn create_test_transform_spec_with_fields(
            name: &str,
            input_fields: Vec<(&str, datafold::transform::native::types::FieldType)>,
            output_field: (&str, datafold::transform::native::types::FieldType),
            transform_type: TransformType,
        ) -> TransformSpec {
            let inputs = input_fields.into_iter()
                .map(|(name, field_type)| create_test_field_definition(name, field_type))
                .collect();
            let output = create_test_field_definition(output_field.0, output_field.1);

            TransformSpec::new(name, inputs, output, transform_type)
        }

        /// Create test data for user scenarios
        pub fn create_test_user_data() -> HashMap<String, FieldValue> {
            let mut data = HashMap::new();
            data.insert("id".to_string(), FieldValue::Integer(123));
            data.insert("name".to_string(), FieldValue::String("John Doe".to_string()));
            data.insert("email".to_string(), FieldValue::String("john@example.com".to_string()));
            data.insert("age".to_string(), FieldValue::Integer(30));
            data.insert("active".to_string(), FieldValue::Boolean(true));
            data.insert("score".to_string(), FieldValue::Number(85.5));
            data
        }

        /// Create test data for e-commerce scenarios
        pub fn create_test_ecommerce_data() -> HashMap<String, FieldValue> {
            let mut data = HashMap::new();
            data.insert("order_id".to_string(), FieldValue::String("ORD-001".to_string()));
            data.insert("customer_id".to_string(), FieldValue::Integer(456));
            data.insert("items".to_string(), FieldValue::Array(vec![
                FieldValue::String("widget_a".to_string()),
                FieldValue::String("widget_b".to_string()),
                FieldValue::String("widget_c".to_string()),
            ]));
            data.insert("quantities".to_string(), FieldValue::Array(vec![
                FieldValue::Integer(2),
                FieldValue::Integer(1),
                FieldValue::Integer(3),
            ]));
            data.insert("prices".to_string(), FieldValue::Array(vec![
                FieldValue::Number(10.99),
                FieldValue::Number(25.50),
                FieldValue::Number(5.25),
            ]));
            data.insert("total".to_string(), FieldValue::Number(63.23));
            data
        }

        /// Create test data for analytics scenarios
        pub fn create_test_analytics_data() -> HashMap<String, FieldValue> {
            let mut data = HashMap::new();
            data.insert("user_id".to_string(), FieldValue::Integer(789));
            data.insert("session_id".to_string(), FieldValue::String("sess_abc123".to_string()));
            data.insert("page_views".to_string(), FieldValue::Array(vec![
                FieldValue::String("/home".to_string()),
                FieldValue::String("/products".to_string()),
                FieldValue::String("/checkout".to_string()),
                FieldValue::String("/thank-you".to_string()),
            ]));
            data.insert("timestamps".to_string(), FieldValue::Array(vec![
                FieldValue::String("2024-01-01T10:00:00Z".to_string()),
                FieldValue::String("2024-01-01T10:05:00Z".to_string()),
                FieldValue::String("2024-01-01T10:10:00Z".to_string()),
                FieldValue::String("2024-01-01T10:15:00Z".to_string()),
            ]));
            data.insert("event_types".to_string(), FieldValue::Array(vec![
                FieldValue::String("page_view".to_string()),
                FieldValue::String("scroll".to_string()),
                FieldValue::String("click".to_string()),
                FieldValue::String("purchase".to_string()),
            ]));
            data
        }

        /// Create large dataset for performance testing (10K+ records)
        pub fn create_large_dataset(record_count: usize) -> HashMap<String, FieldValue> {
            let mut users = Vec::new();

            for i in 0..record_count {
                let mut user = HashMap::new();
                user.insert("id".to_string(), FieldValue::Integer(i as i64));
                user.insert("name".to_string(), FieldValue::String(format!("User_{}", i)));
                user.insert("age".to_string(), FieldValue::Integer((20 + (i % 60)) as i64));
                user.insert("active".to_string(), FieldValue::Boolean(i % 2 == 0));
                user.insert("score".to_string(), FieldValue::Number(50.0 + (i % 50) as f64));
                user.insert("email".to_string(), FieldValue::String(format!("user_{}@example.com", i)));

                // Add nested address data for complex field access testing
                let mut address = HashMap::new();
                address.insert("street".to_string(), FieldValue::String(format!("{} Main St", i % 1000)));
                address.insert("city".to_string(), FieldValue::String(format!("City_{}", i % 10)));
                address.insert("zipcode".to_string(), FieldValue::String(format!("{:05}", i % 90000 + 10000)));
                user.insert("address".to_string(), FieldValue::Object(address));

                users.push(FieldValue::Object(user));
            }

            let mut data = HashMap::new();
            data.insert("users".to_string(), FieldValue::Array(users));
            data
        }

        /// Create a simple test schema for validation
        pub fn create_test_schema() -> &'static str {
            r#"{
                "name": "test_schema",
                "schema_type": "Single",
                "payment_config": {
                    "base_multiplier": 1.0,
                    "min_payment_threshold": 0
                },
                "fields": {
                    "id": {
                        "field_type": "Single",
                        "permission_policy": {
                            "read_policy": { "Distance": 0 },
                            "write_policy": { "Distance": 0 }
                        },
                        "payment_config": {
                            "base_multiplier": 1.0,
                            "trust_distance_scaling": "None",
                            "min_payment": null
                        },
                        "field_mappers": {}
                    },
                    "name": {
                        "field_type": "Single",
                        "permission_policy": {
                            "read_policy": { "Distance": 0 },
                            "write_policy": { "Distance": 0 }
                        },
                        "payment_config": {
                            "base_multiplier": 1.0,
                            "trust_distance_scaling": "None",
                            "min_payment": null
                        },
                        "field_mappers": {}
                    },
                    "age": {
                        "field_type": "Single",
                        "permission_policy": {
                            "read_policy": { "Distance": 0 },
                            "write_policy": { "Distance": 0 }
                        },
                        "payment_config": {
                            "base_multiplier": 1.0,
                            "trust_distance_scaling": "None",
                            "min_payment": null
                        },
                        "field_mappers": {}
                    }
                }
            }"#
        }

        /// Mock database operations for testing
        #[derive(Debug)]
        pub struct MockDatabaseOperations;

        #[async_trait::async_trait]
        impl DatabaseOperationsTrait for MockDatabaseOperations {
            async fn store_schema(&self, _name: &str, _schema: &str) -> Result<(), SchemaError> {
                Ok(())
            }

            async fn get_schema(&self, _name: &str) -> Result<Option<String>, SchemaError> {
                Ok(None)
            }

            async fn delete_schema(&self, _name: &str) -> Result<(), SchemaError> {
                Ok(())
            }

            async fn list_schemas(&self) -> Result<Vec<String>, SchemaError> {
                Ok(vec!["test_schema".to_string()])
            }
        }

        /// Performance benchmark helper
        pub struct PerformanceBenchmark {
            pub name: String,
            pub execution_time_ns: u64,
            pub memory_usage: usize,
            pub operations_count: usize,
        }

        impl PerformanceBenchmark {
            pub fn new(name: String) -> Self {
                Self {
                    name,
                    execution_time_ns: 0,
                    memory_usage: 0,
                    operations_count: 0,
                }
            }

            pub fn record_execution_time(&mut self, start: Instant, end: Instant) {
                self.execution_time_ns = end.duration_since(start).as_nanos() as u64;
            }

            pub fn record_operations(&mut self, count: usize) {
                self.operations_count = count;
            }
        }

        /// Type safety validation helper
        pub struct TypeSafetyValidator;

        impl TypeSafetyValidator {
            /// Validate that all field types match expected types
            pub fn validate_field_types(data: &HashMap<String, FieldValue>, expected_types: &HashMap<String, datafold::transform::native::types::FieldType>) -> Result<(), String> {
                for (field_name, expected_type) in expected_types {
                    if let Some(value) = data.get(field_name) {
                        if !expected_type.matches(value) {
                            return Err(format!("Field '{}' type mismatch: expected {:?}, got {:?}", field_name, expected_type, value));
                        }
                    } else {
                        return Err(format!("Missing required field: {}", field_name));
                    }
                }
                Ok(())
            }

            /// Validate nested object field access
            pub fn validate_nested_access(data: &HashMap<String, FieldValue>, access_path: &str, expected_type: datafold::transform::native::types::FieldType) -> Result<(), String> {
                let parts: Vec<&str> = access_path.split('.').collect();
                let mut current_value = data.get(parts[0])
                    .ok_or_else(|| format!("Field '{}' not found", parts[0]))?;

                for part in &parts[1..] {
                    match current_value {
                        FieldValue::Object(obj) => {
                            current_value = obj.get(*part)
                                .ok_or_else(|| format!("Field '{}' not found in object", part))?;
                        }
                        FieldValue::Array(arr) => {
                            let index: usize = part.parse()
                                .map_err(|_| format!("Invalid array index: {}", part))?;
                            current_value = arr.get(index)
                                .ok_or_else(|| format!("Array index '{}' out of bounds", index))?;
                        }
                        _ => return Err(format!("Cannot access field '{}' on non-object/array type", part)),
                    }
                }

                if !expected_type.matches(current_value) {
                    return Err(format!("Nested access '{}' type mismatch: expected {:?}, got {:?}", access_path, expected_type, current_value));
                }

                Ok(())
            }
        }
    }

    // NTS-3-8-1: Complete Transform Pipeline Validation
    mod complete_pipeline_validation {
        use super::*;
        use test_utils::{create_test_transform_spec, TypeSafetyValidator};

        #[tokio::test]
        async fn test_complete_ecommerce_pipeline() {
            let executor = test_utils::create_test_executor();
            let order_data = test_utils::create_test_ecommerce_data();

            let input = NativeTransformInput {
                values: order_data,
                schema_name: None,
            };

            // Create a comprehensive transform pipeline for order processing
            let transforms = vec![
                // Step 1: Validate order data quality
                test_utils::create_test_transform_spec_with_fields(
                    "validate_order",
                    vec![
                        ("order_id", datafold::transform::native::types::FieldType::String),
                        ("customer_id", datafold::transform::native::types::FieldType::Integer),
                        ("items", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("quantities", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Integer) }),
                        ("prices", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Number) }),
                        ("total", datafold::transform::native::types::FieldType::Number),
                    ],
                    ("valid", datafold::transform::native::types::FieldType::Boolean),
                    TransformType::Filter(FilterTransform {
                        condition: FilterCondition::And {
                            conditions: vec![
                                FilterCondition::GreaterThan {
                                    field: "total".to_string(),
                                    value: FieldValue::Number(0.0),
                                },
                                FilterCondition::GreaterThan {
                                    field: "quantities".to_string(),
                                    value: FieldValue::Integer(0),
                                },
                            ],
                        },
                    }),
                ),
                // Step 2: Transform and enrich order data
                test_utils::create_test_transform_spec_with_fields(
                    "process_order",
                    vec![
                        ("order_id", datafold::transform::native::types::FieldType::String),
                        ("customer_id", datafold::transform::native::types::FieldType::Integer),
                        ("items", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("quantities", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Integer) }),
                        ("prices", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Number) }),
                        ("total", datafold::transform::native::types::FieldType::Number),
                    ],
                    ("result", datafold::transform::native::types::FieldType::String),
                    TransformType::Map(MapTransform::new({
                        let mut mappings = HashMap::new();
                        mappings.insert("customer_key".to_string(), FieldMapping::Direct {
                            field: "customer_id".to_string(),
                        });
                        mappings.insert("total_items".to_string(), FieldMapping::Function {
                            name: "length".to_string(),
                            arguments: vec!["items".to_string()],
                        });
                        mappings.insert("order_summary".to_string(), FieldMapping::Expression {
                            expression: "order_id + \" - \" + to_string(total_items) + \" items, $\" + to_string(total)".to_string(),
                        });
                        mappings.insert("is_large_order".to_string(), FieldMapping::Expression {
                            expression: "total > 50.0".to_string(),
                        });
                        mappings.insert("average_price".to_string(), FieldMapping::Expression {
                            expression: "total / total_items".to_string(),
                        });
                        mappings
                    })),
                ),
                // Step 3: Aggregate order statistics
                test_utils::create_test_transform_spec_with_fields(
                    "order_stats",
                    vec![
                        ("order_id", datafold::transform::native::types::FieldType::String),
                        ("customer_id", datafold::transform::native::types::FieldType::Integer),
                        ("items", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("quantities", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Integer) }),
                        ("prices", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::Number) }),
                        ("total", datafold::transform::native::types::FieldType::Number),
                    ],
                    ("sum_total", datafold::transform::native::types::FieldType::Number),
                    TransformType::Reduce(ReduceTransform::new(
                        ReducerType::Sum { field: "total".to_string() },
                        vec![], // group_by fields
                    )),
                ),
            ];

            let chain_spec = create_test_transform_spec("ecommerce_pipeline", TransformType::Chain(transforms));

            let start_time = Instant::now();
            let result = executor.execute_transform(&chain_spec, input).await.unwrap();
            let execution_time = start_time.elapsed();

            // Validate pipeline execution
            assert!(result.metadata.success);
            assert!(execution_time.as_millis() < 1000, "Pipeline should complete within 1 second");

            // Validate field types throughout pipeline
            let expected_types = HashMap::from([
                ("customer_key".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("total_items".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("order_summary".to_string(), datafold::transform::native::types::FieldType::String),
                ("is_large_order".to_string(), datafold::transform::native::types::FieldType::Boolean),
                ("average_price".to_string(), datafold::transform::native::types::FieldType::Number),
            ]);

            TypeSafetyValidator::validate_field_types(&result.values, &expected_types)
                .expect("Type safety validation should pass");

            // Validate business logic
            assert_eq!(result.values.get("customer_key"), Some(&FieldValue::Integer(456)));
            assert_eq!(result.values.get("total_items"), Some(&FieldValue::Integer(3)));
            assert_eq!(result.values.get("is_large_order"), Some(&FieldValue::Boolean(true)));
            assert_eq!(result.values.get("average_price"), Some(&FieldValue::Number(21.076666666666668))); // 63.23 / 3
            assert_eq!(result.values.get("sum_total"), Some(&FieldValue::Number(63.23)));
        }

        #[tokio::test]
        async fn test_user_analytics_pipeline() {
            let executor = test_utils::create_test_executor();
            let analytics_data = test_utils::create_test_analytics_data();

            let input = NativeTransformInput {
                values: analytics_data,
                schema_name: None,
            };

            // Create analytics processing pipeline
            let transforms = vec![
                // Step 1: Filter valid sessions
                test_utils::create_test_transform_spec_with_fields(
                    "filter_valid_sessions",
                    vec![
                        ("user_id", datafold::transform::native::types::FieldType::Integer),
                        ("session_id", datafold::transform::native::types::FieldType::String),
                        ("page_views", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("timestamps", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("event_types", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                    ],
                    ("valid", datafold::transform::native::types::FieldType::Boolean),
                    TransformType::Filter(FilterTransform {
                        condition: FilterCondition::GreaterThan {
                            field: "page_views".to_string(),
                            value: FieldValue::Integer(2),
                        },
                    }),
                ),
                // Step 2: Transform session data with complex expressions
                test_utils::create_test_transform_spec_with_fields(
                    "process_session",
                    vec![
                        ("user_id", datafold::transform::native::types::FieldType::Integer),
                        ("session_id", datafold::transform::native::types::FieldType::String),
                        ("page_views", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("timestamps", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("event_types", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                    ],
                    ("result", datafold::transform::native::types::FieldType::String),
                    TransformType::Map(MapTransform::new({
                        let mut mappings = HashMap::new();
                        mappings.insert("session_duration".to_string(), FieldMapping::Function {
                            name: "length".to_string(),
                            arguments: vec!["timestamps".to_string()],
                        });
                        mappings.insert("has_purchase".to_string(), FieldMapping::Expression {
                            expression: "contains(event_types, \"purchase\")".to_string(),
                        });
                        mappings.insert("session_summary".to_string(), FieldMapping::Expression {
                            expression: "session_id + \" - \" + to_string(session_duration) + \" events, purchase: \" + to_string(has_purchase)".to_string(),
                        });
                        mappings.insert("user_engagement_score".to_string(), FieldMapping::Expression {
                            expression: "length(page_views) * 10 + (has_purchase ? 50 : 0)".to_string(),
                        });
                        mappings
                    })),
                ),
                // Step 3: Calculate session statistics
                test_utils::create_test_transform_spec_with_fields(
                    "session_stats",
                    vec![
                        ("user_id", datafold::transform::native::types::FieldType::Integer),
                        ("session_id", datafold::transform::native::types::FieldType::String),
                        ("page_views", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("timestamps", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("event_types", datafold::transform::native::types::FieldType::Array { element_type: Box::new(datafold::transform::native::types::FieldType::String) }),
                        ("session_duration", datafold::transform::native::types::FieldType::Integer),
                        ("has_purchase", datafold::transform::native::types::FieldType::Boolean),
                        ("session_summary", datafold::transform::native::types::FieldType::String),
                        ("user_engagement_score", datafold::transform::native::types::FieldType::Integer),
                    ],
                    ("average_score", datafold::transform::native::types::FieldType::Number),
                    TransformType::Reduce(ReduceTransform::new(
                        ReducerType::Average { field: "user_engagement_score".to_string() },
                        vec![], // group_by fields
                    )),
                ),
            ];

            let chain_spec = create_test_transform_spec("analytics_pipeline", TransformType::Chain(transforms));

            let result = executor.execute_transform(&chain_spec, input).await.unwrap();

            // Validate analytics processing
            assert!(result.metadata.success);
            assert_eq!(result.values.get("session_duration"), Some(&FieldValue::Integer(4)));
            assert_eq!(result.values.get("has_purchase"), Some(&FieldValue::Boolean(true)));
            assert_eq!(result.values.get("user_engagement_score"), Some(&FieldValue::Integer(90))); // 4 * 10 + 50
            assert_eq!(result.values.get("average_score"), Some(&FieldValue::Number(90.0)));

            // Validate metadata
            assert!(result.metadata.execution_time_ns > 0);
            assert_eq!(result.metadata.transform_type, "Chain([Filter(FilterTransform), Map(MapTransform), Reduce(ReduceTransform)])");
        }

        #[tokio::test]
        async fn test_complex_data_enrichment_pipeline() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            input_data.insert("user_id".to_string(), FieldValue::Integer(123));
            input_data.insert("product_id".to_string(), FieldValue::String("PROD_001".to_string()));
            input_data.insert("rating".to_string(), FieldValue::Integer(4));
            input_data.insert("review_text".to_string(), FieldValue::String("Great product!".to_string()));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            // Create data enrichment pipeline with complex type conversions and validations
            let transforms = vec![
                // Step 1: Validate input data quality
                test_utils::create_test_transform_spec_with_fields(
                    "validate_input",
                    vec![
                        ("user_id", datafold::transform::native::types::FieldType::Integer),
                        ("product_id", datafold::transform::native::types::FieldType::String),
                        ("rating", datafold::transform::native::types::FieldType::Integer),
                        ("review_text", datafold::transform::native::types::FieldType::String),
                    ],
                    ("valid", datafold::transform::native::types::FieldType::Boolean),
                    TransformType::Filter(FilterTransform {
                        condition: FilterCondition::And {
                            conditions: vec![
                                FilterCondition::GreaterThan {
                                    field: "user_id".to_string(),
                                    value: FieldValue::Integer(0),
                                },
                                FilterCondition::GreaterThan {
                                    field: "rating".to_string(),
                                    value: FieldValue::Integer(0),
                                },
                            ],
                        },
                    }),
                ),
                // Step 2: Enrich with derived data and complex expressions
                test_utils::create_test_transform_spec_with_fields(
                    "enrich_data",
                    vec![
                        ("user_id", datafold::transform::native::types::FieldType::Integer),
                        ("product_id", datafold::transform::native::types::FieldType::String),
                        ("rating", datafold::transform::native::types::FieldType::Integer),
                        ("review_text", datafold::transform::native::types::FieldType::String),
                    ],
                    ("result", datafold::transform::native::types::FieldType::String),
                    TransformType::Map(MapTransform::new({
                        let mut mappings = HashMap::new();
                        mappings.insert("review_key".to_string(), FieldMapping::Expression {
                            expression: "to_string(user_id) + \"_\" + product_id".to_string(),
                        });
                        mappings.insert("rating_category".to_string(), FieldMapping::Expression {
                            expression: "rating >= 4 ? \"positive\" : \"neutral\"".to_string(),
                        });
                        mappings.insert("rating_score".to_string(), FieldMapping::Expression {
                            expression: "rating * 10".to_string(),
                        });
                        mappings.insert("is_recommended".to_string(), FieldMapping::Expression {
                            expression: "rating >= 4".to_string(),
                        });
                        mappings.insert("review_length".to_string(), FieldMapping::Function {
                            name: "length".to_string(),
                            arguments: vec!["review_text".to_string()],
                        });
                        mappings.insert("review_sentiment".to_string(), FieldMapping::Expression {
                            expression: "length(review_text) > 20 ? \"detailed\" : \"brief\"".to_string(),
                        });
                        mappings
                    })),
                ),
            ];

            let chain_spec = create_test_transform_spec("enrichment_pipeline", TransformType::Chain(transforms));

            let result = executor.execute_transform(&chain_spec, input).await.unwrap();

            // Validate data enrichment results
            assert_eq!(result.values.get("review_key"), Some(&FieldValue::String("123_PROD_001".to_string())));
            assert_eq!(result.values.get("rating_category"), Some(&FieldValue::String("positive".to_string())));
            assert_eq!(result.values.get("rating_score"), Some(&FieldValue::Integer(40)));
            assert_eq!(result.values.get("is_recommended"), Some(&FieldValue::Boolean(true)));
            assert_eq!(result.values.get("review_length"), Some(&FieldValue::Integer(14)));
            assert_eq!(result.values.get("review_sentiment"), Some(&FieldValue::String("brief".to_string())));

            // Validate comprehensive type safety
            let expected_types = HashMap::from([
                ("review_key".to_string(), datafold::transform::native::types::FieldType::String),
                ("rating_category".to_string(), datafold::transform::native::types::FieldType::String),
                ("rating_score".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("is_recommended".to_string(), datafold::transform::native::types::FieldType::Boolean),
                ("review_length".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("review_sentiment".to_string(), datafold::transform::native::types::FieldType::String),
            ]);

            TypeSafetyValidator::validate_field_types(&result.values, &expected_types)
                .expect("Type safety validation should pass for enrichment pipeline");
        }
    }

    // NTS-3-8-2: Schema Integration and Validation Tests
    mod schema_integration_validation {
        use super::*;
        use test_utils::{create_test_transform_spec, TypeSafetyValidator};

        #[tokio::test]
        async fn test_schema_validation_with_complex_transforms() {
            let executor = test_utils::create_test_executor();

            // First, load a test schema
            let schema_json = test_utils::create_test_schema();
            executor.schema_registry()
                .load_native_schema_from_json(schema_json).await.unwrap();

            let valid_data = test_utils::create_test_user_data();
            let input = NativeTransformInput {
                values: valid_data,
                schema_name: Some("test_schema".to_string()),
            };

            // Test that validation passes with valid data
            let mut field_mappings = HashMap::new();
            field_mappings.insert("validated_id".to_string(), FieldMapping::Direct {
                field: "id".to_string(),
            });
            field_mappings.insert("validated_name".to_string(), FieldMapping::Direct {
                field: "name".to_string(),
            });
            field_mappings.insert("validated_age".to_string(), FieldMapping::Direct {
                field: "age".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("schema_validation_test", TransformType::Map(map_transform));

            let result = executor.execute_transform(&spec, input).await.unwrap();
            assert!(result.metadata.success);

            // Validate schema compliance
            let schema = executor.schema_registry().get_schema("test_schema").unwrap();
            assert_eq!(schema.name, "test_schema");
            assert_eq!(schema.fields.len(), 3);
        }

        #[tokio::test]
        async fn test_schema_validation_failure_handling() {
            let executor = test_utils::create_test_executor();

            // Load a test schema
            let schema_json = test_utils::create_test_schema();
            executor.schema_registry()
                .load_native_schema_from_json(schema_json).await.unwrap();

            // Create invalid data (wrong types)
            let mut invalid_data = HashMap::new();
            invalid_data.insert("id".to_string(), FieldValue::String("not_a_number".to_string()));
            invalid_data.insert("name".to_string(), FieldValue::Integer(123));
            invalid_data.insert("age".to_string(), FieldValue::Boolean(false));

            let input = NativeTransformInput {
                values: invalid_data,
                schema_name: Some("test_schema".to_string()),
            };

            let mut field_mappings = HashMap::new();
            field_mappings.insert("test_field".to_string(), FieldMapping::Direct {
                field: "id".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("schema_validation_failure_test", TransformType::Map(map_transform));

            let result = executor.execute_transform(&spec, input).await;

            // Should fail due to schema validation
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_nested_field_schema_validation() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();

            // Create nested user object with address
            let mut user_obj = HashMap::new();
            user_obj.insert("name".to_string(), FieldValue::String("Alice".to_string()));
            user_obj.insert("age".to_string(), FieldValue::Integer(30));

            let mut address_obj = HashMap::new();
            address_obj.insert("street".to_string(), FieldValue::String("123 Main St".to_string()));
            address_obj.insert("city".to_string(), FieldValue::String("Anytown".to_string()));
            address_obj.insert("zipcode".to_string(), FieldValue::String("12345".to_string()));

            user_obj.insert("address".to_string(), FieldValue::Object(address_obj));
            input_data.insert("user".to_string(), FieldValue::Object(user_obj));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            // Create transforms that access nested fields
            let mut field_mappings = HashMap::new();
            field_mappings.insert("user_name".to_string(), FieldMapping::Expression {
                expression: "user.name".to_string(),
            });
            field_mappings.insert("user_age".to_string(), FieldMapping::Expression {
                expression: "user.age".to_string(),
            });
            field_mappings.insert("user_city".to_string(), FieldMapping::Expression {
                expression: "user.address.city".to_string(),
            });
            field_mappings.insert("full_address".to_string(), FieldMapping::Expression {
                expression: "user.address.street + \", \" + user.address.city + \" \" + user.address.zipcode".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("nested_field_test", TransformType::Map(map_transform));

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate nested field access
            assert_eq!(result.values.get("user_name"), Some(&FieldValue::String("Alice".to_string())));
            assert_eq!(result.values.get("user_age"), Some(&FieldValue::Integer(30)));
            assert_eq!(result.values.get("user_city"), Some(&FieldValue::String("Anytown".to_string())));
            assert_eq!(result.values.get("full_address"), Some(&FieldValue::String("123 Main St, Anytown 12345".to_string())));

            // Validate type safety for nested access
            let expected_types = HashMap::from([
                ("user_name".to_string(), datafold::transform::native::types::FieldType::String),
                ("user_age".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("user_city".to_string(), datafold::transform::native::types::FieldType::String),
                ("full_address".to_string(), datafold::transform::native::types::FieldType::String),
            ]);

            TypeSafetyValidator::validate_field_types(&result.values, &expected_types)
                .expect("Nested field type validation should pass");
        }
    }

    // NTS-3-8-3: Performance Validation with Large Datasets
    mod performance_validation {
        use super::*;
        use test_utils::create_test_transform_spec;

        #[tokio::test]
        async fn test_large_dataset_transform_performance() {
            let executor = test_utils::create_test_executor();
            const RECORD_COUNT: usize = 10000;

            // Create large dataset for performance testing
            let large_dataset = test_utils::create_large_dataset(RECORD_COUNT);

            let input = NativeTransformInput {
                values: large_dataset,
                schema_name: None,
            };

            // Create a performance-intensive transform pipeline
            let transforms = vec![
                // Step 1: Filter active users
                create_test_transform_spec("filter_active", TransformType::Filter(FilterTransform {
                    condition: FilterCondition::Equals {
                        field: "active".to_string(),
                        value: FieldValue::Boolean(true),
                    },
                })),
                // Step 2: Complex data transformation
                create_test_transform_spec("transform_users", TransformType::Map(MapTransform::new({
                    let mut mappings = HashMap::new();
                    mappings.insert("user_score".to_string(), FieldMapping::Expression {
                        expression: "score * 1.1".to_string(), // 10% score increase
                    });
                    mappings.insert("age_group".to_string(), FieldMapping::Expression {
                        expression: "age < 30 ? \"young\" : (age < 50 ? \"middle\" : \"senior\")".to_string(),
                    });
                    mappings.insert("email_domain".to_string(), FieldMapping::Expression {
                        expression: "substring(email, length(email) - 12, length(email))".to_string(),
                    });
                    mappings.insert("address_summary".to_string(), FieldMapping::Expression {
                        expression: "address.city + \", \" + address.zipcode".to_string(),
                    });
                    mappings
                }))),
                // Step 3: Aggregate statistics
                create_test_transform_spec("aggregate_stats", TransformType::Reduce(ReduceTransform::new(
                    ReducerType::Average { field: "user_score".to_string() },
                    vec![], // group_by fields
                ))),
            ];

            let chain_spec = create_test_transform_spec("performance_test", TransformType::Chain(transforms));

            let start_time = Instant::now();
            let result = executor.execute_transform(&chain_spec, input).await.unwrap();
            let execution_time = start_time.elapsed();

            // Performance assertions
            assert!(execution_time.as_millis() < 5000, "Large dataset transform should complete within 5 seconds");
            assert!(result.metadata.success);
            assert!(result.metadata.execution_time_ns > 0);

            // Validate results
            assert!(result.values.contains_key("average"));
            if let FieldValue::Number(score) = result.values.get("average").unwrap() {
                assert!(*score > 50.0, "Average score should be above 50");
            } else {
                panic!("Expected average to be a Number");
            }

            println!("Performance test completed in {}ms for {} records", execution_time.as_millis(), RECORD_COUNT);
        }

        #[tokio::test]
        async fn test_memory_efficiency_with_large_arrays() {
            let executor = test_utils::create_test_executor();

            // Create memory-intensive dataset with large arrays
            let mut memory_dataset = HashMap::new();
            let mut large_array = Vec::new();

            for i in 0..5000 {
                large_array.push(FieldValue::String(format!("item_{}", i)));
            }

            memory_dataset.insert("large_array".to_string(), FieldValue::Array(large_array));
            memory_dataset.insert("count".to_string(), FieldValue::Integer(5000));

            let input = NativeTransformInput {
                values: memory_dataset,
                schema_name: None,
            };

            let mut field_mappings = HashMap::new();
            field_mappings.insert("array_length".to_string(), FieldMapping::Function {
                name: "length".to_string(),
                arguments: vec!["large_array".to_string()],
            });
            field_mappings.insert("first_item".to_string(), FieldMapping::Expression {
                expression: "large_array.0".to_string(),
            });
            field_mappings.insert("last_item".to_string(), FieldMapping::Expression {
                expression: "large_array.4999".to_string(),
            });
            field_mappings.insert("middle_item".to_string(), FieldMapping::Expression {
                expression: "large_array.2500".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("memory_efficiency_test", TransformType::Map(map_transform));

            let start_time = Instant::now();
            let result = executor.execute_transform(&spec, input).await.unwrap();
            let execution_time = start_time.elapsed();

            // Memory efficiency assertions
            assert!(execution_time.as_millis() < 2000, "Memory-intensive operations should be efficient");
            assert!(result.metadata.success);
            assert_eq!(result.values.get("array_length"), Some(&FieldValue::Integer(5000)));
            assert_eq!(result.values.get("first_item"), Some(&FieldValue::String("item_0".to_string())));
            assert_eq!(result.values.get("last_item"), Some(&FieldValue::String("item_4999".to_string())));
            assert_eq!(result.values.get("middle_item"), Some(&FieldValue::String("item_2500".to_string())));
        }

        #[tokio::test]
        async fn test_concurrent_transform_performance() {
            let executor = test_utils::create_test_executor();
            const CONCURRENT_TRANSFORMS: usize = 10;
            const RECORDS_PER_TRANSFORM: usize = 1000;

            let mut handles = Vec::new();

            for i in 0..CONCURRENT_TRANSFORMS {
                let executor_clone = Arc::new(executor.clone());
                let dataset = test_utils::create_large_dataset(RECORDS_PER_TRANSFORM);

                let handle = tokio::spawn(async move {
                    let input = NativeTransformInput {
                        values: dataset,
                        schema_name: None,
                    };

                    let transforms = vec![
                        create_test_transform_spec(&format!("concurrent_filter_{}", i), TransformType::Filter(FilterTransform {
                            condition: FilterCondition::GreaterThan {
                                field: "age".to_string(),
                                value: FieldValue::Integer(25),
                            },
                        })),
                        create_test_transform_spec(&format!("concurrent_map_{}", i), TransformType::Map(MapTransform::new({
                            let mut mappings = HashMap::new();
                            mappings.insert("processed_name".to_string(), FieldMapping::Function {
                                name: "uppercase".to_string(),
                                arguments: vec!["name".to_string()],
                            });
                            mappings
                        }))),
                    ];

                    let chain_spec = create_test_transform_spec(&format!("concurrent_chain_{}", i), TransformType::Chain(transforms));

                    executor_clone.execute_transform(&chain_spec, input).await
                });

                handles.push(handle);
            }

            let start_time = Instant::now();

            // Wait for all concurrent transforms to complete
            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.await.unwrap());
            }

            let total_execution_time = start_time.elapsed();

            // Validate all results
            for result in results {
                assert!(result.unwrap().metadata.success);
            }

            // Performance assertion for concurrent execution
            assert!(total_execution_time.as_millis() < 10000, "Concurrent transforms should complete efficiently");

            println!("Concurrent performance test completed in {}ms for {} parallel transforms", total_execution_time.as_millis(), CONCURRENT_TRANSFORMS);
        }
    }

    // NTS-3-8-4: Error Handling and Recovery Tests
    mod error_handling_and_recovery {
        use super::*;
        use test_utils::create_test_transform_spec;

        #[tokio::test]
        async fn test_comprehensive_error_handling() {
            let executor = test_utils::create_test_executor();

            // Test 1: Missing field access
            let mut input_data = HashMap::new();
            input_data.insert("existing_field".to_string(), FieldValue::String("value".to_string()));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("missing_field".to_string(), FieldMapping::Direct {
                field: "nonexistent_field".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("missing_field_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Missing field access should result in error");

            // Test 2: Invalid expression syntax
            let mut input_data = HashMap::new();
            input_data.insert("a".to_string(), FieldValue::Integer(10));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("invalid_expr".to_string(), FieldMapping::Expression {
                expression: "a + ".to_string(), // Invalid syntax
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("invalid_expression_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Invalid expression syntax should result in error");

            // Test 3: Division by zero
            let mut input_data = HashMap::new();
            input_data.insert("numerator".to_string(), FieldValue::Integer(10));
            input_data.insert("denominator".to_string(), FieldValue::Integer(0));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("result".to_string(), FieldMapping::Expression {
                expression: "numerator / denominator".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("division_by_zero_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Division by zero should result in error");

            // Test 4: Array index out of bounds
            let mut input_data = HashMap::new();
            input_data.insert("array".to_string(), FieldValue::Array(vec![
                FieldValue::Integer(1),
                FieldValue::Integer(2),
                FieldValue::Integer(3),
            ]));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("out_of_bounds".to_string(), FieldMapping::Expression {
                expression: "array.10".to_string(), // Index 10 doesn't exist
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("array_bounds_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Array index out of bounds should result in error");

            // Test 5: Function call with wrong arguments
            let mut input_data = HashMap::new();
            input_data.insert("text".to_string(), FieldValue::String("hello".to_string()));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("wrong_args".to_string(), FieldMapping::Function {
                name: "uppercase".to_string(),
                arguments: vec!["text".to_string(), "extra".to_string()],
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("function_error_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Function call with wrong arguments should result in error");
        }

        #[tokio::test]
        async fn test_nested_transform_error_propagation() {
            let executor = test_utils::create_test_executor();

            let chain_transforms = vec![
                create_test_transform_spec("valid_step", TransformType::Map(MapTransform::new({
                    let mut mappings = HashMap::new();
                    mappings.insert("temp".to_string(), FieldMapping::Constant {
                        value: FieldValue::Integer(42),
                    });
                    mappings
                }))),
                create_test_transform_spec("failing_step", TransformType::Map(MapTransform::new({
                    let mut mappings = HashMap::new();
                    mappings.insert("error_field".to_string(), FieldMapping::Direct {
                        field: "nonexistent".to_string(),
                    });
                    mappings
                }))),
            ];

            let spec = create_test_transform_spec("nested_error_test", TransformType::Chain(chain_transforms));

            let input_data = HashMap::new();
            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await;
            assert!(result.is_err(), "Nested transform errors should propagate correctly");
        }

        #[tokio::test]
        async fn test_error_recovery_mechanisms() {
            let executor = test_utils::create_test_executor();

            // Test graceful handling of partial failures
            let mut input_data = HashMap::new();
            input_data.insert("good_data".to_string(), FieldValue::String("valid".to_string()));
            input_data.insert("bad_data".to_string(), FieldValue::String("".to_string())); // Will cause issues in some operations

            let mut field_mappings = HashMap::new();
            field_mappings.insert("safe_operation".to_string(), FieldMapping::Function {
                name: "length".to_string(),
                arguments: vec!["good_data".to_string()],
            });
            field_mappings.insert("safe_default".to_string(), FieldMapping::Expression {
                expression: "length(good_data) > 0 ? good_data : \"default\"".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("error_recovery_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Verify that safe operations succeeded despite bad data
            assert!(result.metadata.success);
            assert_eq!(result.values.get("safe_operation"), Some(&FieldValue::Integer(5)));
            assert_eq!(result.values.get("safe_default"), Some(&FieldValue::String("valid".to_string())));
        }
    }

    // NTS-3-8-5: Type Safety Validation Throughout Pipeline
    mod type_safety_validation {
        use super::*;
        use test_utils::{create_test_transform_spec, TypeSafetyValidator};

        #[tokio::test]
        async fn test_type_safety_in_complex_expressions() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            input_data.insert("string_val".to_string(), FieldValue::String("hello".to_string()));
            input_data.insert("number_val".to_string(), FieldValue::Integer(42));
            input_data.insert("boolean_val".to_string(), FieldValue::Boolean(true));
            input_data.insert("array_val".to_string(), FieldValue::Array(vec![
                FieldValue::Integer(1),
                FieldValue::Integer(2),
                FieldValue::Integer(3),
            ]));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("string_concat".to_string(), FieldMapping::Expression {
                expression: "string_val + to_string(number_val)".to_string(),
            });
            field_mappings.insert("numeric_calculation".to_string(), FieldMapping::Expression {
                expression: "number_val * 2 + 10".to_string(),
            });
            field_mappings.insert("boolean_logic".to_string(), FieldMapping::Expression {
                expression: "boolean_val && number_val > 40".to_string(),
            });
            field_mappings.insert("array_operation".to_string(), FieldMapping::Function {
                name: "length".to_string(),
                arguments: vec!["array_val".to_string()],
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("type_safety_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate type safety
            let expected_types = HashMap::from([
                ("string_concat".to_string(), datafold::transform::native::types::FieldType::String),
                ("numeric_calculation".to_string(), datafold::transform::native::types::FieldType::Integer),
                ("boolean_logic".to_string(), datafold::transform::native::types::FieldType::Boolean),
                ("array_operation".to_string(), datafold::transform::native::types::FieldType::Integer),
            ]);

            TypeSafetyValidator::validate_field_types(&result.values, &expected_types)
                .expect("Type safety validation should pass");

            // Validate actual values
            assert_eq!(result.values.get("string_concat"), Some(&FieldValue::String("hello42".to_string())));
            assert_eq!(result.values.get("numeric_calculation"), Some(&FieldValue::Integer(94))); // 42 * 2 + 10
            assert_eq!(result.values.get("boolean_logic"), Some(&FieldValue::Boolean(true)));
            assert_eq!(result.values.get("array_operation"), Some(&FieldValue::Integer(3)));
        }

        #[tokio::test]
        async fn test_type_conversion_safety() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            input_data.insert("mixed_types".to_string(), FieldValue::Array(vec![
                FieldValue::String("123".to_string()),
                FieldValue::Integer(456),
                FieldValue::Number(789.0),
                FieldValue::Boolean(true),
            ]));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("string_conversion".to_string(), FieldMapping::Function {
                name: "to_string".to_string(),
                arguments: vec!["mixed_types".to_string()],
            });
            field_mappings.insert("number_conversion".to_string(), FieldMapping::Function {
                name: "to_number".to_string(),
                arguments: vec!["mixed_types".to_string()],
            });
            field_mappings.insert("boolean_conversion".to_string(), FieldMapping::Function {
                name: "to_boolean".to_string(),
                arguments: vec!["mixed_types".to_string()],
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("type_conversion_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate type conversion results
            assert_eq!(result.values.get("string_conversion"), Some(&FieldValue::String("[\"123\", 456, 789.0, true]".to_string())));
            assert_eq!(result.values.get("number_conversion"), Some(&FieldValue::Number(0.0))); // Default fallback for mixed array
            assert_eq!(result.values.get("boolean_conversion"), Some(&FieldValue::Boolean(true))); // Non-empty array is truthy
        }

        #[tokio::test]
        async fn test_array_type_safety() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            input_data.insert("homogeneous_array".to_string(), FieldValue::Array(vec![
                FieldValue::Integer(1),
                FieldValue::Integer(2),
                FieldValue::Integer(3),
            ]));
            input_data.insert("heterogeneous_array".to_string(), FieldValue::Array(vec![
                FieldValue::String("a".to_string()),
                FieldValue::Integer(1),
                FieldValue::Boolean(true),
            ]));
            input_data.insert("empty_array".to_string(), FieldValue::Array(vec![]));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("sum_homogeneous".to_string(), FieldMapping::Function {
                name: "sum".to_string(),
                arguments: vec!["homogeneous_array".to_string()],
            });
            field_mappings.insert("concat_heterogeneous".to_string(), FieldMapping::Function {
                name: "concat".to_string(),
                arguments: vec!["heterogeneous_array".to_string()],
            });
            field_mappings.insert("length_empty".to_string(), FieldMapping::Function {
                name: "length".to_string(),
                arguments: vec!["empty_array".to_string()],
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("array_type_safety_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate array operation results
            assert_eq!(result.values.get("sum_homogeneous"), Some(&FieldValue::Number(6.0))); // 1 + 2 + 3
            assert_eq!(result.values.get("concat_heterogeneous"), Some(&FieldValue::String("a1true".to_string())));
            assert_eq!(result.values.get("length_empty"), Some(&FieldValue::Integer(0)));
        }

        #[tokio::test]
        async fn test_object_field_type_safety() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            let mut user_object = HashMap::new();
            user_object.insert("name".to_string(), FieldValue::String("Alice".to_string()));
            user_object.insert("age".to_string(), FieldValue::Integer(30));
            user_object.insert("active".to_string(), FieldValue::Boolean(true));

            input_data.insert("user".to_string(), FieldValue::Object(user_object));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("user_name".to_string(), FieldMapping::Expression {
                expression: "user.name".to_string(),
            });
            field_mappings.insert("user_age".to_string(), FieldMapping::Expression {
                expression: "user.age".to_string(),
            });
            field_mappings.insert("user_status".to_string(), FieldMapping::Expression {
                expression: "if(user.active, \"active\", \"inactive\")".to_string(),
            });
            field_mappings.insert("user_summary".to_string(), FieldMapping::Expression {
                expression: "user.name + \" (\" + to_string(user.age) + \")\"".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("object_field_safety_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate object field access type safety
            assert_eq!(result.values.get("user_name"), Some(&FieldValue::String("Alice".to_string())));
            assert_eq!(result.values.get("user_age"), Some(&FieldValue::Integer(30)));
            assert_eq!(result.values.get("user_status"), Some(&FieldValue::String("active".to_string())));
            assert_eq!(result.values.get("user_summary"), Some(&FieldValue::String("Alice (30)".to_string())));
        }
    }

    // NTS-3-8-6: Backward Compatibility and Migration Tests
    mod backward_compatibility_migration {
        use super::*;
        use test_utils::create_test_transform_spec;

        #[tokio::test]
        async fn test_mixed_legacy_and_native_transforms() {
            let executor = test_utils::create_test_executor();

            let mut input_data = HashMap::new();
            input_data.insert("legacy_field".to_string(), FieldValue::String("legacy_value".to_string()));
            input_data.insert("native_field".to_string(), FieldValue::Integer(42));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("legacy_processed".to_string(), FieldMapping::Function {
                name: "uppercase".to_string(),
                arguments: vec!["legacy_field".to_string()],
            });
            field_mappings.insert("native_processed".to_string(), FieldMapping::Expression {
                expression: "native_field * 2".to_string(),
            });
            field_mappings.insert("combined_result".to_string(), FieldMapping::Expression {
                expression: "legacy_processed + \"_\" + to_string(native_processed)".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("mixed_transform_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate backward compatibility
            assert!(result.metadata.success);
            assert_eq!(result.values.get("legacy_processed"), Some(&FieldValue::String("LEGACY_VALUE".to_string())));
            assert_eq!(result.values.get("native_processed"), Some(&FieldValue::Integer(84)));
            assert_eq!(result.values.get("combined_result"), Some(&FieldValue::String("LEGACY_VALUE_84".to_string())));
        }

        #[tokio::test]
        async fn test_data_format_migration_scenarios() {
            let executor = test_utils::create_test_executor();

            // Test migrating from old JSON format to native format
            let mut migration_data = HashMap::new();
            migration_data.insert("old_format".to_string(), FieldValue::String("old_data".to_string()));
            migration_data.insert("new_format".to_string(), FieldValue::Integer(100));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("migrated_data".to_string(), FieldMapping::Expression {
                expression: "if(new_format > 0, to_string(new_format), old_format)".to_string(),
            });
            field_mappings.insert("migration_status".to_string(), FieldMapping::Expression {
                expression: "if(new_format > 0, \"migrated\", \"legacy\")".to_string(),
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("migration_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: migration_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate migration logic
            assert_eq!(result.values.get("migrated_data"), Some(&FieldValue::String("100".to_string())));
            assert_eq!(result.values.get("migration_status"), Some(&FieldValue::String("migrated".to_string())));
        }

        #[tokio::test]
        async fn test_function_registry_compatibility() {
            let executor = test_utils::create_test_executor();

            // Test that all built-in functions work correctly with various input types
            let mut input_data = HashMap::new();
            input_data.insert("test_string".to_string(), FieldValue::String("hello".to_string()));
            input_data.insert("test_number".to_string(), FieldValue::Number(42.5));
            input_data.insert("test_integer".to_string(), FieldValue::Integer(42));
            input_data.insert("test_boolean".to_string(), FieldValue::Boolean(true));
            input_data.insert("test_array".to_string(), FieldValue::Array(vec![
                FieldValue::String("a".to_string()),
                FieldValue::String("b".to_string()),
                FieldValue::String("c".to_string()),
            ]));

            let mut field_mappings = HashMap::new();
            field_mappings.insert("string_upper".to_string(), FieldMapping::Function {
                name: "uppercase".to_string(),
                arguments: vec!["test_string".to_string()],
            });
            field_mappings.insert("number_round".to_string(), FieldMapping::Function {
                name: "round".to_string(),
                arguments: vec!["test_number".to_string()],
            });
            field_mappings.insert("array_length".to_string(), FieldMapping::Function {
                name: "length".to_string(),
                arguments: vec!["test_array".to_string()],
            });
            field_mappings.insert("concat_test".to_string(), FieldMapping::Function {
                name: "concat".to_string(),
                arguments: vec!["test_array".to_string()],
            });

            let map_transform = MapTransform::new(field_mappings);
            let spec = create_test_transform_spec("function_compatibility_test", TransformType::Map(map_transform));

            let input = NativeTransformInput {
                values: input_data,
                schema_name: None,
            };

            let result = executor.execute_transform(&spec, input).await.unwrap();

            // Validate function compatibility
            assert_eq!(result.values.get("string_upper"), Some(&FieldValue::String("HELLO".to_string())));
            assert_eq!(result.values.get("number_round"), Some(&FieldValue::Number(43.0)));
            assert_eq!(result.values.get("array_length"), Some(&FieldValue::Integer(3)));
            assert_eq!(result.values.get("concat_test"), Some(&FieldValue::String("abc".to_string())));
        }
    }
}