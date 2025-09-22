# Native Transform Implementation Plan

## Phase 1: Core Data Types Implementation

### 1.1 Field Value Types

```rust
// src/transform/native_types.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Native representation of a field value
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
    
    /// Convert to JSON value (only for persistence/API)
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
    
    /// Create from JSON value (only for persistence/API)
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

### 1.2 Field Definitions

```rust
// src/transform/field_definition.rs

use crate::transform::native_types::{FieldValue, FieldType};
use serde::{Deserialize, Serialize};

/// Schema field definition with native types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default_value: Option<FieldValue>,
}

impl FieldDefinition {
    /// Create a new field definition
    pub fn new(name: String, field_type: FieldType, required: bool) -> Self {
        Self {
            name,
            field_type,
            required,
            default_value: None,
        }
    }
    
    /// Create with default value
    pub fn with_default(name: String, field_type: FieldType, required: bool, default_value: FieldValue) -> Self {
        Self {
            name,
            field_type,
            required,
            default_value: Some(default_value),
        }
    }
    
    /// Validate a field value against this definition
    pub fn validate(&self, value: &FieldValue) -> Result<(), FieldValidationError> {
        if !self.field_type.matches(value) {
            return Err(FieldValidationError::TypeMismatch {
                field_name: self.name.clone(),
                expected: format!("{:?}", self.field_type),
                actual: format!("{:?}", value.field_type()),
            });
        }
        
        if self.required && matches!(value, FieldValue::Null) {
            return Err(FieldValidationError::RequiredFieldMissing {
                field_name: self.name.clone(),
            });
        }
        
        Ok(())
    }
    
    /// Get default value or create appropriate default
    pub fn get_default_value(&self) -> FieldValue {
        self.default_value.clone().unwrap_or_else(|| {
            match &self.field_type {
                FieldType::String => FieldValue::String(String::new()),
                FieldType::Number => FieldValue::Number(0.0),
                FieldType::Integer => FieldValue::Integer(0),
                FieldType::Boolean => FieldValue::Boolean(false),
                FieldType::Array(_) => FieldValue::Array(Vec::new()),
                FieldType::Object(_) => FieldValue::Object(HashMap::new()),
            }
        })
    }
}

/// Field validation errors
#[derive(Debug, thiserror::Error)]
pub enum FieldValidationError {
    #[error("Type mismatch for field '{field_name}': expected {expected}, got {actual}")]
    TypeMismatch { field_name: String, expected: String, actual: String },
    
    #[error("Required field '{field_name}' is missing")]
    RequiredFieldMissing { field_name: String },
}
```

### 1.3 Transform Specifications

```rust
// src/transform/transform_spec.rs

use crate::transform::field_definition::FieldDefinition;
use crate::transform::native_types::FieldValue;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Transform input/output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformSpec {
    pub name: String,
    pub inputs: Vec<FieldDefinition>,
    pub output: FieldDefinition,
    pub transform_type: TransformType,
}

/// Types of transforms supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformType {
    Map(Box<MapTransform>),
    Filter(Box<FilterTransform>),
    Reduce(Box<ReduceTransform>),
    Chain(Vec<TransformSpec>),
}

/// Map transform definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapTransform {
    pub field_mappings: HashMap<String, FieldMapping>,
}

/// Filter transform definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterTransform {
    pub condition: FilterCondition,
}

/// Reduce transform definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReduceTransform {
    pub reducer: ReducerType,
    pub group_by: Vec<String>,
}

/// Field mapping specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldMapping {
    Direct(String), // Direct field copy
    Expression(String), // Expression to evaluate
    Constant(FieldValue), // Constant value
    Function(String, Vec<String>), // Function call with args
}

/// Filter condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterCondition {
    Equals(String, FieldValue),
    NotEquals(String, FieldValue),
    GreaterThan(String, FieldValue),
    LessThan(String, FieldValue),
    Contains(String, FieldValue),
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
}

/// Reducer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReducerType {
    Sum(String),
    Count,
    Average(String),
    Min(String),
    Max(String),
    First(String),
    Last(String),
}
```

## Phase 2: Schema Registry Implementation

### 2.1 Native Schema Types

```rust
// src/schema/native_schema.rs

use crate::transform::field_definition::FieldDefinition;
use crate::transform::transform_spec::TransformSpec;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Native schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub key_config: KeyConfig,
    pub transform_specs: Vec<TransformSpec>,
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
            transform_specs: Vec::new(),
        }
    }
    
    /// Add a field to the schema
    pub fn add_field(&mut self, field: FieldDefinition) {
        self.fields.insert(field.name.clone(), field);
    }
    
    /// Add a transform specification
    pub fn add_transform_spec(&mut self, transform_spec: TransformSpec) {
        self.transform_specs.push(transform_spec);
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
    FieldValidation(#[from] crate::transform::field_definition::FieldValidationError),
}
```

### 2.2 Schema Registry

```rust
// src/schema/native_registry.rs

use crate::schema::native_schema::NativeSchema;
use crate::transform::field_definition::FieldDefinition;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Schema registry with native types
pub struct NativeSchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, NativeSchema>>>,
}

impl NativeSchemaRegistry {
    /// Create a new schema registry
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a schema
    pub async fn register_schema(&self, schema: NativeSchema) -> Result<(), RegistryError> {
        let mut schemas = self.schemas.write().await;
        schemas.insert(schema.name.clone(), schema);
        Ok(())
    }
    
    /// Get a schema by name
    pub async fn get_schema(&self, name: &str) -> Option<NativeSchema> {
        let schemas = self.schemas.read().await;
        schemas.get(name).cloned()
    }
    
    /// Get field definition by schema and field name
    pub async fn get_field(&self, schema_name: &str, field_name: &str) -> Option<FieldDefinition> {
        let schemas = self.schemas.read().await;
        schemas.get(schema_name)?.get_field(field_name).cloned()
    }
    
    /// List all schema names
    pub async fn list_schemas(&self) -> Vec<String> {
        let schemas = self.schemas.read().await;
        schemas.keys().cloned().collect()
    }
}

/// Registry errors
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Schema '{name}' already exists")]
    SchemaExists { name: String },
    
    #[error("Schema '{name}' not found")]
    SchemaNotFound { name: String },
}
```

## Phase 3: Transform Execution Engine

### 3.1 Function Registry

```rust
// src/transform/function_registry.rs

use crate::transform::native_types::{FieldValue, FieldType};
use std::collections::HashMap;
use std::sync::Arc;

/// Function registry for transform operations
pub struct FunctionRegistry {
    functions: HashMap<String, Arc<dyn TransformFunction>>,
}

/// Trait for transform functions
pub trait TransformFunction: Send + Sync {
    fn execute(&self, args: &[FieldValue]) -> Result<FieldValue, TransformError>;
    fn return_type(&self, arg_types: &[FieldType]) -> Result<FieldType, TransformError>;
    fn name(&self) -> &str;
}

impl FunctionRegistry {
    /// Create a new function registry
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        
        // Register built-in functions
        registry.register_builtin_functions();
        registry
    }
    
    /// Register a function
    pub fn register_function(&mut self, name: String, function: Arc<dyn TransformFunction>) {
        self.functions.insert(name, function);
    }
    
    /// Execute a function by name
    pub fn execute_function(
        &self,
        name: &str,
        args: &[FieldValue],
    ) -> Result<FieldValue, TransformError> {
        let function = self.functions.get(name)
            .ok_or_else(|| TransformError::FunctionNotFound { name: name.to_string() })?;
        
        function.execute(args)
    }
    
    /// Register built-in functions
    fn register_builtin_functions(&mut self) {
        // String functions
        self.register_function("concat".to_string(), Arc::new(ConcatFunction));
        self.register_function("upper".to_string(), Arc::new(UpperFunction));
        self.register_function("lower".to_string(), Arc::new(LowerFunction));
        
        // Math functions
        self.register_function("add".to_string(), Arc::new(AddFunction));
        self.register_function("subtract".to_string(), Arc::new(SubtractFunction));
        self.register_function("multiply".to_string(), Arc::new(MultiplyFunction));
        self.register_function("divide".to_string(), Arc::new(DivideFunction));
        
        // Date functions
        self.register_function("now".to_string(), Arc::new(NowFunction));
        self.register_function("date_format".to_string(), Arc::new(DateFormatFunction));
    }
}

/// Built-in string concatenation function
struct ConcatFunction;

impl TransformFunction for ConcatFunction {
    fn execute(&self, args: &[FieldValue]) -> Result<FieldValue, TransformError> {
        let mut result = String::new();
        for arg in args {
            match arg {
                FieldValue::String(s) => result.push_str(s),
                FieldValue::Number(n) => result.push_str(&n.to_string()),
                FieldValue::Integer(i) => result.push_str(&i.to_string()),
                FieldValue::Boolean(b) => result.push_str(&b.to_string()),
                _ => return Err(TransformError::InvalidArgument {
                    function: "concat".to_string(),
                    expected: "string, number, integer, or boolean".to_string(),
                    actual: format!("{:?}", arg.field_type()),
                }),
            }
        }
        Ok(FieldValue::String(result))
    }
    
    fn return_type(&self, _arg_types: &[FieldType]) -> Result<FieldType, TransformError> {
        Ok(FieldType::String)
    }
    
    fn name(&self) -> &str {
        "concat"
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

### 3.2 Transform Execution Engine

```rust
// src/transform/execution_engine.rs

use crate::transform::function_registry::{FunctionRegistry, TransformFunction};
use crate::transform::transform_spec::{TransformSpec, TransformType, FieldMapping};
use crate::transform::native_types::{FieldValue, FieldType};
use crate::schema::native_registry::NativeSchemaRegistry;
use std::collections::HashMap;
use std::sync::Arc;

/// Native transform execution engine
pub struct NativeTransformEngine {
    schema_registry: Arc<NativeSchemaRegistry>,
    function_registry: FunctionRegistry,
}

impl NativeTransformEngine {
    /// Create a new transform engine
    pub fn new(schema_registry: Arc<NativeSchemaRegistry>) -> Self {
        Self {
            schema_registry,
            function_registry: FunctionRegistry::new(),
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
        map_transform: &crate::transform::transform_spec::MapTransform,
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
        filter_transform: &crate::transform::transform_spec::FilterTransform,
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
        reduce_transform: &crate::transform::transform_spec::ReduceTransform,
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
        // This could be expanded to support more complex expressions
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
        // Try to parse as different types
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
        condition: &crate::transform::transform_spec::FilterCondition,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<bool, TransformError> {
        match condition {
            crate::transform::transform_spec::FilterCondition::Equals(field, value) => {
                Ok(input_data.get(field) == Some(value))
            }
            crate::transform::transform_spec::FilterCondition::NotEquals(field, value) => {
                Ok(input_data.get(field) != Some(value))
            }
            crate::transform::transform_spec::FilterCondition::GreaterThan(field, value) => {
                if let Some(field_value) = input_data.get(field) {
                    self.compare_values(field_value, value, |a, b| a > b)
                } else {
                    Ok(false)
                }
            }
            crate::transform::transform_spec::FilterCondition::LessThan(field, value) => {
                if let Some(field_value) = input_data.get(field) {
                    self.compare_values(field_value, value, |a, b| a < b)
                } else {
                    Ok(false)
                }
            }
            crate::transform::transform_spec::FilterCondition::Contains(field, value) => {
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
            crate::transform::transform_spec::FilterCondition::And(conditions) => {
                for condition in conditions {
                    if !self.evaluate_filter_condition(condition, input_data).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::transform::transform_spec::FilterCondition::Or(conditions) => {
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
```

This implementation provides:

1. **Native Rust types** instead of JSON
2. **Type-safe field definitions** with validation
3. **Function registry** for extensible transform operations
4. **Clean execution engine** with native type operations
5. **Comprehensive error handling** with typed errors

The next phases would implement the data pipeline, persistence layer, and API boundary layer. Would you like me to continue with those implementations?
