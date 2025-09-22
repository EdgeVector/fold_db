# Native Transform Architecture Design

## Overview

This document outlines a clean, JSON-free architecture for the transform system using native Rust types. The design eliminates JSON passing between internal components while maintaining full functionality and performance.

## Core Principles

1. **Native Rust Types**: Use structs and enums instead of `JsonValue`
2. **Type Safety**: Compile-time validation instead of runtime JSON parsing
3. **Performance**: Direct struct operations without serialization overhead
4. **JSON Only at Boundaries**: Use JSON only for API requests/responses and persistence
5. **Clear Data Flow**: Explicit, typed data transformations

## Architecture Components

### 1. Core Data Types

```rust
// src/transform/types.rs

/// Native representation of a field value
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Object(HashMap<String, FieldValue>),
    Null,
}

/// Schema field definition with native types
#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default_value: Option<FieldValue>,
}

/// Supported field types
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Number,
    Integer,
    Boolean,
    Array(Box<FieldType>),
    Object(HashMap<String, FieldType>),
}

/// Transform input/output specification
#[derive(Debug, Clone)]
pub struct TransformSpec {
    pub inputs: Vec<FieldDefinition>,
    pub output: FieldDefinition,
    pub transform_type: TransformType,
}

/// Types of transforms supported
#[derive(Debug, Clone)]
pub enum TransformType {
    Map(Box<MapTransform>),
    Filter(Box<FilterTransform>),
    Reduce(Box<ReduceTransform>),
    Chain(Vec<TransformSpec>),
}

/// Map transform definition
#[derive(Debug, Clone)]
pub struct MapTransform {
    pub field_mappings: HashMap<String, FieldMapping>,
}

/// Field mapping specification
#[derive(Debug, Clone)]
pub enum FieldMapping {
    Direct(String), // Direct field copy
    Expression(String), // Expression to evaluate
    Constant(FieldValue), // Constant value
    Function(String, Vec<String>), // Function call with args
}
```

### 2. Schema-to-Struct Mapping

```rust
// src/schema/native_types.rs

/// Native schema representation
#[derive(Debug, Clone)]
pub struct NativeSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub key_config: KeyConfig,
    pub transform_specs: Vec<TransformSpec>,
}

/// Key configuration for different schema types
#[derive(Debug, Clone)]
pub enum KeyConfig {
    Single { key_field: String },
    Range { hash_field: String, range_field: String },
    HashRange { hash_field: String, range_field: String },
}

/// Schema registry with native types
pub struct NativeSchemaRegistry {
    schemas: HashMap<String, NativeSchema>,
    field_cache: HashMap<String, FieldDefinition>,
}

impl NativeSchemaRegistry {
    /// Convert JSON schema to native schema
    pub fn from_json_schema(json_schema: &JsonSchema) -> Result<Self, SchemaError> {
        // Convert JSON schema to native types
        // Validate field types and relationships
        // Build native schema representation
    }
    
    /// Get field definition by name
    pub fn get_field(&self, schema_name: &str, field_name: &str) -> Option<&FieldDefinition> {
        self.schemas.get(schema_name)?.fields.get(field_name)
    }
}
```

### 3. Transform Execution Engine

```rust
// src/transform/execution_engine.rs

/// Native transform execution engine
pub struct NativeTransformEngine {
    schema_registry: Arc<NativeSchemaRegistry>,
    function_registry: FunctionRegistry,
}

/// Function registry for transform operations
pub struct FunctionRegistry {
    functions: HashMap<String, Box<dyn TransformFunction>>,
}

/// Trait for transform functions
pub trait TransformFunction: Send + Sync {
    fn execute(&self, args: &[FieldValue]) -> Result<FieldValue, TransformError>;
    fn return_type(&self, arg_types: &[FieldType]) -> Result<FieldType, TransformError>;
}

impl NativeTransformEngine {
    /// Execute a transform with native types
    pub fn execute_transform(
        &self,
        transform_spec: &TransformSpec,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        match &transform_spec.transform_type {
            TransformType::Map(map_transform) => {
                self.execute_map_transform(map_transform, input_data)
            }
            TransformType::Filter(filter_transform) => {
                self.execute_filter_transform(filter_transform, input_data)
            }
            TransformType::Reduce(reduce_transform) => {
                self.execute_reduce_transform(reduce_transform, input_data)
            }
            TransformType::Chain(chain) => {
                self.execute_transform_chain(chain, input_data)
            }
        }
    }
    
    /// Execute map transform
    fn execute_map_transform(
        &self,
        map_transform: &MapTransform,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        let mut result = HashMap::new();
        
        for (output_field, mapping) in &map_transform.field_mappings {
            let value = match mapping {
                FieldMapping::Direct(field_name) => {
                    input_data.get(field_name).cloned().unwrap_or(FieldValue::Null)
                }
                FieldMapping::Expression(expr) => {
                    self.evaluate_expression(expr, input_data)?
                }
                FieldMapping::Constant(value) => value.clone(),
                FieldMapping::Function(func_name, args) => {
                    self.execute_function(func_name, args, input_data)?
                }
            };
            result.insert(output_field.clone(), value);
        }
        
        Ok(FieldValue::Object(result))
    }
    
    /// Evaluate expression with native types
    fn evaluate_expression(
        &self,
        expression: &str,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        // Parse expression into AST
        // Evaluate with native types
        // Return typed result
    }
}
```

### 4. Data Processing Pipeline

```rust
// src/transform/pipeline.rs

/// Native data processing pipeline
pub struct NativeDataPipeline {
    engine: Arc<NativeTransformEngine>,
    schema_registry: Arc<NativeSchemaRegistry>,
}

/// Processing context for transforms
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    pub schema_name: String,
    pub input_data: HashMap<String, FieldValue>,
    pub transform_specs: Vec<TransformSpec>,
}

impl NativeDataPipeline {
    /// Process data through transform pipeline
    pub fn process_data(
        &self,
        context: ProcessingContext,
    ) -> Result<HashMap<String, FieldValue>, TransformError> {
        let mut current_data = context.input_data;
        
        for transform_spec in context.transform_specs {
            let result = self.engine.execute_transform(&transform_spec, &current_data)?;
            
            // Update current data for next transform
            if let FieldValue::Object(obj) = result {
                current_data = obj;
            }
        }
        
        Ok(current_data)
    }
    
    /// Process single transform
    pub fn process_single_transform(
        &self,
        transform_spec: &TransformSpec,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        self.engine.execute_transform(transform_spec, input_data)
    }
}
```

### 5. Persistence Layer

```rust
// src/persistence/native_persistence.rs

/// Native data persistence with minimal JSON usage
pub struct NativePersistence {
    db_ops: Arc<DbOperations>,
    schema_registry: Arc<NativeSchemaRegistry>,
}

impl NativePersistence {
    /// Store native data to database
    pub fn store_data(
        &self,
        schema_name: &str,
        data: &HashMap<String, FieldValue>,
    ) -> Result<(), PersistenceError> {
        // Convert native types to database format
        let db_data = self.convert_to_db_format(schema_name, data)?;
        
        // Store using existing database operations
        self.db_ops.store_data(schema_name, &db_data)?;
        
        Ok(())
    }
    
    /// Load data from database as native types
    pub fn load_data(
        &self,
        schema_name: &str,
        key: &str,
    ) -> Result<HashMap<String, FieldValue>, PersistenceError> {
        // Load from database
        let db_data = self.db_ops.load_data(schema_name, key)?;
        
        // Convert to native types
        self.convert_from_db_format(schema_name, &db_data)
    }
    
    /// Convert native types to database format
    fn convert_to_db_format(
        &self,
        schema_name: &str,
        data: &HashMap<String, FieldValue>,
    ) -> Result<serde_json::Value, PersistenceError> {
        // Only convert to JSON for database storage
        // Use serde for this conversion
        Ok(serde_json::to_value(data)?)
    }
    
    /// Convert database format to native types
    fn convert_from_db_format(
        &self,
        schema_name: &str,
        db_data: &serde_json::Value,
    ) -> Result<HashMap<String, FieldValue>, PersistenceError> {
        // Convert from JSON to native types
        // Validate against schema
        self.validate_and_convert(schema_name, db_data)
    }
}
```

### 6. API Boundary Layer

```rust
// src/api/json_boundary.rs

/// JSON conversion layer for API boundaries
pub struct JsonBoundaryLayer {
    schema_registry: Arc<NativeSchemaRegistry>,
}

impl JsonBoundaryLayer {
    /// Convert API request JSON to native types
    pub fn json_to_native(
        &self,
        schema_name: &str,
        json_data: &serde_json::Value,
    ) -> Result<HashMap<String, FieldValue>, ApiError> {
        // Validate JSON against schema
        // Convert to native types
        // Return typed data
    }
    
    /// Convert native types to API response JSON
    pub fn native_to_json(
        &self,
        schema_name: &str,
        native_data: &HashMap<String, FieldValue>,
    ) -> Result<serde_json::Value, ApiError> {
        // Convert native types to JSON
        // Format for API response
        // Return JSON
    }
    
    /// Process API request with native types
    pub fn process_api_request(
        &self,
        schema_name: &str,
        request_json: &serde_json::Value,
        pipeline: &NativeDataPipeline,
    ) -> Result<serde_json::Value, ApiError> {
        // Convert JSON to native
        let native_data = self.json_to_native(schema_name, request_json)?;
        
        // Process with native pipeline
        let context = ProcessingContext {
            schema_name: schema_name.to_string(),
            input_data: native_data,
            transform_specs: self.get_transform_specs(schema_name)?,
        };
        
        let result = pipeline.process_data(context)?;
        
        // Convert result back to JSON
        self.native_to_json(schema_name, &result)
    }
}
```

## Benefits of This Architecture

### 1. **Performance Improvements**
- **No JSON parsing** in hot paths
- **Direct struct operations** instead of serialization
- **Compile-time type checking** instead of runtime validation
- **Memory efficiency** with native types

### 2. **Type Safety**
- **Compile-time errors** for type mismatches
- **Explicit field types** instead of generic `JsonValue`
- **Schema validation** at compile time where possible

### 3. **Maintainability**
- **Clear data structures** instead of JSON manipulation
- **Explicit error handling** with typed errors
- **Easier debugging** with native types

### 4. **Testability**
- **Direct struct creation** in tests
- **No JSON format dependencies** in test data
- **Clearer test assertions** with typed values

## Migration Strategy

### Phase 1: Core Types
1. Implement `FieldValue` and `FieldDefinition` types
2. Create `NativeSchema` and `NativeSchemaRegistry`
3. Add basic type conversion utilities

### Phase 2: Transform Engine
1. Implement `NativeTransformEngine`
2. Create `FunctionRegistry` with basic functions
3. Add expression evaluation with native types

### Phase 3: Pipeline Integration
1. Implement `NativeDataPipeline`
2. Integrate with existing database operations
3. Add persistence layer with minimal JSON usage

### Phase 4: API Integration
1. Implement `JsonBoundaryLayer`
2. Update HTTP routes to use native types
3. Maintain JSON compatibility at API boundaries

### Phase 5: Testing & Validation
1. Add comprehensive tests for native types
2. Validate performance improvements
3. Ensure backward compatibility

## Implementation Priority

1. **High Priority**: Core data types and schema registry
2. **Medium Priority**: Transform engine and pipeline
3. **Low Priority**: API boundary layer and persistence

This architecture eliminates JSON passing while maintaining all functionality and providing significant performance and maintainability improvements.
