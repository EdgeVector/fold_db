# Native Transform Implementation Guide

## Getting Started: Step-by-Step Implementation

This guide provides concrete steps to implement the native transform system, starting with the most critical components.

## Phase 1: Core Foundation (Week 1)

### Step 1.1: Create Core Types Module

```bash
# Create the new module structure
mkdir -p src/transform/native
touch src/transform/native/mod.rs
touch src/transform/native/types.rs
touch src/transform/native/field_definition.rs
touch src/transform/native/transform_spec.rs
```

### Step 1.2: Implement FieldValue Type

```rust
// src/transform/native/types.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Native representation of a field value - replaces JsonValue
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Object(HashMap<String, FieldValue>),
    Null,
}

impl FieldValue {
    /// Get the type of this field value
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::String(_) => FieldType::String,
            FieldValue::Number(_) => FieldType::Number,
            FieldValue::Integer(_) => FieldType::Integer,
            FieldValue::Boolean(_) => FieldType::Boolean,
            FieldValue::Array(_) => FieldType::Array(Box::new(FieldType::String)), // TODO: infer from elements
            FieldValue::Object(_) => FieldType::Object(HashMap::new()), // TODO: infer from fields
            FieldValue::Null => FieldType::String, // Default type for null
        }
    }
    
    /// Convert to JSON value (only for persistence/API boundaries)
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            FieldValue::String(s) => serde_json::Value::String(s.clone()),
            FieldValue::Number(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))
            ),
            FieldValue::Integer(i) => serde_json::Value::Number(
                serde_json::Number::from(*i)
            ),
            FieldValue::Boolean(b) => serde_json::Value::Bool(*b),
            FieldValue::Array(arr) => serde_json::Value::Array(
                arr.iter().map(|v| v.to_json_value()).collect()
            ),
            FieldValue::Object(obj) => serde_json::Value::Object(
                obj.iter().map(|(k, v)| (k.clone(), v.to_json_value())).collect()
            ),
            FieldValue::Null => serde_json::Value::Null,
        }
    }
    
    /// Create from JSON value (only for persistence/API boundaries)
    pub fn from_json_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) => FieldValue::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    FieldValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    FieldValue::Number(f)
                } else {
                    FieldValue::Number(0.0)
                }
            }
            serde_json::Value::Bool(b) => FieldValue::Boolean(b),
            serde_json::Value::Array(arr) => FieldValue::Array(
                arr.into_iter().map(FieldValue::from_json_value).collect()
            ),
            serde_json::Value::Object(obj) => FieldValue::Object(
                obj.into_iter().map(|(k, v)| (k, FieldValue::from_json_value(v))).collect()
            ),
            serde_json::Value::Null => FieldValue::Null,
        }
    }
}

/// Supported field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Number,
    Integer,
    Boolean,
    Array(Box<FieldType>),
    Object(HashMap<String, FieldType>),
}

impl FieldType {
    /// Check if a field value matches this type
    pub fn matches(&self, value: &FieldValue) -> bool {
        match (self, value) {
            (FieldType::String, FieldValue::String(_)) => true,
            (FieldType::Number, FieldValue::Number(_)) => true,
            (FieldType::Integer, FieldValue::Integer(_)) => true,
            (FieldType::Boolean, FieldValue::Boolean(_)) => true,
            (FieldType::Array(element_type), FieldValue::Array(arr)) => {
                arr.iter().all(|v| element_type.matches(v))
            }
            (FieldType::Object(field_types), FieldValue::Object(obj)) => {
                field_types.iter().all(|(field_name, field_type)| {
                    obj.get(field_name).map_or(false, |v| field_type.matches(v))
                })
            }
            _ => false,
        }
    }
}
```

### Step 1.3: Create Basic Tests

```rust
// tests/unit/native_types_tests.rs

use datafold::transform::native::types::{FieldValue, FieldType};

#[test]
fn test_field_value_creation() {
    let string_val = FieldValue::String("hello".to_string());
    let number_val = FieldValue::Number(42.5);
    let integer_val = FieldValue::Integer(42);
    let boolean_val = FieldValue::Boolean(true);
    
    assert_eq!(string_val.field_type(), FieldType::String);
    assert_eq!(number_val.field_type(), FieldType::Number);
    assert_eq!(integer_val.field_type(), FieldType::Integer);
    assert_eq!(boolean_val.field_type(), FieldType::Boolean);
}

#[test]
fn test_json_conversion() {
    let original = FieldValue::String("test".to_string());
    let json = original.to_json_value();
    let converted = FieldValue::from_json_value(json);
    
    assert_eq!(original, converted);
}

#[test]
fn test_type_matching() {
    let string_type = FieldType::String;
    let string_value = FieldValue::String("hello".to_string());
    let number_value = FieldValue::Number(42.0);
    
    assert!(string_type.matches(&string_value));
    assert!(!string_type.matches(&number_value));
}
```

## Phase 2: Schema Integration (Week 2)

### Step 2.1: Create Native Schema Module

```bash
mkdir -p src/schema/native
touch src/schema/native/mod.rs
touch src/schema/native/schema.rs
touch src/schema/native/registry.rs
```

### Step 2.2: Implement Native Schema

```rust
// src/schema/native/schema.rs

use crate::transform::native::types::{FieldValue, FieldType};
use crate::transform::native::field_definition::FieldDefinition;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Native schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub key_config: KeyConfig,
}

/// Key configuration for different schema types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyConfig {
    Single { key_field: String },
    Range { hash_field: String, range_field: String },
    HashRange { hash_field: String, range_field: String },
}

impl NativeSchema {
    /// Create a new native schema
    pub fn new(name: String, key_config: KeyConfig) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            key_config,
        }
    }
    
    /// Add a field to the schema
    pub fn add_field(&mut self, field: FieldDefinition) {
        self.fields.insert(field.name.clone(), field);
    }
    
    /// Get field definition by name
    pub fn get_field(&self, field_name: &str) -> Option<&FieldDefinition> {
        self.fields.get(field_name)
    }
    
    /// Validate data against this schema
    pub fn validate_data(&self, data: &HashMap<String, FieldValue>) -> Result<(), SchemaValidationError> {
        // Check required fields
        for (field_name, field_def) in &self.fields {
            if field_def.required {
                if let Some(value) = data.get(field_name) {
                    field_def.validate(value)?;
                } else {
                    return Err(SchemaValidationError::RequiredFieldMissing {
                        field_name: field_name.clone(),
                    });
                }
            }
        }
        
        // Validate field types
        for (field_name, value) in data {
            if let Some(field_def) = self.fields.get(field_name) {
                field_def.validate(value)?;
            }
        }
        
        Ok(())
    }
}

/// Schema validation errors
#[derive(Debug, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("Required field '{field_name}' is missing")]
    RequiredFieldMissing { field_name: String },
    
    #[error("Field validation error: {0}")]
    FieldValidation(#[from] crate::transform::native::field_definition::FieldValidationError),
}
```

## Phase 3: Transform Engine (Week 3)

### Step 3.1: Create Transform Engine Module

```bash
mkdir -p src/transform/engine
touch src/transform/engine/mod.rs
touch src/transform/engine/executor.rs
touch src/transform/engine/functions.rs
```

### Step 3.2: Implement Basic Transform Executor

```rust
// src/transform/engine/executor.rs

use crate::transform::native::types::{FieldValue, FieldType};
use crate::transform::native::transform_spec::{TransformSpec, TransformType, FieldMapping};
use std::collections::HashMap;
use std::sync::Arc;

/// Native transform execution engine
pub struct NativeTransformExecutor {
    function_registry: Arc<FunctionRegistry>,
}

impl NativeTransformExecutor {
    /// Create a new transform executor
    pub fn new() -> Self {
        Self {
            function_registry: Arc::new(FunctionRegistry::new()),
        }
    }
    
    /// Execute a transform with native types
    pub async fn execute_transform(
        &self,
        transform_spec: &TransformSpec,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        match &transform_spec.transform_type {
            TransformType::Map(map_transform) => {
                self.execute_map_transform(map_transform, input_data).await
            }
            TransformType::Filter(filter_transform) => {
                self.execute_filter_transform(filter_transform, input_data).await
            }
            TransformType::Reduce(reduce_transform) => {
                self.execute_reduce_transform(reduce_transform, input_data).await
            }
            TransformType::Chain(chain) => {
                self.execute_transform_chain(chain, input_data).await
            }
        }
    }
    
    /// Execute map transform
    async fn execute_map_transform(
        &self,
        map_transform: &crate::transform::native::transform_spec::MapTransform,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        let mut result = HashMap::new();
        
        for (output_field, mapping) in &map_transform.field_mappings {
            let value = match mapping {
                FieldMapping::Direct(field_name) => {
                    input_data.get(field_name).cloned().unwrap_or(FieldValue::Null)
                }
                FieldMapping::Expression(expr) => {
                    self.evaluate_expression(expr, input_data).await?
                }
                FieldMapping::Constant(value) => value.clone(),
                FieldMapping::Function(func_name, args) => {
                    self.execute_function(func_name, args, input_data).await?
                }
            };
            result.insert(output_field.clone(), value);
        }
        
        Ok(FieldValue::Object(result))
    }
    
    /// Execute filter transform
    async fn execute_filter_transform(
        &self,
        filter_transform: &crate::transform::native::transform_spec::FilterTransform,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        let passes_filter = self.evaluate_filter_condition(&filter_transform.condition, input_data).await?;
        
        if passes_filter {
            Ok(FieldValue::Object(input_data.clone()))
        } else {
            Ok(FieldValue::Null)
        }
    }
    
    /// Execute reduce transform
    async fn execute_reduce_transform(
        &self,
        reduce_transform: &crate::transform::native::transform_spec::ReduceTransform,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        // Implementation for reduce transforms
        // This would handle grouping and aggregation
        todo!("Implement reduce transform execution")
    }
    
    /// Execute transform chain
    async fn execute_transform_chain(
        &self,
        chain: &[TransformSpec],
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        let mut current_data = input_data.clone();
        
        for transform_spec in chain {
            let result = self.execute_transform(transform_spec, &current_data).await?;
            
            if let FieldValue::Object(obj) = result {
                current_data = obj;
            } else {
                return Err(TransformError::ExecutionError {
                    message: "Chain transform must return an object".to_string(),
                });
            }
        }
        
        Ok(FieldValue::Object(current_data))
    }
    
    /// Evaluate expression with native types
    async fn evaluate_expression(
        &self,
        expression: &str,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        // Simple expression evaluation for now
        if expression.starts_with("${") && expression.ends_with("}") {
            let field_name = &expression[2..expression.len()-1];
            Ok(input_data.get(field_name).cloned().unwrap_or(FieldValue::Null))
        } else {
            // Try to parse as a constant
            self.parse_constant(expression)
        }
    }
    
    /// Execute function with arguments
    async fn execute_function(
        &self,
        func_name: &str,
        args: &[String],
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, TransformError> {
        let mut resolved_args = Vec::new();
        
        for arg in args {
            if arg.starts_with("${") && arg.ends_with("}") {
                let field_name = &arg[2..arg.len()-1];
                resolved_args.push(input_data.get(field_name).cloned().unwrap_or(FieldValue::Null));
            } else {
                resolved_args.push(self.parse_constant(arg)?);
            }
        }
        
        self.function_registry.execute_function(func_name, &resolved_args)
    }
    
    /// Parse constant value
    fn parse_constant(&self, value: &str) -> Result<FieldValue, TransformError> {
        if value == "true" {
            Ok(FieldValue::Boolean(true))
        } else if value == "false" {
            Ok(FieldValue::Boolean(false))
        } else if value == "null" {
            Ok(FieldValue::Null)
        } else if let Ok(i) = value.parse::<i64>() {
            Ok(FieldValue::Integer(i))
        } else if let Ok(f) = value.parse::<f64>() {
            Ok(FieldValue::Number(f))
        } else if value.starts_with('"') && value.ends_with('"') {
            Ok(FieldValue::String(value[1..value.len()-1].to_string()))
        } else {
            Ok(FieldValue::String(value.to_string()))
        }
    }
    
    /// Evaluate filter condition
    async fn evaluate_filter_condition(
        &self,
        condition: &crate::transform::native::transform_spec::FilterCondition,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<bool, TransformError> {
        match condition {
            crate::transform::native::transform_spec::FilterCondition::Equals(field, value) => {
                Ok(input_data.get(field) == Some(value))
            }
            crate::transform::native::transform_spec::FilterCondition::NotEquals(field, value) => {
                Ok(input_data.get(field) != Some(value))
            }
            crate::transform::native::transform_spec::FilterCondition::GreaterThan(field, value) => {
                if let Some(field_value) = input_data.get(field) {
                    self.compare_values(field_value, value, |a, b| a > b)
                } else {
                    Ok(false)
                }
            }
            crate::transform::native::transform_spec::FilterCondition::LessThan(field, value) => {
                if let Some(field_value) = input_data.get(field) {
                    self.compare_values(field_value, value, |a, b| a < b)
                } else {
                    Ok(false)
                }
            }
            crate::transform::native::transform_spec::FilterCondition::Contains(field, value) => {
                if let Some(field_value) = input_data.get(field) {
                    match field_value {
                        FieldValue::String(s) => {
                            if let FieldValue::String(v) = value {
                                Ok(s.contains(v))
                            } else {
                                Ok(false)
                            }
                        }
                        FieldValue::Array(arr) => {
                            Ok(arr.contains(value))
                        }
                        _ => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            crate::transform::native::transform_spec::FilterCondition::And(conditions) => {
                for condition in conditions {
                    if !self.evaluate_filter_condition(condition, input_data).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::transform::native::transform_spec::FilterCondition::Or(conditions) => {
                for condition in conditions {
                    if self.evaluate_filter_condition(condition, input_data).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
    
    /// Compare two field values
    fn compare_values<F>(&self, a: &FieldValue, b: &FieldValue, compare_fn: F) -> Result<bool, TransformError>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        match (a, b) {
            (FieldValue::Number(a_num), FieldValue::Number(b_num)) => {
                Ok(compare_fn(*a_num, *b_num))
            }
            (FieldValue::Integer(a_int), FieldValue::Integer(b_int)) => {
                Ok(compare_fn(*a_int as f64, *b_int as f64))
            }
            (FieldValue::Number(a_num), FieldValue::Integer(b_int)) => {
                Ok(compare_fn(*a_num, *b_int as f64))
            }
            (FieldValue::Integer(a_int), FieldValue::Number(b_num)) => {
                Ok(compare_fn(*a_int as f64, *b_num))
            }
            _ => Err(TransformError::ExecutionError {
                message: "Cannot compare non-numeric values".to_string(),
            }),
        }
    }
}

/// Transform errors
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Function '{name}' not found")]
    FunctionNotFound { name: String },
    
    #[error("Invalid argument for function '{function}': expected {expected}, got {actual}")]
    InvalidArgument { function: String, expected: String, actual: String },
    
    #[error("Execution error: {message}")]
    ExecutionError { message: String },
}
```

## Phase 4: Integration Testing (Week 4)

### Step 4.1: Create Integration Tests

```rust
// tests/integration/native_transform_integration_test.rs

use datafold::transform::native::types::{FieldValue, FieldType};
use datafold::transform::native::field_definition::FieldDefinition;
use datafold::transform::native::transform_spec::{TransformSpec, TransformType, FieldMapping};
use datafold::transform::engine::executor::NativeTransformExecutor;
use std::collections::HashMap;

#[tokio::test]
async fn test_simple_map_transform() {
    let executor = NativeTransformExecutor::new();
    
    // Create input data
    let mut input_data = HashMap::new();
    input_data.insert("name".to_string(), FieldValue::String("John".to_string()));
    input_data.insert("age".to_string(), FieldValue::Integer(30));
    
    // Create map transform
    let mut field_mappings = HashMap::new();
    field_mappings.insert("full_name".to_string(), FieldMapping::Direct("name".to_string()));
    field_mappings.insert("years_old".to_string(), FieldMapping::Direct("age".to_string()));
    
    let map_transform = crate::transform::native::transform_spec::MapTransform {
        field_mappings,
    };
    
    let transform_spec = TransformSpec {
        name: "test_map".to_string(),
        inputs: vec![],
        output: FieldDefinition::new("output".to_string(), FieldType::Object(HashMap::new()), false),
        transform_type: TransformType::Map(Box::new(map_transform)),
    };
    
    // Execute transform
    let result = executor.execute_transform(&transform_spec, &input_data).await.unwrap();
    
    // Verify result
    if let FieldValue::Object(obj) = result {
        assert_eq!(obj.get("full_name"), Some(&FieldValue::String("John".to_string())));
        assert_eq!(obj.get("years_old"), Some(&FieldValue::Integer(30)));
    } else {
        panic!("Expected object result");
    }
}

#[tokio::test]
async fn test_filter_transform() {
    let executor = NativeTransformExecutor::new();
    
    // Create input data
    let mut input_data = HashMap::new();
    input_data.insert("age".to_string(), FieldValue::Integer(25));
    input_data.insert("name".to_string(), FieldValue::String("Alice".to_string()));
    
    // Create filter transform
    let filter_transform = crate::transform::native::transform_spec::FilterTransform {
        condition: crate::transform::native::transform_spec::FilterCondition::GreaterThan(
            "age".to_string(),
            FieldValue::Integer(20)
        ),
    };
    
    let transform_spec = TransformSpec {
        name: "test_filter".to_string(),
        inputs: vec![],
        output: FieldDefinition::new("output".to_string(), FieldType::Object(HashMap::new()), false),
        transform_type: TransformType::Filter(Box::new(filter_transform)),
    };
    
    // Execute transform
    let result = executor.execute_transform(&transform_spec, &input_data).await.unwrap();
    
    // Verify result (should pass filter)
    if let FieldValue::Object(obj) = result {
        assert_eq!(obj.get("age"), Some(&FieldValue::Integer(25)));
        assert_eq!(obj.get("name"), Some(&FieldValue::String("Alice".to_string())));
    } else {
        panic!("Expected object result");
    }
}
```

## Phase 5: Migration Strategy

### Step 5.1: Gradual Migration Plan

1. **Week 1**: Implement core types and basic tests
2. **Week 2**: Add schema integration and validation
3. **Week 3**: Build transform execution engine
4. **Week 4**: Create comprehensive integration tests
5. **Week 5**: Add API boundary layer
6. **Week 6**: Implement persistence layer
7. **Week 7**: Performance testing and optimization
8. **Week 8**: Full migration and cleanup

### Step 5.2: Backward Compatibility

- Keep existing JSON-based APIs working
- Add new native APIs alongside existing ones
- Gradually migrate internal components
- Remove JSON dependencies last

### Step 5.3: Performance Validation

- Benchmark native types vs JSON
- Measure memory usage improvements
- Test execution speed improvements
- Validate type safety benefits

This implementation provides a solid foundation for a JSON-free transform system with native Rust types, type safety, and improved performance.
