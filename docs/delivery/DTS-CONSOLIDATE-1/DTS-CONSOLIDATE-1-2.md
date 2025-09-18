# [DTS-CONSOLIDATE-1-2] Implement unified execution pattern in executor.rs

[Back to task list](./tasks.md)

## Description

Implement the unified execution pattern in `executor.rs` that consolidates the logic from all three separate executor modules. This creates a single execution path that handles Single, Range, and HashRange schema types while eliminating code duplication.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 20:00:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Unified Execution Method**: Implement single method that handles all schema types
2. **Common Pattern Extraction**: Extract the 8-step execution pattern into reusable methods
3. **Schema-Specific Logic**: Implement focused methods for each schema type
4. **Preserve Functionality**: Maintain identical behavior to existing executors
5. **Performance**: Ensure no performance degradation

## Implementation Plan

### Step 1: Implement Unified Execution Method

1. **Replace Current Implementation**: Update `execute_declarative_transform` to use unified pattern:
   ```rust
   fn execute_declarative_transform(
       transform: &Transform,
       input_values: HashMap<String, JsonValue>,
   ) -> Result<JsonValue, SchemaError> {
       // Extract schema from transform
       let schema = &transform.schema;
       
       // Use unified execution pattern
       Self::execute_declarative_transform_unified(schema, input_values)
   }
   ```

2. **Create Unified Entry Point**: Implement the main unified execution method:
   ```rust
   fn execute_declarative_transform_unified(
       schema: &DeclarativeSchemaDefinition,
       input_values: HashMap<String, JsonValue>,
   ) -> Result<JsonValue, SchemaError> {
       match &schema.schema_type {
           SchemaType::Single => Self::execute_single_pattern(schema, &input_values),
           SchemaType::Range { range_key } => Self::execute_range_pattern(schema, &input_values, range_key),
           SchemaType::HashRange => Self::execute_hashrange_pattern(schema, &input_values),
       }
   }
   ```

### Step 2: Extract Common Execution Pattern

1. **Implement Common Pattern Method**: Create reusable method for the 8-step pattern:
   ```rust
   fn execute_with_common_pattern<F>(
       schema: &DeclarativeSchemaDefinition,
       input_values: &HashMap<String, JsonValue>,
       schema_type_name: &str,
       custom_logic: F,
   ) -> Result<JsonValue, SchemaError>
   where
       F: FnOnce(&DeclarativeSchemaDefinition, &HashMap<String, JsonValue>, Vec<(String, ParsedChain)>, AlignmentValidationResult) -> Result<JsonValue, SchemaError>,
   {
       // 1. Log execution start
       log_schema_execution_start(schema_type_name, &schema.name, None);
       
       // 2. Validate schema
       validate_schema_basic(schema)?;
       
       // 3. Collect expressions
       let all_expressions = collect_expressions_from_schema(schema);
       
       // 4. Parse expressions
       let parsed_chains = parse_expressions_batch(&all_expressions)?;
       
       // 5. Validate field alignment
       let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
       let alignment_result = validate_field_alignment_unified(None, Some(&chains_only))?;
       
       // 6. Execute with custom logic
       let result = custom_logic(schema, input_values, parsed_chains, alignment_result)?;
       
       // 7. Return result
       Ok(result)
   }
   ```

2. **Create Execution Helper Methods**: Implement helper methods for ExecutionEngine setup:
   ```rust
   fn setup_execution_engine(
       parsed_chains: &[(String, ParsedChain)],
       input_data: JsonValue,
       alignment_result: &AlignmentValidationResult,
   ) -> Result<ExecutionResult, SchemaError>
   
   fn aggregate_execution_results(
       parsed_chains: &[(String, ParsedChain)],
       execution_result: &ExecutionResult,
       input_values: &HashMap<String, JsonValue>,
       all_expressions: &[(String, String)],
       schema_type: SchemaType,
   ) -> Result<JsonValue, SchemaError>
   ```

### Step 3: Implement Schema-Specific Patterns

1. **Single Pattern**: Implement focused Single schema execution:
   ```rust
   fn execute_single_pattern(
       schema: &DeclarativeSchemaDefinition,
       input_values: &HashMap<String, JsonValue>,
   ) -> Result<JsonValue, SchemaError> {
       Self::execute_with_common_pattern(
           schema,
           input_values,
           "Single",
           |schema, input_values, parsed_chains, alignment_result| {
               // Single-specific logic: modify expressions with input prefix
               let modified_expressions = modify_expressions_with_input_prefix(&all_expressions, true);
               let modified_chains = parse_expressions_batch(&modified_expressions)?;
               
               // Setup input data with "input" field
               let mut root_object = serde_json::Map::new();
               root_object.insert("input".to_string(), JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
               let input_data = JsonValue::Object(root_object);
               
               // Execute and aggregate
               let execution_result = Self::setup_execution_engine(&modified_chains, input_data, &alignment_result)?;
               Self::aggregate_execution_results(&modified_chains, &execution_result, input_values, &modified_expressions, SchemaType::Single)
           },
       )
   }
   ```

2. **Range Pattern**: Implement focused Range schema execution:
   ```rust
   fn execute_range_pattern(
       schema: &DeclarativeSchemaDefinition,
       input_values: &HashMap<String, JsonValue>,
       range_key: &str,
   ) -> Result<JsonValue, SchemaError> {
       Self::execute_with_common_pattern(
           schema,
           input_values,
           "Range",
           |schema, input_values, parsed_chains, alignment_result| {
               // Range-specific logic: add range key expressions
               let key_expressions = vec![("_range_field".to_string(), range_key.to_string())];
               let all_expressions = collect_expressions_from_schema_with_keys(schema, &key_expressions);
               let parsed_chains = parse_expressions_batch(&all_expressions)?;
               
               // Execute with standard input data
               let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
               let execution_result = Self::setup_execution_engine(&parsed_chains, input_data, &alignment_result)?;
               Self::aggregate_execution_results(&parsed_chains, &execution_result, input_values, &all_expressions, SchemaType::Range)
           },
       )
   }
   ```

3. **HashRange Pattern**: Implement focused HashRange schema execution:
   ```rust
   fn execute_hashrange_pattern(
       schema: &DeclarativeSchemaDefinition,
       input_values: &HashMap<String, JsonValue>,
   ) -> Result<JsonValue, SchemaError> {
       // HashRange-specific validation and key extraction
       let validation_timings = validate_hashrange_schema(schema)?;
       let key_config = extract_hashrange_key_config(schema)?;
       
       Self::execute_with_common_pattern(
           schema,
           input_values,
           "HashRange",
           |schema, input_values, parsed_chains, alignment_result| {
               // HashRange-specific logic: use coordination module
               let result = execute_multi_chain_coordination_with_monitoring(schema, input_values, key_config)?;
               Ok(result)
           },
       )
   }
   ```

### Step 4: Add Helper Functions

1. **Key Config Extraction**: Move from hash_range_executor:
   ```rust
   fn extract_hashrange_key_config(
       schema: &DeclarativeSchemaDefinition,
   ) -> Result<&KeyConfig, SchemaError> {
       let key_config = schema.key.as_ref().ok_or_else(|| {
           SchemaError::InvalidTransform(format!(
               "HashRange schema '{}' must have key configuration with hash_field and range_field", 
               schema.name
           ))
       })?;
       
       info!("📊 HashRange key config - hash_field: {}, range_field: {}", 
             key_config.hash_field, key_config.range_field);
       
       Ok(key_config)
   }
   ```

2. **Performance Logging**: Add performance logging helper:
   ```rust
   fn log_execution_performance(
       schema_type: &str,
       total_duration: Duration,
       validation_duration: Option<Duration>,
   ) {
       if let Some(validation_duration) = validation_duration {
           info!("⏱️ {} execution completed in {:?} (validation: {:?})", 
                 schema_type, total_duration, validation_duration);
       } else {
           info!("⏱️ {} execution completed in {:?}", schema_type, total_duration);
       }
   }
   ```

## Test Plan

### Objective
Verify that the unified execution pattern maintains identical functionality to the separate executors.

### Test Scope
- All existing executor functionality
- Performance characteristics
- Error handling behavior
- Schema type handling

### Key Test Scenarios
1. **Single Schema Execution**: Verify Single schema execution works identically
2. **Range Schema Execution**: Verify Range schema execution works identically  
3. **HashRange Schema Execution**: Verify HashRange schema execution works identically
4. **Error Handling**: Ensure error handling behavior is preserved
5. **Performance**: Verify no performance degradation
6. **Integration**: Ensure integration with existing transform system works

### Success Criteria
- All existing tests pass without modification
- Performance benchmarks show no degradation
- Error handling behavior is identical
- Code compiles successfully
- Unified execution pattern is cleaner and more maintainable

## Files Modified

- `src/transform/executor.rs` - Add unified execution pattern implementation

## Verification

1. **Compilation**: Code compiles successfully with unified execution pattern
2. **Test Suite**: All existing tests pass without modification
3. **Functionality**: All three schema types execute identically to before
4. **Performance**: No performance degradation in execution
5. **Code Quality**: Unified pattern is cleaner and more maintainable
6. **Documentation**: Code is well-documented with clear method signatures
