# Native Transform System API Reference

This document provides comprehensive API documentation for the Native Transform System (NTS-3), including all public interfaces, types, and methods.

## Core Types

### FieldValue

The `FieldValue` enum represents all possible values that can flow through the transform system.

```rust
pub enum FieldValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Object(HashMap<String, FieldValue>),
    Null,
}
```

**Key Methods:**

- `field_type(&self) -> FieldType` - Returns the field type of this value
- `to_json_value(&self) -> JsonValue` - Converts to JSON for boundary operations
- `from_json_value(value: JsonValue) -> Self` - Creates from JSON value

### FieldType

Represents declarative type information for schema fields.

```rust
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Null,
    Array { element_type: Box<FieldType> },
    Object { fields: HashMap<String, FieldType> },
}
```

**Key Methods:**

- `matches(&self, value: &FieldValue) -> bool` - Checks if value satisfies type
- `default_value(&self) -> FieldValue` - Returns deterministic default value

### FieldDefinition

Pairs metadata with validation logic for transform fields.

```rust
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default_value: Option<FieldValue>,
}
```

**Key Methods:**

- `new(name: impl Into<String>, field_type: FieldType) -> Self` - Constructor
- `with_required(self, required: bool) -> Self` - Sets required flag
- `with_default(self, default_value: FieldValue) -> Self` - Sets default value
- `validate(&self) -> Result<(), FieldDefinitionError>` - Validates definition
- `effective_default(&self) -> Option<FieldValue>` - Gets effective default

## Transform Specifications

### TransformSpec

Native transform specification describing inputs, output, and execution behavior.

```rust
pub struct TransformSpec {
    pub name: String,
    pub inputs: Vec<FieldDefinition>,
    pub output: FieldDefinition,
    pub transform_type: TransformType,
}
```

**Key Methods:**

- `new(name, inputs, output, transform_type) -> Self` - Constructor
- `validate(&self) -> Result<(), TransformSpecError>` - Validates specification

### TransformType

Supported transform behaviors.

```rust
pub enum TransformType {
    Map(MapTransform),
    Filter(FilterTransform),
    Reduce(ReduceTransform),
    Chain(Vec<TransformSpec>),
}
```

### MapTransform

Mapping transform metadata.

```rust
pub struct MapTransform {
    pub field_mappings: HashMap<String, FieldMapping>,
}
```

**Key Methods:**

- `new(field_mappings: HashMap<String, FieldMapping>) -> Self` - Constructor

### FieldMapping

Field mapping definitions for map transforms.

```rust
pub enum FieldMapping {
    Direct { field: String },
    Expression { expression: String },
    Constant { value: FieldValue },
    Function { name: String, arguments: Vec<String> },
}
```

### FilterTransform

Filter transform metadata.

```rust
pub struct FilterTransform {
    pub condition: FilterCondition,
}
```

**Key Methods:**

- `new(condition: FilterCondition) -> Self` - Constructor

### FilterCondition

Supported filter conditions.

```rust
pub enum FilterCondition {
    Equals { field: String, value: FieldValue },
    NotEquals { field: String, value: FieldValue },
    GreaterThan { field: String, value: FieldValue },
    LessThan { field: String, value: FieldValue },
    Contains { field: String, value: FieldValue },
    And { conditions: Vec<FilterCondition> },
    Or { conditions: Vec<FilterCondition> },
}
```

### ReduceTransform

Reduce transform metadata.

```rust
pub struct ReduceTransform {
    pub reducer: ReducerType,
    pub group_by: Vec<String>,
}
```

**Key Methods:**

- `new(reducer: ReducerType, group_by: Vec<String>) -> Self` - Constructor

### ReducerType

Supported reducer types for aggregate transforms.

```rust
pub enum ReducerType {
    Sum { field: String },
    Count,
    Average { field: String },
    Min { field: String },
    Max { field: String },
    First { field: String },
    Last { field: String },
}
```

## Function Registry

### FunctionSignature

Function signature defining parameter types and return type.

```rust
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<(String, FieldType)>,
    pub return_type: FieldType,
    pub is_async: bool,
    pub description: String,
}
```

### FunctionRegistry

Function registry that manages built-in and custom functions.

```rust
pub struct FunctionRegistry {
    // Internal function storage
}
```

**Key Methods:**

- `new() -> Self` - Creates empty registry
- `with_built_ins() -> Self` - Creates registry with built-in functions
- `register(&mut self, signature, implementation) -> Result<(), FunctionRegistryError>` - Registers function
- `register_custom(&mut self, signature, implementation) -> Result<(), FunctionRegistryError>` - Registers custom function
- `get_function(&self, name: &str) -> Result<&Function, FunctionRegistryError>` - Gets function by name
- `has_function(&self, name: &str) -> bool` - Checks if function exists
- `list_functions(&self) -> Vec<String>` - Lists all function names
- `get_signature(&self, name: &str) -> Result<&FunctionSignature, FunctionRegistryError>` - Gets function signature
- `execute_function(&self, name: &str, args: Vec<FieldValue>) -> Result<FieldValue, FunctionRegistryError>` - Executes function

## Expression Evaluator

### ExpressionEvaluator

Expression evaluator for native FieldValue types.

```rust
pub struct ExpressionEvaluator<'a> {
    function_registry: &'a FunctionRegistry,
    context: &'a HashMap<String, FieldValue>,
}
```

**Key Methods:**

- `new(function_registry, context) -> Self` - Creates evaluator
- `evaluate_expression(&self, expression: &str) -> Result<FieldValue, ExpressionEvaluationError>` - Evaluates expression string
- `evaluate_ast(&self, expr: Expression) -> Result<FieldValue, ExpressionEvaluationError>` - Evaluates AST expression

## Executor

### NativeTransformExecutor

Core execution engine for native transforms.

```rust
pub struct NativeTransformExecutor {
    schema_registry: Arc<NativeSchemaRegistry>,
    function_registry: Arc<FunctionRegistry>,
}
```

**Key Methods:**

- `new() -> Self` - Creates executor with built-in functions
- `new_with_functions(schema_registry, function_registry) -> Self` - Creates executor with custom functions
- `execute_transform(&self, spec: &TransformSpec, input: NativeTransformInput) -> Result<NativeTransformResult, TransformExecutionError>` - Executes transform
- `schema_registry(&self) -> &NativeSchemaRegistry` - Gets schema registry

### NativeTransformInput

Input structure for transform execution.

```rust
pub struct NativeTransformInput {
    pub values: HashMap<String, FieldValue>,
    pub schema_name: Option<String>,
}
```

### NativeTransformResult

Result structure from transform execution.

```rust
pub struct NativeTransformResult {
    pub values: HashMap<String, FieldValue>,
    pub metadata: ExecutionMetadata,
}
```

### ExecutionMetadata

Metadata about transform execution.

```rust
pub struct ExecutionMetadata {
    pub success: bool,
    pub transform_type: String,
    pub execution_time_ns: u64,
    pub fields_processed: usize,
    pub error_message: Option<String>,
}
```

## Error Types

### TransformSpecError

Validation errors from transform specifications.

```rust
pub enum TransformSpecError {
    EmptyName,
    DuplicateInputField { field: String },
    InputValidation { field: String, source: FieldDefinitionError },
    OutputValidation { field: String, source: FieldDefinitionError },
    EmptyFieldMappings,
    InvalidOutputFieldName { field: String },
    UnknownFieldReference { field: String },
    EmptyExpressionMapping { field: String },
    EmptyFunctionName { field: String },
    UnknownFunctionArgument { function: String, argument: String },
    EmptyConditionGroup,
    ReducerMissingField,
    UnknownReducerField { reducer: &'static str, field: String },
    UnknownGroupByField { field: String },
    EmptyTransformChain,
    InvalidNestedSpec { index: usize, source: Box<TransformSpecError> },
}
```

### FieldDefinitionError

Field definition validation errors.

```rust
pub enum FieldDefinitionError {
    EmptyName,
    NameTooLong { name: String, max: usize },
    InvalidNameStart { name: String },
    InvalidNameCharacters { name: String },
    DefaultTypeMismatch { name: String, declared: Box<FieldType>, actual: Box<FieldType> },
}
```

### FunctionRegistryError

Function registry operation errors.

```rust
pub enum FunctionRegistryError {
    FunctionNotFound { name: String },
    ParameterCountMismatch { name: String, expected: usize, actual: usize },
    ParameterTypeMismatch { name: String, parameter: String, expected: FieldType, actual: FieldValue },
    ExecutionFailed { name: String, reason: String },
    AsyncNotSupported { name: String },
    RegistryNotInitialized,
    InternalError(String),
}
```

### ExpressionEvaluationError

Expression evaluation errors.

```rust
pub enum ExpressionEvaluationError {
    VariableNotFound { name: String },
    FieldNotFound { field: String },
    InvalidFieldAccess { reason: String },
    FunctionNotFound { name: String },
    TypeError { reason: String },
    DivisionByZero,
    InvalidOperation { reason: String },
    ParseError { reason: String },
    EvaluationError { reason: String },
}
```

### TransformExecutionError

Transform execution errors.

```rust
pub enum TransformExecutionError {
    ValidationError { transform: String, reason: String },
    ExecutionError { transform: String, reason: String },
    SchemaValidationError { schema: String, reason: String },
    InternalError { operation: String, reason: String },
}
```

## AST Types (Expression Language)

### Expression

Represents an expression in the transform DSL.

```rust
pub enum Expression {
    Literal(Value),
    Variable(String),
    FieldAccess { object: Box<Expression>, field: String },
    BinaryOp { left: Box<Expression>, operator: Operator, right: Box<Expression> },
    UnaryOp { operator: UnaryOperator, expr: Box<Expression> },
    FunctionCall { name: String, args: Vec<Expression> },
    IfElse { condition: Box<Expression>, then_branch: Box<Expression>, else_branch: Option<Box<Expression>> },
    LetBinding { name: String, value: Box<Expression>, body: Box<Expression> },
    Return(Box<Expression>),
}
```

### Operator

Binary operators supported in expressions.

```rust
pub enum Operator {
    Add, Subtract, Multiply, Divide, Modulo, Power,
    Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual,
    And, Or,
}
```

### UnaryOperator

Unary operators supported in expressions.

```rust
pub enum UnaryOperator {
    Negate,
    Not,
}
```

### Value

Literal values in the AST.

```rust
pub enum Value {
    Number(f64),
    Boolean(bool),
    String(String),
    Null,
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
}
```

## Schema Integration

### NativeSchemaRegistry

Schema registry for native schema operations.

**Key Methods:**

- `new(database_operations: Arc<dyn DatabaseOperationsTrait>) -> Self` - Creates registry
- `load_native_schema_from_json(&self, schema_json: &str) -> Result<String, SchemaError>` - Loads schema from JSON
- `get_schema(&self, name: &str) -> Result<Option<NativeSchema>, SchemaError>` - Gets schema by name
- `list_schemas(&self) -> Result<Vec<String>, SchemaError>` - Lists schema names
- `validate_data(&self, schema_name: &str, data: &FieldValue) -> Result<bool, SchemaError>` - Validates data against schema

### NativeSchema

Native schema representation.

```rust
pub struct NativeSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub schema_type: SchemaType,
    pub payment_config: PaymentConfig,
}
```

## Usage Examples

### Basic Transform Execution

```rust
use datafold::transform::native::transform_spec::{TransformSpec, TransformType, MapTransform, FieldMapping};
use datafold::transform::native::types::FieldValue;
use datafold::transform::native_executor::NativeTransformExecutor;
use std::collections::HashMap;

// Create executor
let executor = NativeTransformExecutor::new();

// Prepare input data
let mut input_data = HashMap::new();
input_data.insert("name".to_string(), FieldValue::String("Alice".to_string()));
input_data.insert("age".to_string(), FieldValue::Integer(30));

// Define transform
let mut field_mappings = HashMap::new();
field_mappings.insert("greeting".to_string(), FieldMapping::Expression {
    expression: "\"Hello, \" + name + \"!\"".to_string(),
});
field_mappings.insert("is_adult".to_string(), FieldMapping::Expression {
    expression: "age >= 18".to_string(),
});

let map_transform = MapTransform::new(field_mappings);
let spec = TransformSpec::new(
    "greeting_transform",
    vec![/* input definitions */],
    /* output definition */,
    TransformType::Map(map_transform),
);

// Execute transform
let result = executor.execute_transform(&spec, input_data).await?;
```

### Custom Function Registration

```rust
use datafold::transform::function_registry::{FunctionRegistry, FunctionSignature, FieldType};
use datafold::transform::native::types::FieldValue;

// Create custom function
let double_impl = |args: Vec<FieldValue>| {
    Box::pin(async move {
        if let FieldValue::Integer(x) = args[0] {
            Ok(FieldValue::Integer(x * 2))
        } else {
            Err(FunctionRegistryError::ParameterTypeMismatch { /* ... */ })
        }
    })
};

// Register function
let mut registry = FunctionRegistry::new();
registry.register(
    FunctionSignature {
        name: "double".to_string(),
        parameters: vec![("value".to_string(), FieldType::Integer)],
        return_type: FieldType::Integer,
        is_async: false,
        description: "Double an integer value".to_string(),
    },
    double_impl,
)?;
```

### Schema Validation

```rust
use datafold::transform::native_executor::NativeTransformInput;

// Load and validate against schema
let schema_json = r#"{"name": "user_schema", "fields": {...}}"#;
executor.schema_registry()
    .load_native_schema_from_json(schema_json).await?;

let input = NativeTransformInput {
    values: user_data,
    schema_name: Some("user_schema".to_string()),
};

let result = executor.execute_transform(&spec, input).await?;