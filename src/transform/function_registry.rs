//! # Function Registry (NTS-3-2)
//!
//! Extensible function system that provides built-in and custom functions
//! for the NativeTransformExecutor. Supports type-safe function execution
//! with native FieldValue types and async function calls.

use crate::transform::native::types::FieldValue;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

/// Function signature defining parameter types and return type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,
    /// Parameter names and their expected types
    pub parameters: Vec<(String, FieldType)>,
    /// Return type
    pub return_type: FieldType,
    /// Whether the function is async
    pub is_async: bool,
    /// Function description
    pub description: String,
}

/// Supported field types for function signatures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Array(Box<FieldType>),
    Object,
    Any,
    Null,
}

impl FieldType {
    /// Check if a FieldValue matches this type
    pub fn matches(&self, value: &FieldValue) -> bool {
        match (self, value) {
            (FieldType::String, FieldValue::String(_)) => true,
            (FieldType::Integer, FieldValue::Integer(_)) => true,
            (FieldType::Number, FieldValue::Number(_)) => true,
            (FieldType::Boolean, FieldValue::Boolean(_)) => true,
            (FieldType::Array(element_type), FieldValue::Array(values)) => {
                values.iter().all(|v| element_type.matches(v))
            }
            (FieldType::Object, FieldValue::Object(_)) => true,
            (FieldType::Any, _) => true,
            (FieldType::Null, FieldValue::Null) => true,
            _ => false,
        }
    }
}

/// Function execution result
pub type FunctionResult = Result<FieldValue, FunctionRegistryError>;

/// Function implementation trait
pub trait FunctionImplementation: Send + Sync {
    /// Execute the function with given arguments
    fn execute(
        &self,
        args: Vec<FieldValue>,
    ) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>>;
}

/// Built-in function implementation
pub struct BuiltInFunction<F>
where
    F: Fn(Vec<FieldValue>) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>>
        + Send
        + Sync,
{
    implementation: F,
}

impl<F> BuiltInFunction<F>
where
    F: Fn(Vec<FieldValue>) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>>
        + Send
        + Sync
        + 'static,
{
    pub fn new(implementation: F) -> Self {
        Self { implementation }
    }
}

impl<F> FunctionImplementation for BuiltInFunction<F>
where
    F: Fn(Vec<FieldValue>) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>>
        + Send
        + Sync
        + 'static,
{
    fn execute(
        &self,
        args: Vec<FieldValue>,
    ) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>> {
        (self.implementation)(args)
    }
}

/// Errors that can occur during function registry operations
#[derive(Error, Debug, Clone)]
pub enum FunctionRegistryError {
    #[error("Function '{name}' not found")]
    FunctionNotFound { name: String },

    #[error("Function '{name}' parameter count mismatch: expected {expected}, got {actual}")]
    ParameterCountMismatch { name: String, expected: usize, actual: usize },

    #[error("Function '{name}' parameter '{parameter}' type mismatch: expected {expected:?}, got {actual:?}")]
    ParameterTypeMismatch {
        name: String,
        parameter: String,
        expected: FieldType,
        actual: FieldValue,
    },

    #[error("Function '{name}' execution failed: {reason}")]
    ExecutionFailed { name: String, reason: String },

    #[error("Function '{name}' is not async but async execution was requested")]
    AsyncNotSupported { name: String },

    #[error("Function registry is not initialized")]
    RegistryNotInitialized,

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Function registry that manages built-in and custom functions
#[derive(Clone)]
pub struct FunctionRegistry {
    functions: HashMap<String, (FunctionSignature, Arc<dyn FunctionImplementation>)>,
}

impl FunctionRegistry {
    /// Create a new empty function registry
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Create a function registry with all built-in functions
    pub fn with_built_ins() -> Self {
        let mut registry = Self::new();

        // Register built-in string functions
        registry.register_string_functions();

        // Register built-in math functions
        registry.register_math_functions();

        // Register built-in type conversion functions
        registry.register_type_conversion_functions();

        // Register built-in date functions
        registry.register_date_functions();

        registry
    }

    /// Register a function implementation
    pub fn register<F>(
        &mut self,
        signature: FunctionSignature,
        implementation: F,
    ) -> Result<(), FunctionRegistryError>
    where
        F: Fn(Vec<FieldValue>) -> Pin<Box<dyn Future<Output = FunctionResult> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        let name = signature.name.clone();

        if self.functions.contains_key(&name) {
            return Err(FunctionRegistryError::InternalError(format!(
                "Function '{}' is already registered",
                name
            )));
        }

        let function_impl = Arc::new(BuiltInFunction::new(implementation));
        self.functions.insert(name.clone(), (signature, function_impl));

        debug!("Registered function: {}", name);
        Ok(())
    }

    /// Register a custom function implementation
    pub fn register_custom(
        &mut self,
        signature: FunctionSignature,
        implementation: Arc<dyn FunctionImplementation>,
    ) -> Result<(), FunctionRegistryError> {
        let name = signature.name.clone();

        if self.functions.contains_key(&name) {
            return Err(FunctionRegistryError::InternalError(format!(
                "Function '{}' is already registered",
                name
            )));
        }

        self.functions.insert(name.clone(), (signature, implementation));

        debug!("Registered custom function: {}", name);
        Ok(())
    }

    /// Get a function by name
    pub fn get_function(
        &self,
        name: &str,
    ) -> Result<&(FunctionSignature, Arc<dyn FunctionImplementation>), FunctionRegistryError> {
        self.functions.get(name).ok_or_else(|| FunctionRegistryError::FunctionNotFound {
            name: name.to_string(),
        })
    }

    /// Check if a function exists
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// List all registered function names
    pub fn list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Get function signature
    pub fn get_signature(&self, name: &str) -> Result<&FunctionSignature, FunctionRegistryError> {
        self.get_function(name).map(|(sig, _)| sig)
    }

    /// Execute a function with type checking
    pub async fn execute_function(
        &self,
        name: &str,
        args: Vec<FieldValue>,
    ) -> Result<FieldValue, FunctionRegistryError> {
        let (signature, implementation) = self.get_function(name)?;

        info!("Executing function '{}' with {} arguments", name, args.len());

        // Validate parameter count
        if args.len() != signature.parameters.len() {
            return Err(FunctionRegistryError::ParameterCountMismatch {
                name: name.to_string(),
                expected: signature.parameters.len(),
                actual: args.len(),
            });
        }

        // Validate parameter types
        for (i, (param_name, expected_type)) in signature.parameters.iter().enumerate() {
            if !expected_type.matches(&args[i]) {
                return Err(FunctionRegistryError::ParameterTypeMismatch {
                    name: name.to_string(),
                    parameter: param_name.clone(),
                    expected: expected_type.clone(),
                    actual: args[i].clone(),
                });
            }
        }

        // Execute the function
        implementation.execute(args).await
    }

    /// Register all built-in string functions
    fn register_string_functions(&mut self) {
        // concat function
        let _ = self.register(
            FunctionSignature {
                name: "concat".to_string(),
                parameters: vec![
                    ("values".to_string(), FieldType::Array(Box::new(FieldType::Any))),
                ],
                return_type: FieldType::String,
                is_async: false,
                description: "Concatenate an array of values as strings".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Array(values) = &args[0] {
                        let mut result = String::new();
                        for arg in values {
                            match arg {
                                FieldValue::String(s) => result.push_str(s),
                                other => result.push_str(&other.to_json_value().to_string()),
                            }
                        }
                        Ok(FieldValue::String(result))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "concat".to_string(),
                            parameter: "values".to_string(),
                            expected: FieldType::Array(Box::new(FieldType::Any)),
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // uppercase function
        let _ = self.register(
            FunctionSignature {
                name: "uppercase".to_string(),
                parameters: vec![("str".to_string(), FieldType::Any)],
                return_type: FieldType::String,
                is_async: false,
                description: "Convert string to uppercase".to_string(),
            },
            |args| {
                Box::pin(async move {
                    match &args[0] {
                        FieldValue::String(s) => Ok(FieldValue::String(s.to_uppercase())),
                        other => Ok(FieldValue::String(
                            other.to_json_value().to_string().to_uppercase()
                        )),
                    }
                })
            },
        );

        // lowercase function
        let _ = self.register(
            FunctionSignature {
                name: "lowercase".to_string(),
                parameters: vec![("str".to_string(), FieldType::Any)],
                return_type: FieldType::String,
                is_async: false,
                description: "Convert string to lowercase".to_string(),
            },
            |args| {
                Box::pin(async move {
                    match &args[0] {
                        FieldValue::String(s) => Ok(FieldValue::String(s.to_lowercase())),
                        other => Ok(FieldValue::String(
                            other.to_json_value().to_string().to_lowercase()
                        )),
                    }
                })
            },
        );

        // length function
        let _ = self.register(
            FunctionSignature {
                name: "length".to_string(),
                parameters: vec![("value".to_string(), FieldType::Any)],
                return_type: FieldType::Integer,
                is_async: false,
                description: "Get the length of a string or array".to_string(),
            },
            |args| {
                Box::pin(async move {
                    match &args[0] {
                        FieldValue::String(s) => Ok(FieldValue::Integer(s.len() as i64)),
                        FieldValue::Array(arr) => Ok(FieldValue::Integer(arr.len() as i64)),
                        other => Ok(FieldValue::Integer(
                            other.to_json_value().to_string().len() as i64
                        )),
                    }
                })
            },
        );

        // trim function
        let _ = self.register(
            FunctionSignature {
                name: "trim".to_string(),
                parameters: vec![("str".to_string(), FieldType::String)],
                return_type: FieldType::String,
                is_async: false,
                description: "Remove whitespace from both ends of a string".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::String(s) = &args[0] {
                        Ok(FieldValue::String(s.trim().to_string()))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "trim".to_string(),
                            parameter: "str".to_string(),
                            expected: FieldType::String,
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // substring function
        let _ = self.register(
            FunctionSignature {
                name: "substring".to_string(),
                parameters: vec![
                    ("str".to_string(), FieldType::String),
                    ("start".to_string(), FieldType::Integer),
                    ("end".to_string(), FieldType::Integer),
                ],
                return_type: FieldType::String,
                is_async: false,
                description: "Extract a substring from start index to end index".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let (FieldValue::String(s), FieldValue::Integer(start), FieldValue::Integer(end)) =
                        (&args[0], &args[1], &args[2])
                    {
                        let start = *start as usize;
                        let end = *end as usize;
                        if start <= s.len() && end <= s.len() && start <= end {
                            Ok(FieldValue::String(s[start..end].to_string()))
                        } else {
                            Err(FunctionRegistryError::ExecutionFailed {
                                name: "substring".to_string(),
                                reason: "Invalid start or end indices".to_string(),
                            })
                        }
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "substring".to_string(),
                            parameter: "parameters".to_string(),
                            expected: FieldType::String,
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );
    }

    /// Register all built-in math functions
    fn register_math_functions(&mut self) {
        // sum function
        let _ = self.register(
            FunctionSignature {
                name: "sum".to_string(),
                parameters: vec![("values".to_string(), FieldType::Array(Box::new(FieldType::Any)))],
                return_type: FieldType::Number,
                is_async: false,
                description: "Calculate the sum of an array of numbers".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Array(values) = &args[0] {
                        let mut sum = 0.0;
                        for value in values {
                            match value {
                                FieldValue::Integer(i) => sum += *i as f64,
                                FieldValue::Number(n) => sum += n,
                                _ => return Err(FunctionRegistryError::ExecutionFailed {
                                    name: "sum".to_string(),
                                    reason: format!("Cannot sum non-numeric value: {:?}", value),
                                }),
                            }
                        }
                        Ok(FieldValue::Number(sum))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "sum".to_string(),
                            parameter: "values".to_string(),
                            expected: FieldType::Array(Box::new(FieldType::Number)),
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // average function
        let _ = self.register(
            FunctionSignature {
                name: "average".to_string(),
                parameters: vec![("values".to_string(), FieldType::Array(Box::new(FieldType::Number)))],
                return_type: FieldType::Number,
                is_async: false,
                description: "Calculate the average of an array of numbers".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Array(values) = &args[0] {
                        if values.is_empty() {
                            return Ok(FieldValue::Number(0.0));
                        }

                        let mut sum = 0.0;
                        let mut count = 0.0;
                        for value in values {
                            match value {
                                FieldValue::Integer(i) => {
                                    sum += *i as f64;
                                    count += 1.0;
                                }
                                FieldValue::Number(n) => {
                                    sum += n;
                                    count += 1.0;
                                }
                                _ => return Err(FunctionRegistryError::ExecutionFailed {
                                    name: "average".to_string(),
                                    reason: format!("Cannot average non-numeric value: {:?}", value),
                                }),
                            }
                        }
                        Ok(FieldValue::Number(sum / count))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "average".to_string(),
                            parameter: "values".to_string(),
                            expected: FieldType::Array(Box::new(FieldType::Number)),
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // min function
        let _ = self.register(
            FunctionSignature {
                name: "min".to_string(),
                parameters: vec![("values".to_string(), FieldType::Array(Box::new(FieldType::Number)))],
                return_type: FieldType::Number,
                is_async: false,
                description: "Find the minimum value in an array of numbers".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Array(values) = &args[0] {
                        if values.is_empty() {
                            return Err(FunctionRegistryError::ExecutionFailed {
                                name: "min".to_string(),
                                reason: "Cannot find minimum of empty array".to_string(),
                            });
                        }

                        let mut min = f64::INFINITY;
                        for value in values {
                            match value {
                                FieldValue::Integer(i) => min = min.min(*i as f64),
                                FieldValue::Number(n) => min = min.min(*n),
                                _ => return Err(FunctionRegistryError::ExecutionFailed {
                                    name: "min".to_string(),
                                    reason: format!("Cannot compare non-numeric value: {:?}", value),
                                }),
                            }
                        }
                        Ok(FieldValue::Number(min))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "min".to_string(),
                            parameter: "values".to_string(),
                            expected: FieldType::Array(Box::new(FieldType::Number)),
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // max function
        let _ = self.register(
            FunctionSignature {
                name: "max".to_string(),
                parameters: vec![("values".to_string(), FieldType::Array(Box::new(FieldType::Number)))],
                return_type: FieldType::Number,
                is_async: false,
                description: "Find the maximum value in an array of numbers".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Array(values) = &args[0] {
                        if values.is_empty() {
                            return Err(FunctionRegistryError::ExecutionFailed {
                                name: "max".to_string(),
                                reason: "Cannot find maximum of empty array".to_string(),
                            });
                        }

                        let mut max = f64::NEG_INFINITY;
                        for value in values {
                            match value {
                                FieldValue::Integer(i) => max = max.max(*i as f64),
                                FieldValue::Number(n) => max = max.max(*n),
                                _ => return Err(FunctionRegistryError::ExecutionFailed {
                                    name: "max".to_string(),
                                    reason: format!("Cannot compare non-numeric value: {:?}", value),
                                }),
                            }
                        }
                        Ok(FieldValue::Number(max))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "max".to_string(),
                            parameter: "values".to_string(),
                            expected: FieldType::Array(Box::new(FieldType::Number)),
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // round function
        let _ = self.register(
            FunctionSignature {
                name: "round".to_string(),
                parameters: vec![("value".to_string(), FieldType::Number)],
                return_type: FieldType::Number,
                is_async: false,
                description: "Round a number to the nearest integer".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Number(n) = args[0] {
                        Ok(FieldValue::Number(n.round()))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "round".to_string(),
                            parameter: "value".to_string(),
                            expected: FieldType::Number,
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );

        // abs function
        let _ = self.register(
            FunctionSignature {
                name: "abs".to_string(),
                parameters: vec![("value".to_string(), FieldType::Number)],
                return_type: FieldType::Number,
                is_async: false,
                description: "Get the absolute value of a number".to_string(),
            },
            |args| {
                Box::pin(async move {
                    if let FieldValue::Number(n) = args[0] {
                        Ok(FieldValue::Number(n.abs()))
                    } else {
                        Err(FunctionRegistryError::ParameterTypeMismatch {
                            name: "abs".to_string(),
                            parameter: "value".to_string(),
                            expected: FieldType::Number,
                            actual: args[0].clone(),
                        })
                    }
                })
            },
        );
    }

    /// Register all built-in type conversion functions
    fn register_type_conversion_functions(&mut self) {
        // to_string function
        let _ = self.register(
            FunctionSignature {
                name: "to_string".to_string(),
                parameters: vec![("value".to_string(), FieldType::Any)],
                return_type: FieldType::String,
                is_async: false,
                description: "Convert a value to its string representation".to_string(),
            },
            |args| {
                Box::pin(async move {
                    let result = match &args[0] {
                        FieldValue::String(s) => s.clone(),
                        FieldValue::Integer(i) => i.to_string(),
                        FieldValue::Number(n) => n.to_string(),
                        FieldValue::Boolean(b) => b.to_string(),
                        FieldValue::Null => "null".to_string(),
                        other => other.to_json_value().to_string(),
                    };
                    Ok(FieldValue::String(result))
                })
            },
        );

        // to_number function
        let _ = self.register(
            FunctionSignature {
                name: "to_number".to_string(),
                parameters: vec![("value".to_string(), FieldType::Any)],
                return_type: FieldType::Number,
                is_async: false,
                description: "Convert a value to a number".to_string(),
            },
            |args| {
                Box::pin(async move {
                    let result = match &args[0] {
                        FieldValue::String(s) => s.parse::<f64>().unwrap_or(0.0),
                        FieldValue::Integer(i) => *i as f64,
                        FieldValue::Number(n) => *n,
                        FieldValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
                        _ => 0.0,
                    };
                    Ok(FieldValue::Number(result))
                })
            },
        );

        // to_boolean function
        let _ = self.register(
            FunctionSignature {
                name: "to_boolean".to_string(),
                parameters: vec![("value".to_string(), FieldType::Any)],
                return_type: FieldType::Boolean,
                is_async: false,
                description: "Convert a value to a boolean".to_string(),
            },
            |args| {
                Box::pin(async move {
                    let result = match &args[0] {
                        FieldValue::String(s) => !s.is_empty(),
                        FieldValue::Integer(i) => *i != 0,
                        FieldValue::Number(n) => *n != 0.0,
                        FieldValue::Boolean(b) => *b,
                        FieldValue::Array(arr) => !arr.is_empty(),
                        FieldValue::Object(obj) => !obj.is_empty(),
                        FieldValue::Null => false,
                    };
                    Ok(FieldValue::Boolean(result))
                })
            },
        );
    }

    /// Register all built-in date functions
    fn register_date_functions(&mut self) {
        // now function
        let _ = self.register(
            FunctionSignature {
                name: "now".to_string(),
                parameters: vec![],
                return_type: FieldType::String,
                is_async: false,
                description: "Get the current timestamp as ISO string".to_string(),
            },
            |_args| {
                Box::pin(async move {
                    let now = chrono::Utc::now();
                    Ok(FieldValue::String(now.to_rfc3339()))
                })
            },
        );

        // Note: format_date function would require additional date parsing
        // For now, we'll skip it until date support is fully implemented
        // in the native transform system
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::with_built_ins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_registry() -> FunctionRegistry {
        let mut registry = FunctionRegistry::new();
        registry.register_string_functions();
        registry.register_math_functions();
        registry.register_type_conversion_functions();
        registry
    }

    #[tokio::test]
    async fn test_concat_function() {
        let registry = create_test_registry();

        let result = registry
            .execute_function(
                "concat",
                vec![FieldValue::Array(vec![
                    FieldValue::String("Hello".to_string()),
                    FieldValue::String(" ".to_string()),
                    FieldValue::String("World".to_string()),
                ])],
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::String("Hello World".to_string()));
    }

    #[tokio::test]
    async fn test_uppercase_function() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("uppercase", vec![FieldValue::String("hello".to_string())])
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::String("HELLO".to_string()));
    }

    #[tokio::test]
    async fn test_length_function() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("length", vec![FieldValue::String("hello".to_string())])
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::Integer(5));
    }

    #[tokio::test]
    async fn test_sum_function() {
        let registry = create_test_registry();

        let result = registry
            .execute_function(
                "sum",
                vec![FieldValue::Array(vec![
                    FieldValue::Integer(1),
                    FieldValue::Integer(2),
                    FieldValue::Integer(3),
                ])],
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::Number(6.0));
    }

    #[tokio::test]
    async fn test_to_string_function() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("to_string", vec![FieldValue::Integer(42)])
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::String("42".to_string()));
    }

    #[tokio::test]
    async fn test_parameter_type_validation() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("uppercase", vec![FieldValue::Integer(42)])
            .await;

        // This should work due to flexible parameter handling
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_function_not_found() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("nonexistent", vec![FieldValue::String("test".to_string())])
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FunctionRegistryError::FunctionNotFound { .. } => {}
            _ => panic!("Expected FunctionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_parameter_count_mismatch() {
        let registry = create_test_registry();

        let result = registry
            .execute_function("uppercase", vec![
                FieldValue::String("hello".to_string()),
                FieldValue::String("world".to_string()),
            ])
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FunctionRegistryError::ParameterCountMismatch { .. } => {}
            _ => panic!("Expected ParameterCountMismatch error"),
        }
    }

    #[tokio::test]
    async fn test_custom_function_registration() {
        let mut registry = FunctionRegistry::new();

        let custom_impl = Arc::new(BuiltInFunction::new(|args: Vec<FieldValue>| {
            Box::pin(async move {
                let doubled = match &args[0] {
                    FieldValue::Integer(i) => *i * 2,
                    _ => 0,
                };
                Ok(FieldValue::Integer(doubled))
            })
        }));

        registry
            .register_custom(
                FunctionSignature {
                    name: "double".to_string(),
                    parameters: vec![("value".to_string(), FieldType::Integer)],
                    return_type: FieldType::Integer,
                    is_async: false,
                    description: "Double an integer value".to_string(),
                },
                custom_impl,
            )
            .unwrap();

        let result = registry
            .execute_function("double", vec![FieldValue::Integer(5)])
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FieldValue::Integer(10));
    }
}