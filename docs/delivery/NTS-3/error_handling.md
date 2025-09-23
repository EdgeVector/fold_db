# Error Handling and Troubleshooting Guide

This comprehensive guide covers error handling patterns, common issues, and troubleshooting techniques for the Native Transform System (NTS-3).

## Table of Contents

- [Error Types](#error-types)
- [Error Handling Strategies](#error-handling-strategies)
- [Common Issues and Solutions](#common-issues-and-solutions)
- [Debugging Techniques](#debugging-techniques)
- [Performance Troubleshooting](#performance-troubleshooting)
- [Best Practices](#best-practices)

## Error Types

### Transform Execution Errors

#### Validation Errors

```rust
use datafold::transform::native_executor::TransformExecutionError;

match executor.execute_transform(&spec, input).await {
    Err(TransformExecutionError::ValidationError { transform, reason }) => {
        println!("Transform '{}' validation failed: {}", transform, reason);
        // Handle validation error
    }
    Err(e) => println!("Other error: {:?}", e),
}
```

**Common validation errors:**
- Missing required fields
- Type mismatches
- Invalid field names
- Schema validation failures

#### Execution Errors

```rust
match executor.execute_transform(&spec, input).await {
    Err(TransformExecutionError::ExecutionError { transform, reason }) => {
        println!("Transform '{}' execution failed: {}", transform, reason);
        // Handle execution error
    }
    Ok(result) => {
        println!("Transform succeeded: {:?}", result.values);
    }
}
```

**Common execution errors:**
- Division by zero
- Array index out of bounds
- Function call failures
- Expression evaluation errors

#### Schema Validation Errors

```rust
match executor.execute_transform(&spec, input).await {
    Err(TransformExecutionError::SchemaValidationError { schema, reason }) => {
        println!("Schema '{}' validation failed: {}", schema, reason);
        // Handle schema validation error
    }
    Ok(result) => {
        println!("Transform with schema validation succeeded");
    }
}
```

**Common schema validation errors:**
- Required field missing
- Field type mismatch
- Invalid nested object structure
- Array element validation failure

### Expression Evaluation Errors

#### Variable Resolution Errors

```rust
use datafold::transform::expression_evaluator::ExpressionEvaluationError;

match evaluator.evaluate_expression("missing_field + 1").await {
    Err(ExpressionEvaluationError::VariableNotFound { name }) => {
        println!("Variable '{}' not found in context", name);
        // Handle missing variable
    }
    Err(e) => println!("Other expression error: {:?}", e),
}
```

#### Field Access Errors

```rust
match evaluator.evaluate_expression("user.missing_field").await {
    Err(ExpressionEvaluationError::FieldNotFound { field }) => {
        println!("Field '{}' not found", field);
        // Handle missing field
    }
    Err(ExpressionEvaluationError::InvalidFieldAccess { reason }) => {
        println!("Invalid field access: {}", reason);
        // Handle invalid access
    }
    Ok(result) => println!("Field access succeeded: {:?}", result),
}
```

#### Type Errors

```rust
match evaluator.evaluate_expression("42 + \"hello\"").await {
    Err(ExpressionEvaluationError::TypeError { reason }) => {
        println!("Type error: {}", reason);
        // Handle type error
    }
    Ok(result) => println!("Expression succeeded: {:?}", result),
}
```

### Function Registry Errors

#### Function Not Found

```rust
use datafold::transform::function_registry::FunctionRegistryError;

let registry = FunctionRegistry::with_built_ins();

match registry.execute_function("nonexistent_function", vec![]).await {
    Err(FunctionRegistryError::FunctionNotFound { name }) => {
        println!("Function '{}' not found", name);
        // Handle missing function
    }
    Err(e) => println!("Other function error: {:?}", e),
}
```

#### Parameter Errors

```rust
match registry.execute_function("uppercase", vec![]).await {
    Err(FunctionRegistryError::ParameterCountMismatch { name, expected, actual }) => {
        println!("Function '{}' expects {} parameters, got {}", name, expected, actual);
        // Handle parameter count mismatch
    }
    Err(FunctionRegistryError::ParameterTypeMismatch { name, parameter, expected, actual }) => {
        println!("Function '{}' parameter '{}' expects {:?}, got {:?}", name, parameter, expected, actual);
        // Handle parameter type mismatch
    }
    Ok(result) => println!("Function succeeded: {:?}", result),
}
```

## Error Handling Strategies

### 1. Defensive Programming

#### Safe Field Access

```rust
// Unsafe - may panic
"user.profile.email"

// Safe - handles missing fields
"user != null ? user.profile.email : 'default@example.com'"
```

#### Safe Array Access

```rust
// Unsafe - may panic
"scores.10"

// Safe - checks bounds
"length(scores) > 10 ? scores.10 : null"
```

#### Safe Type Conversion

```rust
// Unsafe - may produce unexpected results
"field + 10"

// Safe - explicit conversion
"to_number(field) + 10"
```

### 2. Error Recovery Patterns

#### Fallback Values

```rust
// Use fallback for missing fields
FieldMapping::Expression {
    expression: "user.name || 'Unknown User'".to_string(),
}

// Use fallback for failed operations
FieldMapping::Expression {
    expression: "safe_divide(numerator, denominator) || 0".to_string(),
}
```

#### Conditional Processing

```rust
// Only process if conditions are met
let filter_condition = FilterCondition::And {
    conditions: vec![
        FilterCondition::GreaterThan {
            field: "age".to_string(),
            value: FieldValue::Integer(0),
        },
        FilterCondition::Contains {
            field: "email".to_string(),
            value: FieldValue::String("@".to_string()),
        },
    ],
};
```

#### Graceful Degradation

```rust
// Multiple fallback strategies
FieldMapping::Expression {
    expression: r#"
        user != null && user.profile != null
            ? user.profile.name
            : (user != null ? user.name : 'Unknown')
    "#.to_string(),
}
```

### 3. Error Aggregation

#### Collect Multiple Errors

```rust
async fn validate_and_transform(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    inputs: Vec<NativeTransformInput>,
) -> Vec<Result<NativeTransformResult, TransformExecutionError>> {
    let mut results = Vec::new();

    for input in inputs {
        match executor.execute_transform(spec, input).await {
            Ok(result) => results.push(Ok(result)),
            Err(e) => results.push(Err(e)),
        }
    }

    results
}
```

#### Error Summary

```rust
fn summarize_errors(results: &[Result<NativeTransformResult, TransformExecutionError>]) {
    let mut error_counts = std::collections::HashMap::new();

    for result in results {
        if let Err(e) = result {
            let error_type = format!("{:?}", e);
            *error_counts.entry(error_type).or_insert(0) += 1;
        }
    }

    println!("Error summary:");
    for (error_type, count) in error_counts {
        println!("  {}: {} occurrences", error_type, count);
    }
}
```

## Common Issues and Solutions

### 1. Field Access Issues

#### Problem: Null Reference Errors

**Symptoms:**
```
ExpressionEvaluationError::FieldNotFound
TransformExecutionError::ExecutionError
```

**Solutions:**

1. **Null Checks:**
```rust
// Check for null before access
"user != null ? user.profile : null"

// Check nested nulls
"user != null && user.profile != null ? user.profile.email : 'default'"
```

2. **Safe Navigation:**
```rust
// Use safe navigation patterns
"coalesce(user.name, 'Unknown')"
"coalesce(user.profile.email, 'no-email@example.com')"
```

3. **Default Values:**
```rust
// Provide defaults in field mappings
FieldMapping::Expression {
    expression: "user.name || 'Default Name'".to_string(),
}
```

#### Problem: Array Index Out of Bounds

**Symptoms:**
```
ExpressionEvaluationError::InvalidFieldAccess
TransformExecutionError::ExecutionError
```

**Solutions:**

1. **Bounds Checking:**
```rust
// Check array length before access
"length(scores) > 0 ? scores.0 : null"
"length(scores) > 1 ? scores.1 : scores.0"
```

2. **Safe Array Functions:**
```rust
// Use built-in array functions
"first(scores)"
"last(scores)"
"length(scores)"
```

3. **Conditional Access:**
```rust
// Only access if array exists and has elements
"scores != null && length(scores) > 0 ? scores.0 : null"
```

### 2. Type Conversion Issues

#### Problem: Implicit Type Conversion Failures

**Symptoms:**
```
ExpressionEvaluationError::TypeError
FunctionRegistryError::ParameterTypeMismatch
```

**Solutions:**

1. **Explicit Type Conversion:**
```rust
// Always use explicit conversion
"to_string(number_field)"
"to_number(string_field)"
"to_boolean(flag_field)"
```

2. **Type Validation:**
```rust
// Validate type before conversion
"is_number(field) ? to_number(field) : 0"
"is_string(field) ? field : 'default'"
```

3. **Safe Arithmetic:**
```rust
// Handle mixed types safely
"to_number(field1) + to_number(field2)"
"to_string(field) + '_suffix'"
```

### 3. Function Call Issues

#### Problem: Function Not Found

**Symptoms:**
```
FunctionRegistryError::FunctionNotFound
TransformExecutionError::ExecutionError
```

**Solutions:**

1. **Verify Function Names:**
```rust
// Check available functions
let available_functions = registry.list_functions();
if !available_functions.contains(&"my_function".to_string()) {
    println!("Function not available");
}
```

2. **Use Built-in Functions:**
```rust
// Use correct built-in function names
"uppercase(text)"  // Not "toUpper"
"lowercase(text)"  // Not "toLower"
"average(scores)"  // Not "avg"
```

3. **Custom Function Registration:**
```rust
// Register custom functions before use
registry.register(custom_function_signature, custom_function_impl)?;
```

#### Problem: Parameter Count Mismatch

**Symptoms:**
```
FunctionRegistryError::ParameterCountMismatch
```

**Solutions:**

1. **Check Function Signatures:**
```rust
// Verify expected parameters
let signature = registry.get_signature("substring")?;
println!("Parameters: {:?}", signature.parameters);
// Expected: [("str", String), ("start", Integer), ("end", Integer)]
```

2. **Correct Parameter Count:**
```rust
// Wrong - missing parameters
"substring(text, 0)"

// Correct - all parameters
"substring(text, 0, 5)"
```

3. **Optional Parameters:**
```rust
// Some functions have optional parameters
"concat([a, b, c])"  // Single array parameter
"concat(a, b, c)"    // Multiple parameters (if supported)
```

### 4. Schema Validation Issues

#### Problem: Schema Not Found

**Symptoms:**
```
TransformExecutionError::SchemaValidationError
SchemaError::SchemaNotFound
```

**Solutions:**

1. **Verify Schema Loading:**
```rust
// Ensure schema is loaded before use
let schema_name = schema_registry.load_native_schema_from_json(schema_json).await?;
println!("Loaded schema: {}", schema_name);
```

2. **Check Schema Name:**
```rust
// Use correct schema name in transform input
let input = NativeTransformInput {
    values: data,
    schema_name: Some("my_schema".to_string()),  // Must match loaded schema
};
```

3. **List Available Schemas:**
```rust
// Debug available schemas
let available_schemas = schema_registry.list_schemas().await?;
println!("Available schemas: {:?}", available_schemas);
```

#### Problem: Field Validation Failures

**Symptoms:**
```
SchemaValidationError::TypeMismatch
SchemaValidationError::RequiredFieldMissing
```

**Solutions:**

1. **Check Field Types:**
```rust
// Ensure data matches schema field types
let field_def = schema.fields.get("age").unwrap();
println!("Expected type: {:?}", field_def.field_type);

// Wrong
data.insert("age".to_string(), FieldValue::String("30".to_string()));

// Correct
data.insert("age".to_string(), FieldValue::Integer(30));
```

2. **Provide Required Fields:**
```rust
// Ensure all required fields are present
for (field_name, field_def) in &schema.fields {
    if field_def.required && !data.contains_key(field_name) {
        println!("Missing required field: {}", field_name);
        // Add default value or handle error
    }
}
```

3. **Validate Data Before Transform:**
```rust
// Pre-validate data
if !schema_registry.validate_data("schema_name", &data).await? {
    println!("Data validation failed - fix data before transforming");
    return Err("Invalid data".into());
}
```

### 5. Performance Issues

#### Problem: Slow Transform Execution

**Symptoms:**
- Transform execution takes longer than expected
- Memory usage higher than anticipated
- CPU usage spikes during transforms

**Solutions:**

1. **Profile Execution Time:**
```rust
use std::time::Instant;

let start = Instant::now();
let result = executor.execute_transform(&spec, data).await?;
let duration = start.elapsed();

if duration.as_millis() > 100 {
    println!("Slow transform: {:?}", duration);
}
```

2. **Optimize Field Access:**
```rust
// Avoid repeated field traversal
"to_string(user.id) + '_' + user.name"  // Single traversal

// Instead of
"user.id + '_' + user.name"  // Multiple traversals
```

3. **Use Efficient Data Structures:**
```rust
// Use appropriate data structures
FieldValue::Array(vec![/* ordered data */])  // For ordered data
FieldValue::Object(HashMap::new())           // For key-value data
```

#### Problem: Memory Usage Issues

**Symptoms:**
- High memory consumption during transforms
- Memory leaks or excessive allocations

**Solutions:**

1. **Monitor Memory Usage:**
```rust
fn measure_memory_usage(data: &HashMap<String, FieldValue>) -> usize {
    let mut total = 0;
    for (key, value) in data {
        total += key.len() + std::mem::size_of_val(value);
    }
    total
}
```

2. **Reuse Transform Specifications:**
```rust
// Reuse specifications to avoid repeated allocations
let cached_spec = Arc::new(transform_spec);

for data in data_batch {
    executor.execute_transform(&cached_spec, data).await?;
}
```

3. **Process in Batches:**
```rust
// Process data in smaller batches
let batch_size = 1000;
for chunk in data.chunks(batch_size) {
    for item in chunk {
        executor.execute_transform(&spec, item).await?;
    }
}
```

## Debugging Techniques

### 1. Logging and Tracing

#### Enable Debug Logging

```rust
use log::LevelFilter;
use env_logger::Env;

env_logger::init_from_env(Env::default().default_filter_or("debug"));

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Debug logging will show transform execution details
    let result = executor.execute_transform(&spec, data).await?;
    Ok(())
}
```

#### Add Debug Information to Transforms

```rust
// Add debug expressions to track execution
let debug_mappings = HashMap::from([
    ("debug_info".to_string(), FieldMapping::Expression {
        expression: r#"
            'name=' + to_string(name) +
            ', age=' + to_string(age) +
            ', valid=' + to_string(age >= 18)
        "#.to_string(),
    }),
]);
```

### 2. Step-by-Step Debugging

#### Debug Individual Components

```rust
// Debug field access
match evaluator.evaluate_expression("user.name").await {
    Ok(result) => println!("Field access OK: {:?}", result),
    Err(e) => println!("Field access error: {:?}", e),
}

// Debug function calls
match evaluator.evaluate_expression("uppercase('hello')").await {
    Ok(result) => println!("Function call OK: {:?}", result),
    Err(e) => println!("Function call error: {:?}", e),
}

// Debug complex expressions
match evaluator.evaluate_expression("user.age >= 18 && user.active").await {
    Ok(result) => println!("Complex expression OK: {:?}", result),
    Err(e) => println!("Complex expression error: {:?}", e),
}
```

#### Validate Transform Specification

```rust
// Validate transform spec before execution
match spec.validate() {
    Ok(()) => println!("Transform specification is valid"),
    Err(e) => println!("Transform specification error: {:?}", e),
}

// Check field mappings
for (output_field, mapping) in &map_transform.field_mappings {
    println!("Mapping {} -> {:?}", output_field, mapping);
}
```

### 3. Data Inspection

#### Inspect Input Data

```rust
// Log input data structure
println!("Input data keys: {:?}", data.keys().collect::<Vec<_>>());
for (key, value) in &data {
    println!("  {}: {:?} (type: {:?})", key, value, value.field_type());
}
```

#### Inspect Transform Results

```rust
// Log transform results
println!("Transform results keys: {:?}", result.values.keys().collect::<Vec<_>>());
for (key, value) in &result.values {
    println!("  {}: {:?} (type: {:?})", key, value, value.field_type());
}

// Check metadata
println!("Transform metadata: {:?}", result.metadata);
```

#### Compare Expected vs Actual

```rust
// Define expected results
let expected_keys = vec!["user_id", "full_name", "is_adult"];
let actual_keys = result.values.keys().collect::<Vec<_>>();

if expected_keys != actual_keys {
    println!("Key mismatch!");
    println!("Expected: {:?}", expected_keys);
    println!("Actual: {:?}", actual_keys);
}
```

## Performance Troubleshooting

### 1. Execution Profiling

#### Measure Component Performance

```rust
async fn profile_transform_components(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    data: &HashMap<String, FieldValue>,
) -> HashMap<String, Duration> {
    let mut timings = HashMap::new();

    // Profile field access
    let field_access_start = Instant::now();
    // ... field access operations
    let field_access_time = field_access_start.elapsed();
    timings.insert("field_access".to_string(), field_access_time);

    // Profile function calls
    let function_call_start = Instant::now();
    // ... function call operations
    let function_call_time = function_call_start.elapsed();
    timings.insert("function_calls".to_string(), function_call_time);

    // Profile expression evaluation
    let expression_start = Instant::now();
    // ... expression evaluation operations
    let expression_time = expression_start.elapsed();
    timings.insert("expressions".to_string(), expression_time);

    timings
}
```

#### Identify Bottlenecks

```rust
let timings = profile_transform_components(&executor, &spec, &data).await;

let mut sorted_timings: Vec<_> = timings.iter().collect();
sorted_timings.sort_by(|a, b| b.1.cmp(a.1));

println!("Performance bottlenecks (slowest first):");
for (component, duration) in sorted_timings {
    println!("  {}: {:?}", component, duration);
}
```

### 2. Memory Profiling

#### Track Memory Usage

```rust
use std::mem;

fn track_memory_usage(stage: &str) {
    let memory_info = sys_info::mem_info().unwrap();
    println!(
        "{}: Used {} KB out of {} KB",
        stage,
        memory_info.total - memory_info.free,
        memory_info.total
    );
}

async fn profile_memory_usage(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    data: &HashMap<String, FieldValue>,
) {
    track_memory_usage("Before transform");

    let result = executor.execute_transform(spec, data).await.unwrap();

    track_memory_usage("After transform");

    // Check for memory leaks in data structures
    println!("Input data size: {} bytes", mem::size_of_val(data));
    println!("Output data size: {} bytes", mem::size_of_val(&result.values));
}
```

#### Detect Memory Leaks

```rust
async fn detect_memory_leaks(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    data: &HashMap<String, FieldValue>,
    iterations: usize,
) {
    let initial_memory = sys_info::mem_info().unwrap().total - sys_info::mem_info().unwrap().free;

    for i in 0..iterations {
        let _ = executor.execute_transform(spec, data.clone()).await.unwrap();

        if i % 100 == 0 {
            let current_memory = sys_info::mem_info().unwrap().total - sys_info::mem_info().unwrap().free;
            println!("Iteration {}: {} KB used", i, current_memory - initial_memory);

            if current_memory - initial_memory > 10000 {  // 10MB threshold
                println!("Potential memory leak detected!");
                break;
            }
        }
    }
}
```

## Best Practices

### 1. Error Prevention

#### Input Validation

```rust
// Always validate input data before processing
async fn safe_transform(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    data: HashMap<String, FieldValue>,
) -> Result<NativeTransformResult, Box<dyn std::error::Error>> {
    // Pre-validate data
    if let Some(schema_name) = &spec.inputs.get(0).map(|_| "input_schema") {
        if !schema_registry.validate_data(schema_name, &FieldValue::Object(data.clone())).await? {
            return Err("Input data validation failed".into());
        }
    }

    // Execute transform
    let result = executor.execute_transform(spec, NativeTransformInput {
        values: data,
        schema_name: Some("input_schema".to_string()),
    }).await?;

    Ok(result)
}
```

#### Safe Expression Design

```rust
// Use safe expressions that handle edge cases
FieldMapping::Expression {
    expression: r#"
        user != null && user.age != null
            ? (user.age >= 18 ? "adult" : "minor")
            : "unknown"
    "#.to_string(),
}

// Avoid unsafe expressions
// "user.age >= 18"  // May fail if user or age is null
```

### 2. Error Handling

#### Comprehensive Error Handling

```rust
async fn robust_transform_execution(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    input: NativeTransformInput,
) -> Result<NativeTransformResult, String> {
    match executor.execute_transform(spec, input).await {
        Ok(result) => {
            // Validate result
            if result.values.is_empty() {
                return Err("Transform returned empty result".to_string());
            }
            Ok(result)
        }
        Err(TransformExecutionError::ValidationError { transform, reason }) => {
            Err(format!("Validation error in transform '{}': {}", transform, reason))
        }
        Err(TransformExecutionError::ExecutionError { transform, reason }) => {
            Err(format!("Execution error in transform '{}': {}", transform, reason))
        }
        Err(TransformExecutionError::SchemaValidationError { schema, reason }) => {
            Err(format!("Schema validation error for '{}': {}", schema, reason))
        }
        Err(e) => {
            Err(format!("Unexpected error: {:?}", e))
        }
    }
}
```

#### Error Recovery

```rust
async fn transform_with_recovery(
    executor: &NativeTransformExecutor,
    spec: &TransformSpec,
    input: NativeTransformInput,
) -> Result<NativeTransformResult, String> {
    match executor.execute_transform(spec, input).await {
        Ok(result) => Ok(result),
        Err(TransformExecutionError::ValidationError { .. }) => {
            // Try with default values
            let recovered_input = add_default_values(input);
            executor.execute_transform(spec, recovered_input).await
                .map_err(|e| format!("Recovery failed: {:?}", e))
        }
        Err(e) => Err(format!("Non-recoverable error: {:?}", e)),
    }
}

fn add_default_values(mut input: NativeTransformInput) -> NativeTransformInput {
    // Add default values for missing fields
    if !input.values.contains_key("age") {
        input.values.insert("age".to_string(), FieldValue::Integer(0));
    }
    input
}
```

### 3. Debugging

#### Debug Builds

```rust
// Use debug builds for development
cargo build

// Enable debug assertions
RUSTFLAGS="-C debug-assertions" cargo build
```

#### Logging Configuration

```rust
use log::LevelFilter;
use env_logger::Builder;

Builder::new()
    .filter_level(LevelFilter::Debug)
    .init();

// Set specific module logging
std::env::set_var("RUST_LOG", "datafold::transform=debug");
```

#### Debug Utilities

```rust
struct DebugTransform {
    executor: NativeTransformExecutor,
    spec: TransformSpec,
}

impl DebugTransform {
    async fn debug_execute(&self, data: HashMap<String, FieldValue>) -> Result<DebugResult, String> {
        println!("Input: {:?}", data);

        let result = self.executor.execute_transform(&self.spec, NativeTransformInput {
            values: data,
            schema_name: None,
        }).await.map_err(|e| format!("{:?}", e))?;

        println!("Output: {:?}", result.values);
        println!("Metadata: {:?}", result.metadata);

        Ok(result)
    }
}
```

This comprehensive error handling and troubleshooting guide provides all the tools and techniques needed to effectively debug, monitor, and maintain NTS-3 transforms in production environments.