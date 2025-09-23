# Function Registry Documentation

The Native Transform System (NTS-3) includes a comprehensive function registry with built-in functions for string manipulation, mathematical operations, type conversion, and date operations. This document provides detailed documentation for all available functions.

## Overview

The function registry provides:
- **Type-safe function execution** with compile-time parameter validation
- **Built-in functions** for common operations
- **Extensible architecture** for custom function registration
- **Async function support** for I/O operations
- **Error handling** with specific error types

## Function Categories

### String Functions

#### `concat(values: Array<Any>) -> String`

Concatenates all values in an array as strings.

**Parameters:**
- `values`: Array of any type (elements will be converted to strings)

**Returns:** Single concatenated string

**Examples:**
```rust
concat(["Hello", " ", "World"]) // Returns: "Hello World"
concat([1, 2, 3])              // Returns: "123"
concat(["a", 1, true])         // Returns: "atrue"
```

#### `uppercase(str: Any) -> String`

Converts a string to uppercase. Non-string inputs are converted to strings first.

**Parameters:**
- `str`: Value to convert (string, number, boolean, etc.)

**Returns:** Uppercase string representation

**Examples:**
```rust
uppercase("hello")      // Returns: "HELLO"
uppercase("Hello")      // Returns: "HELLO"
uppercase(123)          // Returns: "123"
uppercase(true)         // Returns: "TRUE"
```

#### `lowercase(str: Any) -> String`

Converts a string to lowercase. Non-string inputs are converted to strings first.

**Parameters:**
- `str`: Value to convert (string, number, boolean, etc.)

**Returns:** Lowercase string representation

**Examples:**
```rust
lowercase("HELLO")      // Returns: "hello"
lowercase("Hello")      // Returns: "hello"
lowercase(123)          // Returns: "123"
lowercase(true)         // Returns: "true"
```

#### `length(value: Any) -> Integer`

Returns the length of a string or array.

**Parameters:**
- `value`: String or array to measure

**Returns:** Length as integer

**Examples:**
```rust
length("hello")         // Returns: 5
length([1, 2, 3])      // Returns: 3
length("")             // Returns: 0
length([])             // Returns: 0
```

#### `trim(str: String) -> String`

Removes whitespace from both ends of a string.

**Parameters:**
- `str`: String to trim

**Returns:** Trimmed string

**Examples:**
```rust
trim("  hello  ")      // Returns: "hello"
trim("  hello")        // Returns: "hello"
trim("hello  ")        // Returns: "hello"
trim("  ")             // Returns: ""
```

#### `substring(str: String, start: Integer, end: Integer) -> String`

Extracts a substring from a string.

**Parameters:**
- `str`: Source string
- `start`: Starting index (0-based, inclusive)
- `end`: Ending index (0-based, exclusive)

**Returns:** Extracted substring

**Examples:**
```rust
substring("hello", 0, 2)    // Returns: "he"
substring("hello", 1, 4)    // Returns: "ell"
substring("hello", 2, 5)    // Returns: "llo"
```

**Error Conditions:**
- `start` or `end` out of bounds
- `start` > `end`
- `start` or `end` negative

### Math Functions

#### `sum(values: Array<Number>) -> Number`

Calculates the sum of all numeric values in an array.

**Parameters:**
- `values`: Array of numbers (integers or floats)

**Returns:** Sum as number

**Examples:**
```rust
sum([1, 2, 3, 4])          // Returns: 10.0
sum([1.5, 2.5, 3.0])       // Returns: 7.0
sum([10, 20.5, 30])        // Returns: 60.5
sum([])                    // Returns: 0.0
```

**Error Conditions:**
- Array contains non-numeric values
- Empty array (returns 0.0, not an error)

#### `average(values: Array<Number>) -> Number`

Calculates the average of all numeric values in an array.

**Parameters:**
- `values`: Array of numbers (integers or floats)

**Returns:** Average as number

**Examples:**
```rust
average([1, 2, 3, 4])      // Returns: 2.5
average([10, 20, 30])      // Returns: 20.0
average([5.5, 6.5])        // Returns: 6.0
average([])                // Returns: 0.0
```

**Error Conditions:**
- Array contains non-numeric values
- Empty array (returns 0.0, not an error)

#### `min(values: Array<Number>) -> Number`

Finds the minimum value in an array of numbers.

**Parameters:**
- `values`: Array of numbers (integers or floats)

**Returns:** Minimum value as number

**Examples:**
```rust
min([3, 1, 4, 1, 5])       // Returns: 1.0
min([10.5, 8.2, 12.1])     // Returns: 8.2
min([42])                  // Returns: 42.0
```

**Error Conditions:**
- Array contains non-numeric values
- Empty array (throws error)

#### `max(values: Array<Number>) -> Number`

Finds the maximum value in an array of numbers.

**Parameters:**
- `values`: Array of numbers (integers or floats)

**Returns:** Maximum value as number

**Examples:**
```rust
max([3, 1, 4, 1, 5])       // Returns: 5.0
max([10.5, 8.2, 12.1])     // Returns: 12.1
max([42])                  // Returns: 42.0
```

**Error Conditions:**
- Array contains non-numeric values
- Empty array (throws error)

#### `round(value: Number) -> Number`

Rounds a number to the nearest integer.

**Parameters:**
- `value`: Number to round

**Returns:** Rounded number

**Examples:**
```rust
round(3.7)                 // Returns: 4.0
round(3.2)                 // Returns: 3.0
round(3.5)                 // Returns: 4.0
round(-3.7)                // Returns: -4.0
```

#### `abs(value: Number) -> Number`

Returns the absolute value of a number.

**Parameters:**
- `value`: Number to get absolute value of

**Returns:** Absolute value as number

**Examples:**
```rust
abs(5.5)                   // Returns: 5.5
abs(-3.2)                  // Returns: 3.2
abs(-42)                   // Returns: 42.0
abs(0)                     // Returns: 0.0
```

### Type Conversion Functions

#### `to_string(value: Any) -> String`

Converts any value to its string representation.

**Parameters:**
- `value`: Value to convert

**Returns:** String representation

**Examples:**
```rust
to_string(42)              // Returns: "42"
to_string(3.14)            // Returns: "3.14"
to_string(true)            // Returns: "true"
to_string(null)            // Returns: "null"
to_string([1, 2, 3])       // Returns: "[1, 2, 3]"
```

#### `to_number(value: Any) -> Number`

Converts a value to a number. Returns 0.0 for invalid conversions.

**Parameters:**
- `value`: Value to convert

**Returns:** Number representation or 0.0

**Examples:**
```rust
to_number("42")            // Returns: 42.0
to_number("3.14")          // Returns: 3.14
to_number(true)            // Returns: 1.0
to_number(false)           // Returns: 0.0
to_number("abc")           // Returns: 0.0
to_number(null)            // Returns: 0.0
```

#### `to_boolean(value: Any) -> Boolean`

Converts a value to a boolean based on truthiness.

**Parameters:**
- `value`: Value to convert

**Returns:** Boolean representation

**Examples:**
```rust
to_boolean(1)              // Returns: true
to_boolean(0)              // Returns: false
to_boolean(42)             // Returns: true
to_boolean("hello")        // Returns: true
to_boolean("")             // Returns: false
to_boolean([1, 2])         // Returns: true
to_boolean([])             // Returns: false
to_boolean(null)           // Returns: false
```

### Date Functions

#### `now() -> String`

Returns the current timestamp as an ISO 8601 formatted string.

**Parameters:** None

**Returns:** Current timestamp as ISO string (e.g., "2024-01-15T10:30:00Z")

**Examples:**
```rust
now()  // Returns: "2024-01-15T10:30:00Z" (current time)
```

## Function Usage in Transforms

### Using Functions in Map Transforms

```rust
use datafold::transform::native::transform_spec::{FieldMapping, MapTransform};
use std::collections::HashMap;

let mut field_mappings = HashMap::new();

// String functions
field_mappings.insert("upper_name".to_string(), FieldMapping::Function {
    name: "uppercase".to_string(),
    arguments: vec!["name".to_string()],
});

// Math functions
field_mappings.insert("total_score".to_string(), FieldMapping::Function {
    name: "sum".to_string(),
    arguments: vec!["scores".to_string()],
});

// Type conversion
field_mappings.insert("score_text".to_string(), FieldMapping::Function {
    name: "to_string".to_string(),
    arguments: vec!["score".to_string()],
});

// Multiple functions in expressions
field_mappings.insert("display_score".to_string(), FieldMapping::Expression {
    expression: "to_string(score) + \" points\"".to_string(),
});

let map_transform = MapTransform::new(field_mappings);
```

### Using Functions in Filter Conditions

Functions can be used within filter conditions, but they must return boolean values:

```rust
use datafold::transform::native::transform_spec::{FilterCondition, FilterTransform};

// Using length function in filter
let filter_condition = FilterCondition::GreaterThan {
    field: "length(username)".to_string(),
    value: FieldValue::Integer(5),
};

// Using boolean conversion in filter
let filter_condition = FilterCondition::Equals {
    field: "to_boolean(active)".to_string(),
    value: FieldValue::Boolean(true),
};
```

### Custom Function Registration

You can register custom functions to extend the built-in functionality:

```rust
use datafold::transform::function_registry::{FunctionRegistry, FunctionSignature, FieldType};
use datafold::transform::native::types::FieldValue;

// Create custom function implementation
let double_impl = |args: Vec<FieldValue>| {
    Box::pin(async move {
        if let FieldValue::Integer(x) = args[0] {
            Ok(FieldValue::Integer(x * 2))
        } else {
            Err(FunctionRegistryError::ParameterTypeMismatch {
                name: "double".to_string(),
                parameter: "value".to_string(),
                expected: FieldType::Integer,
                actual: args[0].clone(),
            })
        }
    })
};

// Register the function
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

## Function Error Handling

### Common Error Types

1. **FunctionNotFound**: Function doesn't exist in registry
   ```rust
   // Error: "nonexistent" function not found
   FieldMapping::Function {
       name: "nonexistent".to_string(),
       arguments: vec!["field".to_string()],
   }
   ```

2. **ParameterCountMismatch**: Wrong number of arguments
   ```rust
   // Error: uppercase expects 1 argument, got 2
   FieldMapping::Function {
       name: "uppercase".to_string(),
       arguments: vec!["field1".to_string(), "field2".to_string()],
   }
   ```

3. **ParameterTypeMismatch**: Wrong parameter types
   ```rust
   // Error: sum expects Array<Number>, got String
   FieldMapping::Function {
       name: "sum".to_string(),
       arguments: vec!["text_field".to_string()],
   }
   ```

4. **ExecutionFailed**: Runtime errors during function execution
   ```rust
   // Error: division by zero in custom function
   FieldMapping::Function {
       name: "custom_divide".to_string(),
       arguments: vec!["numerator".to_string(), "zero".to_string()],
   }
   ```

### Error Recovery Strategies

1. **Use Default Values**: Provide fallback values in expressions
   ```rust
   FieldMapping::Expression {
       expression: "to_number(field) || 0".to_string(), // Fallback to 0
   }
   ```

2. **Validate Inputs**: Use filter transforms to validate data before processing
   ```rust
   // First filter out invalid records
   FilterTransform {
       condition: FilterCondition::GreaterThan {
           field: "length(field)".to_string(),
           value: FieldValue::Integer(0),
       },
   }
   ```

3. **Handle Optional Fields**: Use conditional expressions
   ```rust
   FieldMapping::Expression {
       expression: "field != null ? to_string(field) : \"default\"".to_string(),
   }
   ```

## Performance Considerations

### Function Execution Overhead

- **Built-in functions**: Minimal overhead, optimized for performance
- **Custom functions**: Additional overhead for parameter validation and type checking
- **Nested function calls**: Each function call adds validation overhead

### Optimization Strategies

1. **Minimize Function Calls**: Use expressions where possible instead of function calls
   ```rust
   // Less optimal: Multiple function calls
   FieldMapping::Expression {
       expression: "to_string(field1) + \"_\" + to_string(field2)".to_string(),
   }

   // More optimal: Single function call
   FieldMapping::Function {
       name: "concat".to_string(),
       arguments: vec!["field1".to_string(), "_".to_string(), "field2".to_string()],
   }
   ```

2. **Use Direct Field Access**: Prefer direct field access over function calls
   ```rust
   // Optimal: Direct access
   FieldMapping::Direct { field: "existing_field".to_string() }

   // Less optimal: Function call
   FieldMapping::Function {
       name: "to_string".to_string(),
       arguments: vec!["existing_field".to_string()],
   }
   ```

3. **Batch Operations**: Use array functions instead of individual operations
   ```rust
   // Optimal: Single function call on array
   FieldMapping::Function {
       name: "sum".to_string(),
       arguments: vec!["values".to_string()],
   }

   // Less optimal: Individual additions
   FieldMapping::Expression {
       expression: "values.0 + values.1 + values.2".to_string(),
   }
   ```

## Function Registry API

### Creating a Custom Registry

```rust
use datafold::transform::function_registry::FunctionRegistry;

// Create registry with only built-in functions
let registry = FunctionRegistry::with_built_ins();

// Create empty registry for custom functions only
let mut custom_registry = FunctionRegistry::new();

// Add custom functions
custom_registry.register(/* custom function */)?;

// Use with executor
let executor = NativeTransformExecutor::new_with_functions(
    schema_registry,
    Arc::new(custom_registry),
);
```

### Function Discovery

```rust
let registry = FunctionRegistry::with_built_ins();

// List all available functions
let function_names = registry.list_functions();
println!("Available functions: {:?}", function_names);

// Get function signature
let signature = registry.get_signature("uppercase")?;
println!("Uppercase function: {:?}", signature);

// Check if function exists
if registry.has_function("custom_function") {
    println!("Custom function is available");
}
```

### Async Function Support

```rust
// Functions can be async
let async_function_impl = |args: Vec<FieldValue>| {
    Box::pin(async move {
        // Perform async operations
        let result = some_async_operation(args[0]).await?;
        Ok(result)
    })
};

registry.register(
    FunctionSignature {
        name: "async_operation".to_string(),
        parameters: vec![("input".to_string(), FieldType::String)],
        return_type: FieldType::String,
        is_async: true, // Must be set to true for async functions
        description: "Async operation example".to_string(),
    },
    async_function_impl,
)?;
```

## Best Practices

### Function Selection

1. **Use Built-in Functions**: Prefer built-in functions over custom implementations
2. **Validate Inputs**: Always validate function inputs to prevent runtime errors
3. **Handle Edge Cases**: Consider empty arrays, null values, and type conversion errors
4. **Document Custom Functions**: Provide clear documentation for custom function behavior

### Performance Optimization

1. **Minimize Function Calls**: Use direct field access and expressions when possible
2. **Batch Operations**: Use array functions for processing multiple values
3. **Type Consistency**: Use consistent types to avoid unnecessary conversions
4. **Error Handling**: Implement proper error handling to prevent cascade failures

### Maintainability

1. **Clear Naming**: Use descriptive function and parameter names
2. **Documentation**: Document function behavior, parameters, and return values
3. **Testing**: Test functions with various input types and edge cases
4. **Versioning**: Consider backward compatibility when modifying functions

This comprehensive function registry provides all the tools needed for complex data transformations while maintaining type safety and performance.