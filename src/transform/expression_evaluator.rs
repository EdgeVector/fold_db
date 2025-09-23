//! # Expression Evaluator (NTS-3-4)
//!
//! Native type expression parsing and evaluation system for the NativeTransformExecutor.
//! This module provides a complete expression evaluation engine that works with FieldValue
//! types and integrates with the FunctionRegistry for extensible function support.

use crate::transform::function_registry::FunctionRegistry;
use crate::transform::native::types::FieldValue;
use crate::transform::parser::TransformParser;
use log::{debug, error};
use std::collections::HashMap;
use thiserror::Error;

/// Expression evaluation error types
#[derive(Error, Debug, Clone)]
pub enum ExpressionEvaluationError {
    #[error("Variable '{name}' not found")]
    VariableNotFound { name: String },

    #[error("Field '{field}' not found in object")]
    FieldNotFound { field: String },

    #[error("Invalid field access: {reason}")]
    InvalidFieldAccess { reason: String },

    #[error("Function '{name}' not found")]
    FunctionNotFound { name: String },

    #[error("Type error: {reason}")]
    TypeError { reason: String },

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid operation: {reason}")]
    InvalidOperation { reason: String },

    #[error("Parse error: {reason}")]
    ParseError { reason: String },

    #[error("Evaluation error: {reason}")]
    EvaluationError { reason: String },
}

/// Expression evaluator for native FieldValue types
pub struct ExpressionEvaluator<'a> {
    /// Function registry for function calls
    function_registry: &'a FunctionRegistry,

    /// Context variables for field/variable resolution
    context: &'a HashMap<String, FieldValue>,
}

impl<'a> ExpressionEvaluator<'a> {
    /// Create a new expression evaluator
    pub fn new(
        function_registry: &'a FunctionRegistry,
        context: &'a HashMap<String, FieldValue>,
    ) -> Self {
        Self {
            function_registry,
            context,
        }
    }

    /// Evaluate an expression string
    pub async fn evaluate_expression(
        &self,
        expression: &str,
    ) -> Result<FieldValue, ExpressionEvaluationError> {
        debug!("🧮 Evaluating expression: {}", expression);

        // Parse the expression using the existing PEST parser
        let parser = TransformParser::new();
        let parsed_expr = parser.parse_expression(expression)
            .map_err(|e| ExpressionEvaluationError::ParseError {
                reason: format!("Failed to parse expression '{}': {}", expression, e)
            })?;

        // Evaluate the parsed expression
        self.evaluate_ast(parsed_expr).await
    }

    /// Evaluate an AST expression
    async fn evaluate_ast(&self, expr: crate::transform::ast::Expression) -> Result<FieldValue, ExpressionEvaluationError> {
        match expr {
            crate::transform::ast::Expression::Literal(value) => {
                Ok(self.convert_ast_value_to_field_value(value))
            }
            crate::transform::ast::Expression::Variable(name) => {
                self.resolve_variable(&name).await
            }
            crate::transform::ast::Expression::FieldAccess { object, field } => {
                let obj_value = Box::pin(self.evaluate_ast(*object)).await?;
                self.resolve_field_access(obj_value, &field).await
            }
            crate::transform::ast::Expression::BinaryOp { left, operator, right } => {
                let left_val = Box::pin(self.evaluate_ast(*left)).await?;
                let right_val = Box::pin(self.evaluate_ast(*right)).await?;
                self.evaluate_binary_op(left_val, operator, right_val).await
            }
            crate::transform::ast::Expression::UnaryOp { operator, expr } => {
                let val = Box::pin(self.evaluate_ast(*expr)).await?;
                self.evaluate_unary_op(operator, val).await
            }
            crate::transform::ast::Expression::FunctionCall { name, args } => {
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(Box::pin(self.evaluate_ast(arg)).await?);
                }
                self.evaluate_function_call(&name, arg_values).await
            }
            crate::transform::ast::Expression::IfElse { condition, then_branch, else_branch } => {
                let cond_val = Box::pin(self.evaluate_ast(*condition)).await?;
                if self.is_truthy(&cond_val) {
                    Box::pin(self.evaluate_ast(*then_branch)).await
                } else if let Some(else_expr) = else_branch {
                    Box::pin(self.evaluate_ast(*else_expr)).await
                } else {
                    Ok(FieldValue::Null)
                }
            }
            crate::transform::ast::Expression::LetBinding { name: _, value, body } => {
                let _val = Box::pin(self.evaluate_ast(*value)).await?;
                // For now, we don't support local variables, just evaluate the body
                // In a full implementation, we'd need to maintain a local scope
                Box::pin(self.evaluate_ast(*body)).await
            }
            crate::transform::ast::Expression::Return(expr) => {
                Box::pin(self.evaluate_ast(*expr)).await
            }
        }
    }

    /// Convert AST Value to FieldValue
    fn convert_ast_value_to_field_value(&self, value: crate::transform::ast::Value) -> FieldValue {
        match value {
            crate::transform::ast::Value::Number(n) => FieldValue::Number(n),
            crate::transform::ast::Value::Boolean(b) => FieldValue::Boolean(b),
            crate::transform::ast::Value::String(s) => FieldValue::String(s),
            crate::transform::ast::Value::Null => FieldValue::Null,
            crate::transform::ast::Value::Object(_) => {
                // For now, convert to JSON and back to handle object conversion
                let json_value: serde_json::Value = value.into();
                FieldValue::from_json_value(json_value)
            }
            crate::transform::ast::Value::Array(_) => {
                // For now, convert to JSON and back to handle array conversion
                let json_value: serde_json::Value = value.into();
                FieldValue::from_json_value(json_value)
            }
        }
    }

    /// Resolve a variable from context
    async fn resolve_variable(&self, name: &str) -> Result<FieldValue, ExpressionEvaluationError> {
        self.context.get(name)
            .cloned()
            .ok_or_else(|| ExpressionEvaluationError::VariableNotFound {
                name: name.to_string(),
            })
    }

    /// Resolve field access on an object or array
    async fn resolve_field_access(&self, object: FieldValue, field: &str) -> Result<FieldValue, ExpressionEvaluationError> {
        match object {
            FieldValue::Object(mut obj) => {
                obj.remove(field)
                    .ok_or_else(|| ExpressionEvaluationError::FieldNotFound {
                        field: field.to_string(),
                    })
            }
            FieldValue::Array(mut arr) => {
                // Handle array index access like "array.0" or "array.1"
                if let Ok(index) = field.parse::<usize>() {
                    if index < arr.len() {
                        Ok(arr.swap_remove(index))
                    } else {
                        Err(ExpressionEvaluationError::InvalidFieldAccess {
                            reason: format!("Array index {} out of bounds (length: {})", index, arr.len()),
                        })
                    }
                } else {
                    Err(ExpressionEvaluationError::InvalidFieldAccess {
                        reason: format!("Cannot access field '{}' on array", field),
                    })
                }
            }
            _ => Err(ExpressionEvaluationError::InvalidFieldAccess {
                reason: format!("Cannot access field '{}' on non-object type", field),
            }),
        }
    }

    /// Evaluate binary operations
    async fn evaluate_binary_op(
        &self,
        left: FieldValue,
        operator: crate::transform::ast::Operator,
        right: FieldValue,
    ) -> Result<FieldValue, ExpressionEvaluationError> {
        match operator {
            // Arithmetic operators
            crate::transform::ast::Operator::Add => self.evaluate_add(left, right).await,
            crate::transform::ast::Operator::Subtract => self.evaluate_subtract(left, right).await,
            crate::transform::ast::Operator::Multiply => self.evaluate_multiply(left, right).await,
            crate::transform::ast::Operator::Divide => self.evaluate_divide(left, right).await,
            crate::transform::ast::Operator::Modulo => self.evaluate_modulo(left, right).await,
            crate::transform::ast::Operator::Power => self.evaluate_power(left, right).await,

            // Comparison operators
            crate::transform::ast::Operator::Equal => self.evaluate_equal(left, right).await,
            crate::transform::ast::Operator::NotEqual => self.evaluate_not_equal(left, right).await,
            crate::transform::ast::Operator::LessThan => self.evaluate_less_than(left, right).await,
            crate::transform::ast::Operator::LessThanOrEqual => self.evaluate_less_than_or_equal(left, right).await,
            crate::transform::ast::Operator::GreaterThan => self.evaluate_greater_than(left, right).await,
            crate::transform::ast::Operator::GreaterThanOrEqual => self.evaluate_greater_than_or_equal(left, right).await,

            // Logical operators
            crate::transform::ast::Operator::And => self.evaluate_and(left, right).await,
            crate::transform::ast::Operator::Or => self.evaluate_or(left, right).await,
        }
    }

    /// Evaluate unary operations
    async fn evaluate_unary_op(
        &self,
        operator: crate::transform::ast::UnaryOperator,
        operand: FieldValue,
    ) -> Result<FieldValue, ExpressionEvaluationError> {
        match operator {
            crate::transform::ast::UnaryOperator::Negate => self.evaluate_negate(operand).await,
            crate::transform::ast::UnaryOperator::Not => self.evaluate_not(operand).await,
        }
    }

    /// Evaluate function calls
    async fn evaluate_function_call(
        &self,
        name: &str,
        args: Vec<FieldValue>,
    ) -> Result<FieldValue, ExpressionEvaluationError> {
        self.function_registry
            .execute_function(name, args)
            .await
            .map_err(|e| ExpressionEvaluationError::FunctionNotFound {
                name: format!("{}: {}", name, e),
            })
    }

    /// Check if a value is truthy
    fn is_truthy(&self, value: &FieldValue) -> bool {
        match value {
            FieldValue::Boolean(b) => *b,
            FieldValue::String(s) => !s.is_empty(),
            FieldValue::Integer(i) => *i != 0,
            FieldValue::Number(n) => *n != 0.0,
            FieldValue::Array(arr) => !arr.is_empty(),
            FieldValue::Object(obj) => !obj.is_empty(),
            FieldValue::Null => false,
        }
    }

    // Arithmetic operation implementations
    async fn evaluate_add(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => Ok(FieldValue::Integer(a + b)),
            (FieldValue::Number(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a + b)),
            (FieldValue::Integer(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a as f64 + b)),
            (FieldValue::Number(a), FieldValue::Integer(b)) => Ok(FieldValue::Number(a + b as f64)),
            (FieldValue::String(a), FieldValue::String(b)) => Ok(FieldValue::String(format!("{}{}", a, b))),
            (FieldValue::String(a), FieldValue::Integer(b)) => Ok(FieldValue::String(format!("{}{}", a, b))),
            (FieldValue::String(a), FieldValue::Number(b)) => Ok(FieldValue::String(format!("{}{}", a, b))),
            (FieldValue::Integer(a), FieldValue::String(b)) => Ok(FieldValue::String(format!("{}{}", a, b))),
            (FieldValue::Number(a), FieldValue::String(b)) => Ok(FieldValue::String(format!("{}{}", a, b))),
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot add incompatible types: {:?} + {:?}", left_val, right_val),
            }),
        }
    }

    async fn evaluate_subtract(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => Ok(FieldValue::Integer(a - b)),
            (FieldValue::Number(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a - b)),
            (FieldValue::Integer(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a as f64 - b)),
            (FieldValue::Number(a), FieldValue::Integer(b)) => Ok(FieldValue::Number(a - b as f64)),
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot subtract incompatible types: {:?} - {:?}", left_val, right_val),
            }),
        }
    }

    async fn evaluate_multiply(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => Ok(FieldValue::Integer(a * b)),
            (FieldValue::Number(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a * b)),
            (FieldValue::Integer(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a as f64 * b)),
            (FieldValue::Number(a), FieldValue::Integer(b)) => Ok(FieldValue::Number(a * b as f64)),
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot multiply incompatible types: {:?} * {:?}", left_val, right_val),
            }),
        }
    }

    async fn evaluate_divide(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => {
                if b == 0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a as f64 / b as f64))
            }
            (FieldValue::Number(a), FieldValue::Number(b)) => {
                if b == 0.0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a / b))
            }
            (FieldValue::Integer(a), FieldValue::Number(b)) => {
                if b == 0.0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a as f64 / b))
            }
            (FieldValue::Number(a), FieldValue::Integer(b)) => {
                if b == 0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a / b as f64))
            }
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot divide incompatible types: {:?} / {:?}", left_val, right_val),
            }),
        }
    }

    async fn evaluate_modulo(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => {
                if b == 0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Integer(a % b))
            }
            (FieldValue::Number(a), FieldValue::Number(b)) => {
                if b == 0.0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a % b))
            }
            (FieldValue::Integer(a), FieldValue::Number(b)) => {
                if b == 0.0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a as f64 % b))
            }
            (FieldValue::Number(a), FieldValue::Integer(b)) => {
                if b == 0 {
                    return Err(ExpressionEvaluationError::DivisionByZero);
                }
                Ok(FieldValue::Number(a % b as f64))
            }
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compute modulo for incompatible types: {:?} % {:?}", left_val, right_val),
            }),
        }
    }

    async fn evaluate_power(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => {
                if b < 0 {
                    Ok(FieldValue::Number((a as f64).powi(b as i32)))
                } else {
                    Ok(FieldValue::Integer(a.pow(b as u32)))
                }
            }
            (FieldValue::Number(a), FieldValue::Number(b)) => Ok(FieldValue::Number(a.powf(b))),
            (FieldValue::Integer(a), FieldValue::Number(b)) => Ok(FieldValue::Number((a as f64).powf(b))),
            (FieldValue::Number(a), FieldValue::Integer(b)) => Ok(FieldValue::Number(a.powi(b as i32))),
            (left_val, right_val) => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compute power for incompatible types: {:?} ^ {:?}", left_val, right_val),
            }),
        }
    }

    // Comparison operation implementations
    async fn evaluate_equal(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let result = match (left, right) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a == b,
            (FieldValue::Number(a), FieldValue::Number(b)) => a == b,
            (FieldValue::String(a), FieldValue::String(b)) => a == b,
            (FieldValue::Boolean(a), FieldValue::Boolean(b)) => a == b,
            (FieldValue::Null, FieldValue::Null) => true,
            _ => false, // Different types are not equal
        };
        Ok(FieldValue::Boolean(result))
    }

    async fn evaluate_not_equal(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let equal_result = self.evaluate_equal(left, right).await?;
        if let FieldValue::Boolean(b) = equal_result {
            Ok(FieldValue::Boolean(!b))
        } else {
            Err(ExpressionEvaluationError::TypeError {
                reason: "Equal operation did not return boolean".to_string(),
            })
        }
    }

    async fn evaluate_less_than(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let result = match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a < b,
            (FieldValue::Number(a), FieldValue::Number(b)) => a < b,
            (FieldValue::Integer(a), FieldValue::Number(b)) => (a as f64) < b,
            (FieldValue::Number(a), FieldValue::Integer(b)) => a < (b as f64),
            (FieldValue::String(a), FieldValue::String(b)) => a < b,
            (left_val, right_val) => return Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compare incompatible types: {:?} < {:?}", left_val, right_val),
            }),
        };
        Ok(FieldValue::Boolean(result))
    }

    async fn evaluate_less_than_or_equal(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let result = match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a <= b,
            (FieldValue::Number(a), FieldValue::Number(b)) => a <= b,
            (FieldValue::Integer(a), FieldValue::Number(b)) => (a as f64) <= b,
            (FieldValue::Number(a), FieldValue::Integer(b)) => a <= (b as f64),
            (FieldValue::String(a), FieldValue::String(b)) => a <= b,
            (left_val, right_val) => return Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compare incompatible types: {:?} <= {:?}", left_val, right_val),
            }),
        };
        Ok(FieldValue::Boolean(result))
    }

    async fn evaluate_greater_than(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let result = match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a > b,
            (FieldValue::Number(a), FieldValue::Number(b)) => a > b,
            (FieldValue::Integer(a), FieldValue::Number(b)) => (a as f64) > b,
            (FieldValue::Number(a), FieldValue::Integer(b)) => a > (b as f64),
            (FieldValue::String(a), FieldValue::String(b)) => a > b,
            (left_val, right_val) => return Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compare incompatible types: {:?} > {:?}", left_val, right_val),
            }),
        };
        Ok(FieldValue::Boolean(result))
    }

    async fn evaluate_greater_than_or_equal(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let result = match (left.clone(), right.clone()) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a >= b,
            (FieldValue::Number(a), FieldValue::Number(b)) => a >= b,
            (FieldValue::Integer(a), FieldValue::Number(b)) => (a as f64) >= b,
            (FieldValue::Number(a), FieldValue::Integer(b)) => a >= (b as f64),
            (FieldValue::String(a), FieldValue::String(b)) => a >= b,
            (left_val, right_val) => return Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot compare incompatible types: {:?} >= {:?}", left_val, right_val),
            }),
        };
        Ok(FieldValue::Boolean(result))
    }

    // Logical operation implementations
    async fn evaluate_and(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let left_bool = self.is_truthy(&left);
        if !left_bool {
            return Ok(FieldValue::Boolean(false));
        }
        let right_bool = self.is_truthy(&right);
        Ok(FieldValue::Boolean(left_bool && right_bool))
    }

    async fn evaluate_or(&self, left: FieldValue, right: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let left_bool = self.is_truthy(&left);
        if left_bool {
            return Ok(FieldValue::Boolean(true));
        }
        let right_bool = self.is_truthy(&right);
        Ok(FieldValue::Boolean(left_bool || right_bool))
    }

    async fn evaluate_negate(&self, operand: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        match operand {
            FieldValue::Integer(i) => Ok(FieldValue::Integer(-i)),
            FieldValue::Number(n) => Ok(FieldValue::Number(-n)),
            _ => Err(ExpressionEvaluationError::TypeError {
                reason: format!("Cannot negate non-numeric type: {:?}", operand),
            }),
        }
    }

    async fn evaluate_not(&self, operand: FieldValue) -> Result<FieldValue, ExpressionEvaluationError> {
        let truthy = self.is_truthy(&operand);
        Ok(FieldValue::Boolean(!truthy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_evaluator() -> ExpressionEvaluator<'static> {
        let registry = Box::leak(Box::new(FunctionRegistry::with_built_ins()));
        let context = Box::leak(Box::new(HashMap::new()));
        ExpressionEvaluator::new(registry, context)
    }

    fn create_test_evaluator_with_context(context: HashMap<String, FieldValue>) -> ExpressionEvaluator<'static> {
        let registry = Box::leak(Box::new(FunctionRegistry::with_built_ins()));
        let context = Box::leak(Box::new(context));
        ExpressionEvaluator::new(registry, context)
    }

    #[tokio::test]
    async fn test_evaluate_simple_literals() {
        let evaluator = create_test_evaluator();

        // Test number literals
        let result = evaluator.evaluate_expression("42").await.unwrap();
        assert_eq!(result, FieldValue::Number(42.0));

        let result = evaluator.evaluate_expression("3.14").await.unwrap();
        assert_eq!(result, FieldValue::Number(3.14)); // 3.14159 - 0.00186 = 3.14

        // Test string literals
        let result = evaluator.evaluate_expression("\"hello\"").await.unwrap();
        assert_eq!(result, FieldValue::String("hello".to_string()));

        // Test boolean literals
        let result = evaluator.evaluate_expression("true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test null literal
        let result = evaluator.evaluate_expression("null").await.unwrap();
        assert_eq!(result, FieldValue::Null);
    }

    #[tokio::test]
    async fn test_arithmetic_operators() {
        let evaluator = create_test_evaluator();

        // Test addition
        let result = evaluator.evaluate_expression("1 + 2").await.unwrap();
        assert_eq!(result, FieldValue::Number(3.0));

        let result = evaluator.evaluate_expression("1.5 + 2.5").await.unwrap();
        assert_eq!(result, FieldValue::Number(4.0));

        let result = evaluator.evaluate_expression("1 + 2.5").await.unwrap();
        assert_eq!(result, FieldValue::Number(3.5));

        // Test subtraction
        let result = evaluator.evaluate_expression("5 - 3").await.unwrap();
        assert_eq!(result, FieldValue::Number(2.0));

        // Test multiplication
        let result = evaluator.evaluate_expression("3 * 4").await.unwrap();
        assert_eq!(result, FieldValue::Number(12.0));

        // Test division
        let result = evaluator.evaluate_expression("8 / 2").await.unwrap();
        assert_eq!(result, FieldValue::Number(4.0));

        // Test modulo
        let result = evaluator.evaluate_expression("7 % 3").await.unwrap();
        assert_eq!(result, FieldValue::Number(1.0));

        let result = evaluator.evaluate_expression("7.5 % 2.5").await.unwrap();
        assert_eq!(result, FieldValue::Number(0.0));

        // Test power
        let result = evaluator.evaluate_expression("2 ^ 3").await.unwrap();
        assert_eq!(result, FieldValue::Number(8.0));

        // Test division by zero
        let result = evaluator.evaluate_expression("1 / 0").await;
        assert!(result.is_err());
        if let Err(ExpressionEvaluationError::DivisionByZero) = result {
            // Expected
        } else {
            panic!("Expected DivisionByZero error");
        }
    }

    #[tokio::test]
    async fn test_comparison_operators() {
        let evaluator = create_test_evaluator();

        // Test equality
        let result = evaluator.evaluate_expression("5 == 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("5 == 6").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        let result = evaluator.evaluate_expression("\"hello\" == \"hello\"").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        // Test inequality
        let result = evaluator.evaluate_expression("5 != 6").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("5 != 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test less than
        let result = evaluator.evaluate_expression("3 < 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("5 < 3").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test less than or equal
        let result = evaluator.evaluate_expression("3 <= 3").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("3 <= 2").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test greater than
        let result = evaluator.evaluate_expression("5 > 3").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("3 > 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test greater than or equal
        let result = evaluator.evaluate_expression("5 >= 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("3 >= 5").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));
    }

    #[tokio::test]
    async fn test_logical_operators() {
        let evaluator = create_test_evaluator();

        // Test logical AND
        let result = evaluator.evaluate_expression("true && true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("true && false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        let result = evaluator.evaluate_expression("false && true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        let result = evaluator.evaluate_expression("false && false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test logical OR
        let result = evaluator.evaluate_expression("true || true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("true || false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("false || true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("false || false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        // Test logical NOT
        let result = evaluator.evaluate_expression("!true").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        let result = evaluator.evaluate_expression("!false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        // Test truthy evaluation with non-boolean types
        let result = evaluator.evaluate_expression("1 && 2").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("0 && 1").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));

        let result = evaluator.evaluate_expression("\"\" || \"hello\"").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));
    }

    #[tokio::test]
    async fn test_string_concatenation() {
        let evaluator = create_test_evaluator();

        // Test string concatenation
        let result = evaluator.evaluate_expression("\"hello\" + \" \" + \"world\"").await.unwrap();
        assert_eq!(result, FieldValue::String("hello world".to_string()));

        // Test mixed type concatenation
        let result = evaluator.evaluate_expression("\"age: \" + 25").await.unwrap();
        assert_eq!(result, FieldValue::String("age: 25".to_string()));
    }

    #[tokio::test]
    async fn test_variable_resolution() {
        let mut context = HashMap::new();
        context.insert("name".to_string(), FieldValue::String("Alice".to_string()));
        context.insert("age".to_string(), FieldValue::Integer(30));
        context.insert("price".to_string(), FieldValue::Number(99.99));

        let evaluator = create_test_evaluator_with_context(context);

        // Test variable resolution
        let result = evaluator.evaluate_expression("name").await.unwrap();
        assert_eq!(result, FieldValue::String("Alice".to_string()));

        let result = evaluator.evaluate_expression("age").await.unwrap();
        assert_eq!(result, FieldValue::Integer(30));

        let result = evaluator.evaluate_expression("price").await.unwrap();
        assert_eq!(result, FieldValue::Number(99.99));

        // Test undefined variable
        let result = evaluator.evaluate_expression("undefined_var").await;
        assert!(result.is_err());
        if let Err(ExpressionEvaluationError::VariableNotFound { name }) = result {
            assert_eq!(name, "undefined_var");
        } else {
            panic!("Expected VariableNotFound error");
        }
    }

    #[tokio::test]
    async fn test_field_access() {
        let mut context = HashMap::new();

        // Create a nested object
        let mut person = HashMap::new();
        person.insert("name".to_string(), FieldValue::String("Alice".to_string()));
        person.insert("age".to_string(), FieldValue::Integer(30));

        let mut address = HashMap::new();
        address.insert("street".to_string(), FieldValue::String("123 Main St".to_string()));
        address.insert("city".to_string(), FieldValue::String("Anytown".to_string()));
        person.insert("address".to_string(), FieldValue::Object(address));

        context.insert("person".to_string(), FieldValue::Object(person));

        // Create an array for array access testing
        let scores = vec![
            FieldValue::Integer(85),
            FieldValue::Integer(92),
            FieldValue::Integer(78)
        ];
        context.insert("scores".to_string(), FieldValue::Array(scores));

        let evaluator = create_test_evaluator_with_context(context);

        // Test object field access
        let result = evaluator.evaluate_expression("person.name").await.unwrap();
        assert_eq!(result, FieldValue::String("Alice".to_string()));

        let result = evaluator.evaluate_expression("person.age").await.unwrap();
        assert_eq!(result, FieldValue::Integer(30));

        let result = evaluator.evaluate_expression("person.address.city").await.unwrap();
        assert_eq!(result, FieldValue::String("Anytown".to_string()));

        // Test array index access
        let result = evaluator.evaluate_expression("scores.0").await.unwrap();
        assert_eq!(result, FieldValue::Integer(85));

        let result = evaluator.evaluate_expression("scores.2").await.unwrap();
        assert_eq!(result, FieldValue::Integer(78));

        // Test non-existent field
        let result = evaluator.evaluate_expression("person.nonexistent").await;
        assert!(result.is_err());
        if let Err(ExpressionEvaluationError::FieldNotFound { field }) = result {
            assert_eq!(field, "nonexistent");
        } else {
            panic!("Expected FieldNotFound error");
        }

        // Test array index out of bounds
        let result = evaluator.evaluate_expression("scores.10").await;
        assert!(result.is_err());
        if let Err(ExpressionEvaluationError::InvalidFieldAccess { reason }) = result {
            assert!(reason.contains("out of bounds"));
        } else {
            panic!("Expected InvalidFieldAccess error");
        }
    }

    #[tokio::test]
    async fn test_function_calls() {
        let evaluator = create_test_evaluator();

        // Test built-in function calls
        let result = evaluator.evaluate_expression("uppercase(\"hello\")").await.unwrap();
        assert_eq!(result, FieldValue::String("HELLO".to_string()));

        let result = evaluator.evaluate_expression("lowercase(\"WORLD\")").await.unwrap();
        assert_eq!(result, FieldValue::String("world".to_string()));

        let result = evaluator.evaluate_expression("length(\"hello\")").await.unwrap();
        assert_eq!(result, FieldValue::Integer(5));

        let result = evaluator.evaluate_expression("concat([\"a\", \"b\", \"c\"])").await.unwrap();
        assert_eq!(result, FieldValue::String("abc".to_string()));

        let result = evaluator.evaluate_expression("sum([1, 2, 3, 4])").await.unwrap();
        assert_eq!(result, FieldValue::Number(10.0));

        // Test function with wrong number of arguments
        let result = evaluator.evaluate_expression("uppercase()").await;
        assert!(result.is_err());
        if let Err(ExpressionEvaluationError::FunctionNotFound { name }) = result {
            assert!(name.contains("uppercase"));
        } else {
            panic!("Expected FunctionNotFound error");
        }
    }

    #[tokio::test]
    async fn test_operator_precedence() {
        let evaluator = create_test_evaluator();

        // Test precedence: multiplication before addition
        let result = evaluator.evaluate_expression("2 + 3 * 4").await.unwrap();
        assert_eq!(result, FieldValue::Number(14.0)); // 3 * 4 = 12, + 2 = 14

        // Test precedence: parentheses override precedence
        let result = evaluator.evaluate_expression("(2 + 3) * 4").await.unwrap();
        assert_eq!(result, FieldValue::Number(20.0)); // 2 + 3 = 5, * 4 = 20

        let result = evaluator.evaluate_expression("(true || false) && false").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false)); // parentheses override

        // Test complex expression
        let result = evaluator.evaluate_expression("2 + 3 * 4 == 14").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(true));

        let result = evaluator.evaluate_expression("2 + 3 * 4 == 15").await.unwrap();
        assert_eq!(result, FieldValue::Boolean(false));
    }

    #[tokio::test]
    async fn test_complex_expressions() {
        let evaluator = create_test_evaluator();

        // Test complex arithmetic expression
        let result = evaluator.evaluate_expression("2 * (3 + 4) / 2").await.unwrap();
        assert_eq!(result, FieldValue::Number(7.0)); // 2 * 7 / 2 = 14 / 2 = 7

        // Test complex expression with multiple operators
        let result = evaluator.evaluate_expression("1 + 2 * 3 - 4 / 2 + 1").await.unwrap();
        assert_eq!(result, FieldValue::Number(6.0)); // 1 + 6 - 2 + 1 = 6

        // Test expression with function calls
        let result = evaluator.evaluate_expression("length(\"hello\") + 5").await.unwrap();
        assert_eq!(result, FieldValue::Number(10.0));

        // Test expression with field access (mock context)
        let mut context = HashMap::new();
        context.insert("x".to_string(), FieldValue::Integer(10));
        context.insert("y".to_string(), FieldValue::Integer(20));

        let evaluator = create_test_evaluator_with_context(context);

        let result = evaluator.evaluate_expression("x + y * 2").await.unwrap();
        assert_eq!(result, FieldValue::Number(50.0)); // 10 + 20 * 2 = 50
    }

    #[tokio::test]
    async fn test_error_handling() {
        let evaluator = create_test_evaluator();

        // Test invalid syntax
        let result = evaluator.evaluate_expression("2 + ").await;
        assert!(result.is_err());

        // Test invalid operator usage
        let result = evaluator.evaluate_expression("2 + null").await;
        assert!(result.is_err());

        // Test invalid field access
        let result = evaluator.evaluate_expression("null.field").await;
        assert!(result.is_err());

        // Test invalid array access
        let result = evaluator.evaluate_expression("\"string\".0").await;
        assert!(result.is_err());

        // Test function call with wrong arguments
        let result = evaluator.evaluate_expression("uppercase(123, 456)").await;
        assert!(result.is_err());
    }
}
