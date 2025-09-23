# Native Transform System Usage Examples

This document provides comprehensive usage examples for all transform types in the Native Transform System (NTS-3).

## Table of Contents

- [Basic Setup](#basic-setup)
- [Map Transform Examples](#map-transform-examples)
- [Filter Transform Examples](#filter-transform-examples)
- [Reduce Transform Examples](#reduce-transform-examples)
- [Chain Transform Examples](#chain-transform-examples)
- [Complex Expression Examples](#complex-expression-examples)
- [Real-world Scenarios](#real-world-scenarios)

## Basic Setup

All examples assume the following basic setup:

```rust
use datafold::transform::native::transform_spec::{
    TransformSpec, TransformType, MapTransform, FilterTransform,
    ReduceTransform, FieldMapping, FilterCondition, ReducerType
};
use datafold::transform::native::types::{FieldValue, FieldType};
use datafold::transform::native::field_definition::FieldDefinition;
use datafold::transform::native_executor::{NativeTransformExecutor, NativeTransformInput};
use std::collections::HashMap;
```

## Map Transform Examples

### Basic Field Mapping

```rust
async fn basic_field_mapping_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    // Input data
    let mut input_data = HashMap::new();
    input_data.insert("first_name".to_string(), FieldValue::String("John".to_string()));
    input_data.insert("last_name".to_string(), FieldValue::String("Doe".to_string()));
    input_data.insert("age".to_string(), FieldValue::Integer(30));

    // Define field mappings
    let mut field_mappings = HashMap::new();

    // Direct field mapping
    field_mappings.insert("user_id".to_string(), FieldMapping::Direct {
        field: "first_name".to_string(),
    });

    // Expression mapping
    field_mappings.insert("full_name".to_string(), FieldMapping::Expression {
        expression: "first_name + \" \" + last_name".to_string(),
    });

    // Boolean expression
    field_mappings.insert("is_adult".to_string(), FieldMapping::Expression {
        expression: "age >= 18".to_string(),
    });

    // Function call
    field_mappings.insert("name_upper".to_string(), FieldMapping::Function {
        name: "uppercase".to_string(),
        arguments: vec!["full_name".to_string()],
    });

    let map_transform = MapTransform::new(field_mappings);

    // Define field definitions
    let inputs = vec![
        FieldDefinition::new("first_name", FieldType::String),
        FieldDefinition::new("last_name", FieldType::String),
        FieldDefinition::new("age", FieldType::Integer),
    ];

    let output = FieldDefinition::new("result", FieldType::Object {
        fields: HashMap::new(), // Will be inferred
    });

    let spec = TransformSpec::new(
        "basic_mapping",
        inputs,
        output,
        TransformType::Map(map_transform),
    );

    let input = NativeTransformInput {
        values: input_data,
        schema_name: None,
    };

    let result = executor.execute_transform(&spec, input).await?;

    println!("{:?}", result.values);
    // Output:
    // {
    //     "user_id": String("John"),
    //     "full_name": String("John Doe"),
    //     "is_adult": Boolean(true),
    //     "name_upper": String("JOHN DOE")
    // }

    Ok(())
}
```

### Data Type Conversion

```rust
async fn type_conversion_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("string_number".to_string(), FieldValue::String("42".to_string()));
    input_data.insert("boolean_flag".to_string(), FieldValue::Boolean(true));
    input_data.insert("mixed_array".to_string(), FieldValue::Array(vec![
        FieldValue::String("hello".to_string()),
        FieldValue::Integer(123),
        FieldValue::Boolean(false),
    ]));

    let mut field_mappings = HashMap::new();

    // Convert string to number
    field_mappings.insert("parsed_number".to_string(), FieldMapping::Function {
        name: "to_number".to_string(),
        arguments: vec!["string_number".to_string()],
    });

    // Convert boolean to string
    field_mappings.insert("flag_text".to_string(), FieldMapping::Function {
        name: "to_string".to_string(),
        arguments: vec!["boolean_flag".to_string()],
    });

    // Convert boolean to number
    field_mappings.insert("flag_number".to_string(), FieldMapping::Function {
        name: "to_number".to_string(),
        arguments: vec!["boolean_flag".to_string()],
    });

    // Get array length
    field_mappings.insert("array_size".to_string(), FieldMapping::Function {
        name: "length".to_string(),
        arguments: vec!["mixed_array".to_string()],
    });

    let spec = TransformSpec::new(
        "type_conversion",
        vec![
            FieldDefinition::new("string_number", FieldType::String),
            FieldDefinition::new("boolean_flag", FieldType::Boolean),
            FieldDefinition::new("mixed_array", FieldType::Array {
                element_type: Box::new(FieldType::Any),
            }),
        ],
        FieldDefinition::new("result", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Map(MapTransform::new(field_mappings)),
    );

    let result = executor.execute_transform(&spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("{:?}", result.values);
    Ok(())
}
```

### String Manipulation

```rust
async fn string_manipulation_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("text".to_string(), FieldValue::String("  Hello World!  ".to_string()));
    input_data.insert("words".to_string(), FieldValue::Array(vec![
        FieldValue::String("Hello".to_string()),
        FieldValue::String("World".to_string()),
        FieldValue::String("Rust".to_string()),
    ]));

    let mut field_mappings = HashMap::new();

    // String transformations
    field_mappings.insert("upper_text".to_string(), FieldMapping::Function {
        name: "uppercase".to_string(),
        arguments: vec!["text".to_string()],
    });

    field_mappings.insert("lower_text".to_string(), FieldMapping::Function {
        name: "lowercase".to_string(),
        arguments: vec!["text".to_string()],
    });

    field_mappings.insert("trimmed_text".to_string(), FieldMapping::Function {
        name: "trim".to_string(),
        arguments: vec!["text".to_string()],
    });

    field_mappings.insert("text_length".to_string(), FieldMapping::Function {
        name: "length".to_string(),
        arguments: vec!["text".to_string()],
    });

    // Concatenation
    field_mappings.insert("concatenated".to_string(), FieldMapping::Function {
        name: "concat".to_string(),
        arguments: vec!["words".to_string()],
    });

    // Substring extraction
    field_mappings.insert("substring".to_string(), FieldMapping::Function {
        name: "substring".to_string(),
        arguments: vec!["text".to_string(), "8".to_string(), "13".to_string()],
    });

    let spec = TransformSpec::new(
        "string_manipulation",
        vec![
            FieldDefinition::new("text", FieldType::String),
            FieldDefinition::new("words", FieldType::Array {
                element_type: Box::new(FieldType::String),
            }),
        ],
        FieldDefinition::new("result", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Map(MapTransform::new(field_mappings)),
    );

    let result = executor.execute_transform(&spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("{:?}", result.values);
    Ok(())
}
```

## Filter Transform Examples

### Basic Filtering

```rust
async fn basic_filtering_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    // Test data 1: Should pass filter
    let mut data1 = HashMap::new();
    data1.insert("name".to_string(), FieldValue::String("Alice".to_string()));
    data1.insert("age".to_string(), FieldValue::Integer(25));
    data1.insert("active".to_string(), FieldValue::Boolean(true));
    data1.insert("score".to_string(), FieldValue::Number(85.5));

    // Define filter condition
    let filter_condition = FilterCondition::And {
        conditions: vec![
            FilterCondition::GreaterThan {
                field: "age".to_string(),
                value: FieldValue::Integer(18),
            },
            FilterCondition::Equals {
                field: "active".to_string(),
                value: FieldValue::Boolean(true),
            },
            FilterCondition::GreaterThan {
                field: "score".to_string(),
                value: FieldValue::Number(80.0),
            },
        ],
    };

    let filter_transform = FilterTransform {
        condition: filter_condition,
    };

    let inputs = vec![
        FieldDefinition::new("name", FieldType::String),
        FieldDefinition::new("age", FieldType::Integer),
        FieldDefinition::new("active", FieldType::Boolean),
        FieldDefinition::new("score", FieldType::Number),
    ];

    let spec = TransformSpec::new(
        "adult_filter",
        inputs,
        FieldDefinition::new("filtered", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Filter(filter_transform),
    );

    let result1 = executor.execute_transform(&spec, NativeTransformInput {
        values: data1,
        schema_name: None,
    }).await?;

    println!("Data1 passes filter: {}", result1.values.len() > 0);

    // Test data 2: Should fail filter (age too low)
    let mut data2 = HashMap::new();
    data2.insert("name".to_string(), FieldValue::String("Bob".to_string()));
    data2.insert("age".to_string(), FieldValue::Integer(16));
    data2.insert("active".to_string(), FieldValue::Boolean(true));
    data2.insert("score".to_string(), FieldValue::Number(90.0));

    let result2 = executor.execute_transform(&spec, NativeTransformInput {
        values: data2,
        schema_name: None,
    }).await?;

    println!("Data2 passes filter: {}", result2.values.len() > 0);

    Ok(())
}
```

### Complex Filter Conditions

```rust
async fn complex_filtering_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("user_id".to_string(), FieldValue::Integer(123));
    input_data.insert("email".to_string(), FieldValue::String("user@example.com".to_string()));
    input_data.insert("age".to_string(), FieldValue::Integer(30));
    input_data.insert("country".to_string(), FieldValue::String("US".to_string()));
    input_data.insert("subscription".to_string(), FieldValue::String("premium".to_string()));
    input_data.insert("last_login".to_string(), FieldValue::String("2024-01-15".to_string()));

    // Complex filter with nested conditions
    let filter_condition = FilterCondition::Or {
        conditions: vec![
            FilterCondition::And {
                conditions: vec![
                    FilterCondition::Equals {
                        field: "country".to_string(),
                        value: FieldValue::String("US".to_string()),
                    },
                    FilterCondition::Equals {
                        field: "subscription".to_string(),
                        value: FieldValue::String("premium".to_string()),
                    },
                ],
            },
            FilterCondition::And {
                conditions: vec![
                    FilterCondition::Contains {
                        field: "email".to_string(),
                        value: FieldValue::String("example.com".to_string()),
                    },
                    FilterCondition::GreaterThan {
                        field: "age".to_string(),
                        value: FieldValue::Integer(25),
                    },
                ],
            },
        ],
    };

    let filter_transform = FilterTransform {
        condition: filter_condition,
    };

    let spec = TransformSpec::new(
        "complex_filter",
        vec![
            FieldDefinition::new("user_id", FieldType::Integer),
            FieldDefinition::new("email", FieldType::String),
            FieldDefinition::new("age", FieldType::Integer),
            FieldDefinition::new("country", FieldType::String),
            FieldDefinition::new("subscription", FieldType::String),
            FieldDefinition::new("last_login", FieldType::String),
        ],
        FieldDefinition::new("filtered", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Filter(filter_transform),
    );

    let result = executor.execute_transform(&spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("Complex filter passed: {}", result.values.len() > 0);

    Ok(())
}
```

## Reduce Transform Examples

### Aggregation Operations

```rust
async fn aggregation_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    // Create multiple records for aggregation
    let records = vec![
        vec![
            ("user_id".to_string(), FieldValue::Integer(1)),
            ("score".to_string(), FieldValue::Number(85.0)),
            ("category".to_string(), FieldValue::String("A".to_string())),
        ],
        vec![
            ("user_id".to_string(), FieldValue::Integer(2)),
            ("score".to_string(), FieldValue::Number(92.0)),
            ("category".to_string(), FieldValue::String("A".to_string())),
        ],
        vec![
            ("user_id".to_string(), FieldValue::Integer(3)),
            ("score".to_string(), FieldValue::Number(78.0)),
            ("category".to_string(), FieldValue::String("B".to_string())),
        ],
        vec![
            ("user_id".to_string(), FieldValue::Integer(4)),
            ("score".to_string(), FieldValue::Number(96.0)),
            ("category".to_string(), FieldValue::String("B".to_string())),
        ],
    ];

    for (i, record) in records.into_iter().enumerate() {
        let mut input_data = HashMap::from_iter(record);

        // Sum aggregation
        let sum_transform = ReduceTransform::new(
            ReducerType::Sum { field: "score".to_string() },
            vec!["category".to_string()],
        );

        let sum_spec = TransformSpec::new(
            "sum_aggregation",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("score", FieldType::Number),
                FieldDefinition::new("category", FieldType::String),
            ],
            FieldDefinition::new("total", FieldType::Number),
            TransformType::Reduce(sum_transform),
        );

        let sum_result = executor.execute_transform(&sum_spec, NativeTransformInput {
            values: input_data.clone(),
            schema_name: None,
        }).await?;

        println!("Record {} sum result: {:?}", i + 1, sum_result.values);
    }

    // Count aggregation
    let count_transform = ReduceTransform::new(
        ReducerType::Count,
        vec!["category".to_string()],
    );

    let count_spec = TransformSpec::new(
        "count_aggregation",
        vec![
            FieldDefinition::new("user_id", FieldType::Integer),
            FieldDefinition::new("category", FieldType::String),
        ],
        FieldDefinition::new("count", FieldType::Integer),
        TransformType::Reduce(count_transform),
    );

    let count_result = executor.execute_transform(&count_spec, NativeTransformInput {
        values: HashMap::from([
            ("user_id".to_string(), FieldValue::Integer(5)),
            ("category".to_string(), FieldValue::String("A".to_string())),
        ]),
        schema_name: None,
    }).await?;

    println!("Count result: {:?}", count_result.values);

    Ok(())
}
```

### Statistical Operations

```rust
async fn statistical_operations_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("scores".to_string(), FieldValue::Array(vec![
        FieldValue::Number(85.0),
        FieldValue::Number(92.0),
        FieldValue::Number(78.0),
        FieldValue::Number(96.0),
        FieldValue::Number(88.0),
        FieldValue::Number(91.0),
    ]));

    // Average calculation
    let avg_transform = ReduceTransform::new(
        ReducerType::Average { field: "scores".to_string() },
        vec![],
    );

    let avg_spec = TransformSpec::new(
        "average_calculation",
        vec![
            FieldDefinition::new("scores", FieldType::Array {
                element_type: Box::new(FieldType::Number),
            }),
        ],
        FieldDefinition::new("average", FieldType::Number),
        TransformType::Reduce(avg_transform),
    );

    let avg_result = executor.execute_transform(&avg_spec, NativeTransformInput {
        values: input_data.clone(),
        schema_name: None,
    }).await?;

    println!("Average: {:?}", avg_result.values);

    // Min/Max calculation
    let min_transform = ReduceTransform::new(
        ReducerType::Min { field: "scores".to_string() },
        vec![],
    );

    let min_spec = TransformSpec::new(
        "min_calculation",
        vec![
            FieldDefinition::new("scores", FieldType::Array {
                element_type: Box::new(FieldType::Number),
            }),
        ],
        FieldDefinition::new("min", FieldType::Number),
        TransformType::Reduce(min_transform),
    );

    let min_result = executor.execute_transform(&min_spec, NativeTransformInput {
        values: input_data.clone(),
        schema_name: None,
    }).await?;

    let max_transform = ReduceTransform::new(
        ReducerType::Max { field: "scores".to_string() },
        vec![],
    );

    let max_spec = TransformSpec::new(
        "max_calculation",
        vec![
            FieldDefinition::new("scores", FieldType::Array {
                element_type: Box::new(FieldType::Number),
            }),
        ],
        FieldDefinition::new("max", FieldType::Number),
        TransformType::Reduce(max_transform),
    );

    let max_result = executor.execute_transform(&max_spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("Min: {:?}, Max: {:?}", min_result.values, max_result.values);

    Ok(())
}
```

## Chain Transform Examples

### Data Processing Pipeline

```rust
async fn data_processing_pipeline_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let input_data = HashMap::from([
        ("user_id".to_string(), FieldValue::Integer(123)),
        ("name".to_string(), FieldValue::String("Alice Johnson".to_string())),
        ("email".to_string(), FieldValue::String("alice@example.com".to_string())),
        ("age".to_string(), FieldValue::Integer(28)),
        ("country".to_string(), FieldValue::String("US".to_string())),
        ("subscription".to_string(), FieldValue::String("basic".to_string())),
        ("last_login".to_string(), FieldValue::String("2024-01-15".to_string())),
    ]);

    // Define a processing pipeline
    let chain_transforms = vec![
        // Step 1: Filter active users
        TransformSpec::new(
            "filter_active",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("name", FieldType::String),
                FieldDefinition::new("email", FieldType::String),
                FieldDefinition::new("age", FieldType::Integer),
                FieldDefinition::new("country", FieldType::String),
                FieldDefinition::new("subscription", FieldType::String),
                FieldDefinition::new("last_login", FieldType::String),
            ],
            FieldDefinition::new("filtered_user", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Filter(FilterTransform {
                condition: FilterCondition::And {
                    conditions: vec![
                        FilterCondition::GreaterThan {
                            field: "age".to_string(),
                            value: FieldValue::Integer(18),
                        },
                        FilterCondition::Contains {
                            field: "email".to_string(),
                            value: FieldValue::String("example.com".to_string()),
                        },
                    ],
                },
            }),
        ),

        // Step 2: Enrich user data
        TransformSpec::new(
            "enrich_user",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("name", FieldType::String),
                FieldDefinition::new("email", FieldType::String),
                FieldDefinition::new("age", FieldType::Integer),
                FieldDefinition::new("country", FieldType::String),
                FieldDefinition::new("subscription", FieldType::String),
                FieldDefinition::new("last_login", FieldType::String),
            ],
            FieldDefinition::new("enriched_user", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Map(MapTransform::new({
                let mut mappings = HashMap::new();
                mappings.insert("display_name".to_string(), FieldMapping::Function {
                    name: "uppercase".to_string(),
                    arguments: vec!["name".to_string()],
                });
                mappings.insert("domain".to_string(), FieldMapping::Expression {
                    expression: "substring(email, index_of(email, \"@\") + 1, length(email))".to_string(),
                });
                mappings.insert("is_premium".to_string(), FieldMapping::Expression {
                    expression: "subscription == \"premium\"".to_string(),
                });
                mappings
            })),
        ),

        // Step 3: Calculate user statistics
        TransformSpec::new(
            "calculate_stats",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("name", FieldType::String),
                FieldDefinition::new("email", FieldType::String),
                FieldDefinition::new("age", FieldType::Integer),
                FieldDefinition::new("country", FieldType::String),
                FieldDefinition::new("subscription", FieldType::String),
                FieldDefinition::new("last_login", FieldType::String),
                FieldDefinition::new("display_name", FieldType::String),
                FieldDefinition::new("domain", FieldType::String),
                FieldDefinition::new("is_premium", FieldType::Boolean),
            ],
            FieldDefinition::new("user_stats", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Map(MapTransform::new({
                let mut mappings = HashMap::new();
                mappings.insert("user_key".to_string(), FieldMapping::Expression {
                    expression: "to_string(user_id) + \"_\" + country".to_string(),
                });
                mappings.insert("name_length".to_string(), FieldMapping::Function {
                    name: "length".to_string(),
                    arguments: vec!["name".to_string()],
                });
                mappings.insert("account_status".to_string(), FieldMapping::Expression {
                    expression: "is_premium ? \"Premium\" : \"Basic\"".to_string(),
                });
                mappings
            })),
        ),
    ];

    let chain_spec = TransformSpec::new(
        "user_processing_pipeline",
        vec![
            FieldDefinition::new("user_id", FieldType::Integer),
            FieldDefinition::new("name", FieldType::String),
            FieldDefinition::new("email", FieldType::String),
            FieldDefinition::new("age", FieldType::Integer),
            FieldDefinition::new("country", FieldType::String),
            FieldDefinition::new("subscription", FieldType::String),
            FieldDefinition::new("last_login", FieldType::String),
        ],
        FieldDefinition::new("processed_user", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Chain(chain_transforms),
    );

    let result = executor.execute_transform(&chain_spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("Pipeline result: {:?}", result.values);

    Ok(())
}
```

## Complex Expression Examples

### Mathematical Expressions

```rust
async fn mathematical_expressions_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("a".to_string(), FieldValue::Number(10.0));
    input_data.insert("b".to_string(), FieldValue::Number(3.0));
    input_data.insert("c".to_string(), FieldValue::Integer(5));

    let mut field_mappings = HashMap::new();

    // Arithmetic operations
    field_mappings.insert("addition".to_string(), FieldMapping::Expression {
        expression: "a + b".to_string(),
    });

    field_mappings.insert("subtraction".to_string(), FieldMapping::Expression {
        expression: "a - b".to_string(),
    });

    field_mappings.insert("multiplication".to_string(), FieldMapping::Expression {
        expression: "a * b".to_string(),
    });

    field_mappings.insert("division".to_string(), FieldMapping::Expression {
        expression: "a / b".to_string(),
    });

    field_mappings.insert("modulo".to_string(), FieldMapping::Expression {
        expression: "c % 2".to_string(),
    });

    field_mappings.insert("power".to_string(), FieldMapping::Expression {
        expression: "b ^ 2".to_string(),
    });

    // Complex expression
    field_mappings.insert("complex_calc".to_string(), FieldMapping::Expression {
        expression: "(a + b) * c / 2".to_string(),
    });

    let spec = TransformSpec::new(
        "mathematical_expressions",
        vec![
            FieldDefinition::new("a", FieldType::Number),
            FieldDefinition::new("b", FieldType::Number),
            FieldDefinition::new("c", FieldType::Integer),
        ],
        FieldDefinition::new("calculations", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Map(MapTransform::new(field_mappings)),
    );

    let result = executor.execute_transform(&spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("Mathematical calculations: {:?}", result.values);

    Ok(())
}
```

### Conditional Logic

```rust
async fn conditional_logic_example() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut input_data = HashMap::new();
    input_data.insert("score".to_string(), FieldValue::Number(85.5));
    input_data.insert("age".to_string(), FieldValue::Integer(20));
    input_data.insert("active".to_string(), FieldValue::Boolean(true));

    let mut field_mappings = HashMap::new();

    // Ternary operations
    field_mappings.insert("grade".to_string(), FieldMapping::Expression {
        expression: "score >= 90 ? \"A\" : (score >= 80 ? \"B\" : (score >= 70 ? \"C\" : \"F\"))".to_string(),
    });

    field_mappings.insert("status".to_string(), FieldMapping::Expression {
        expression: "active ? \"Active\" : \"Inactive\"".to_string(),
    });

    field_mappings.insert("age_group".to_string(), FieldMapping::Expression {
        expression: "age < 18 ? \"Minor\" : (age < 65 ? \"Adult\" : \"Senior\")".to_string(),
    });

    // Nested conditions
    field_mappings.insert("priority".to_string(), FieldMapping::Expression {
        expression: "score >= 90 && active ? \"High\" : (score >= 70 || age < 25 ? \"Medium\" : \"Low\")".to_string(),
    });

    let spec = TransformSpec::new(
        "conditional_logic",
        vec![
            FieldDefinition::new("score", FieldType::Number),
            FieldDefinition::new("age", FieldType::Integer),
            FieldDefinition::new("active", FieldType::Boolean),
        ],
        FieldDefinition::new("results", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Map(MapTransform::new(field_mappings)),
    );

    let result = executor.execute_transform(&spec, NativeTransformInput {
        values: input_data,
        schema_name: None,
    }).await?;

    println!("Conditional results: {:?}", result.values);

    Ok(())
}
```

## Real-world Scenarios

### E-commerce Order Processing

```rust
async fn ecommerce_order_processing() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut order_data = HashMap::new();
    order_data.insert("order_id".to_string(), FieldValue::String("ORD-001".to_string()));
    order_data.insert("customer_id".to_string(), FieldValue::Integer(456));
    order_data.insert("items".to_string(), FieldValue::Array(vec![
        FieldValue::String("widget_a".to_string()),
        FieldValue::String("widget_b".to_string()),
        FieldValue::String("widget_c".to_string()),
    ]));
    order_data.insert("quantities".to_string(), FieldValue::Array(vec![
        FieldValue::Integer(2),
        FieldValue::Integer(1),
        FieldValue::Integer(3),
    ]));
    order_data.insert("prices".to_string(), FieldValue::Array(vec![
        FieldValue::Number(10.99),
        FieldValue::Number(25.50),
        FieldValue::Number(5.25),
    ]));
    order_data.insert("subtotal".to_string(), FieldValue::Number(63.23));
    order_data.insert("tax".to_string(), FieldValue::Number(3.79));
    order_data.insert("shipping".to_string(), FieldValue::Number(5.99));

    let chain_transforms = vec![
        // Step 1: Validate order
        TransformSpec::new(
            "validate_order",
            vec![
                FieldDefinition::new("order_id", FieldType::String),
                FieldDefinition::new("customer_id", FieldType::Integer),
                FieldDefinition::new("items", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("quantities", FieldType::Array {
                    element_type: Box::new(FieldType::Integer),
                }),
                FieldDefinition::new("prices", FieldType::Array {
                    element_type: Box::new(FieldType::Number),
                }),
                FieldDefinition::new("subtotal", FieldType::Number),
                FieldDefinition::new("tax", FieldType::Number),
                FieldDefinition::new("shipping", FieldType::Number),
            ],
            FieldDefinition::new("valid_order", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Filter(FilterTransform {
                condition: FilterCondition::And {
                    conditions: vec![
                        FilterCondition::GreaterThan {
                            field: "subtotal".to_string(),
                            value: FieldValue::Number(0.0),
                        },
                        FilterCondition::GreaterThan {
                            field: "customer_id".to_string(),
                            value: FieldValue::Integer(0),
                        },
                    ],
                },
            }),
        ),

        // Step 2: Process order details
        TransformSpec::new(
            "process_order",
            vec![
                FieldDefinition::new("order_id", FieldType::String),
                FieldDefinition::new("customer_id", FieldType::Integer),
                FieldDefinition::new("items", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("quantities", FieldType::Array {
                    element_type: Box::new(FieldType::Integer),
                }),
                FieldDefinition::new("prices", FieldType::Array {
                    element_type: Box::new(FieldType::Number),
                }),
                FieldDefinition::new("subtotal", FieldType::Number),
                FieldDefinition::new("tax", FieldType::Number),
                FieldDefinition::new("shipping", FieldType::Number),
            ],
            FieldDefinition::new("processed_order", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Map(MapTransform::new({
                let mut mappings = HashMap::new();
                mappings.insert("total_items".to_string(), FieldMapping::Function {
                    name: "length".to_string(),
                    arguments: vec!["items".to_string()],
                });
                mappings.insert("total_amount".to_string(), FieldMapping::Expression {
                    expression: "subtotal + tax + shipping".to_string(),
                });
                mappings.insert("customer_key".to_string(), FieldMapping::Expression {
                    expression: "\"CUST_\" + to_string(customer_id)".to_string(),
                });
                mappings.insert("order_summary".to_string(), FieldMapping::Expression {
                    expression: "order_id + \" - \" + to_string(total_items) + \" items, $\" + to_string(total_amount)".to_string(),
                });
                mappings.insert("is_large_order".to_string(), FieldMapping::Expression {
                    expression: "total_amount > 100.0".to_string(),
                });
                mappings
            })),
        ),
    ];

    let chain_spec = TransformSpec::new(
        "ecommerce_processing",
        vec![
            FieldDefinition::new("order_id", FieldType::String),
            FieldDefinition::new("customer_id", FieldType::Integer),
            FieldDefinition::new("items", FieldType::Array {
                element_type: Box::new(FieldType::String),
            }),
            FieldDefinition::new("quantities", FieldType::Array {
                element_type: Box::new(FieldType::Integer),
            }),
            FieldDefinition::new("prices", FieldType::Array {
                element_type: Box::new(FieldType::Number),
            }),
            FieldDefinition::new("subtotal", FieldType::Number),
            FieldDefinition::new("tax", FieldType::Number),
            FieldDefinition::new("shipping", FieldType::Number),
        ],
        FieldDefinition::new("final_order", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Chain(chain_transforms),
    );

    let result = executor.execute_transform(&chain_spec, NativeTransformInput {
        values: order_data,
        schema_name: None,
    }).await?;

    println!("E-commerce processing result: {:?}", result.values);

    Ok(())
}
```

### User Analytics Pipeline

```rust
async fn user_analytics_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeTransformExecutor::new();

    let mut session_data = HashMap::new();
    session_data.insert("user_id".to_string(), FieldValue::Integer(789));
    session_data.insert("session_id".to_string(), FieldValue::String("sess_abc123".to_string()));
    session_data.insert("page_views".to_string(), FieldValue::Array(vec![
        FieldValue::String("/home".to_string()),
        FieldValue::String("/products".to_string()),
        FieldValue::String("/cart".to_string()),
        FieldValue::String("/checkout".to_string()),
        FieldValue::String("/thank-you".to_string()),
    ]));
    session_data.insert("timestamps".to_string(), FieldValue::Array(vec![
        FieldValue::String("2024-01-01T10:00:00Z".to_string()),
        FieldValue::String("2024-01-01T10:05:00Z".to_string()),
        FieldValue::String("2024-01-01T10:10:00Z".to_string()),
        FieldValue::String("2024-01-01T10:15:00Z".to_string()),
        FieldValue::String("2024-01-01T10:20:00Z".to_string()),
    ]));
    session_data.insert("event_types".to_string(), FieldValue::Array(vec![
        FieldValue::String("page_view".to_string()),
        FieldValue::String("scroll".to_string()),
        FieldValue::String("add_to_cart".to_string()),
        FieldValue::String("purchase".to_string()),
        FieldValue::String("page_view".to_string()),
    ]));
    session_data.insert("session_duration".to_string(), FieldValue::Integer(1200));

    let chain_transforms = vec![
        // Step 1: Filter valid sessions
        TransformSpec::new(
            "filter_valid_sessions",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("session_id", FieldType::String),
                FieldDefinition::new("page_views", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("timestamps", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("event_types", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("session_duration", FieldType::Integer),
            ],
            FieldDefinition::new("valid_session", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Filter(FilterTransform {
                condition: FilterCondition::And {
                    conditions: vec![
                        FilterCondition::GreaterThan {
                            field: "session_duration".to_string(),
                            value: FieldValue::Integer(60),
                        },
                        FilterCondition::GreaterThan {
                            field: "page_views".to_string(),
                            value: FieldValue::Integer(2),
                        },
                    ],
                },
            }),
        ),

        // Step 2: Analyze session data
        TransformSpec::new(
            "analyze_session",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("session_id", FieldType::String),
                FieldDefinition::new("page_views", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("timestamps", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("event_types", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("session_duration", FieldType::Integer),
            ],
            FieldDefinition::new("analyzed_session", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Map(MapTransform::new({
                let mut mappings = HashMap::new();
                mappings.insert("total_page_views".to_string(), FieldMapping::Function {
                    name: "length".to_string(),
                    arguments: vec!["page_views".to_string()],
                });
                mappings.insert("unique_pages".to_string(), FieldMapping::Function {
                    name: "length".to_string(),
                    arguments: vec!["page_views".to_string()], // In real implementation, would be unique count
                });
                mappings.insert("has_purchase".to_string(), FieldMapping::Expression {
                    expression: "contains(event_types, \"purchase\")".to_string(),
                });
                mappings.insert("session_summary".to_string(), FieldMapping::Expression {
                    expression: "session_id + \" - \" + to_string(total_page_views) + \" pages, purchase: \" + to_string(has_purchase)".to_string(),
                });
                mappings.insert("avg_time_per_page".to_string(), FieldMapping::Expression {
                    expression: "session_duration / total_page_views".to_string(),
                });
                mappings
            })),
        ),

        // Step 3: Calculate engagement score
        TransformSpec::new(
            "calculate_engagement",
            vec![
                FieldDefinition::new("user_id", FieldType::Integer),
                FieldDefinition::new("session_id", FieldType::String),
                FieldDefinition::new("page_views", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("timestamps", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("event_types", FieldType::Array {
                    element_type: Box::new(FieldType::String),
                }),
                FieldDefinition::new("session_duration", FieldType::Integer),
                FieldDefinition::new("total_page_views", FieldType::Integer),
                FieldDefinition::new("unique_pages", FieldType::Integer),
                FieldDefinition::new("has_purchase", FieldType::Boolean),
                FieldDefinition::new("session_summary", FieldType::String),
                FieldDefinition::new("avg_time_per_page", FieldType::Integer),
            ],
            FieldDefinition::new("session_analytics", FieldType::Object {
                fields: HashMap::new(),
            }),
            TransformType::Map(MapTransform::new({
                let mut mappings = HashMap::new();
                mappings.insert("engagement_score".to_string(), FieldMapping::Expression {
                    expression: "total_page_views * 10 + (has_purchase ? 50 : 0) + (session_duration > 300 ? 20 : 0)".to_string(),
                });
                mappings.insert("session_quality".to_string(), FieldMapping::Expression {
                    expression: "engagement_score >= 100 ? \"High\" : (engagement_score >= 50 ? \"Medium\" : \"Low\")".to_string(),
                });
                mappings.insert("user_key".to_string(), FieldMapping::Expression {
                    expression: "\"USER_\" + to_string(user_id) + \"_\" + substring(session_id, 5, 11)".to_string(),
                });
                mappings
            })),
        ),
    ];

    let chain_spec = TransformSpec::new(
        "user_analytics_pipeline",
        vec![
            FieldDefinition::new("user_id", FieldType::Integer),
            FieldDefinition::new("session_id", FieldType::String),
            FieldDefinition::new("page_views", FieldType::Array {
                element_type: Box::new(FieldType::String),
            }),
            FieldDefinition::new("timestamps", FieldType::Array {
                element_type: Box::new(FieldType::String),
            }),
            FieldDefinition::new("event_types", FieldType::Array {
                element_type: Box::new(FieldType::String),
            }),
            FieldDefinition::new("session_duration", FieldType::Integer),
        ],
        FieldDefinition::new("analytics_result", FieldType::Object {
            fields: HashMap::new(),
        }),
        TransformType::Chain(chain_transforms),
    );

    let result = executor.execute_transform(&chain_spec, NativeTransformInput {
        values: session_data,
        schema_name: None,
    }).await?;

    println!("User analytics result: {:?}", result.values);

    Ok(())
}
```

These examples demonstrate the full range of capabilities available in the Native Transform System, from basic field mappings to complex multi-step processing pipelines suitable for real-world applications.