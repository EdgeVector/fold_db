//! Interpreter for the transform DSL.
//!
//! This module implements an interpreter for the transform DSL.
//! It evaluates an AST to produce a result value.

use super::ast::{Expression, Operator, UnaryOperator, Value};
use crate::schema::types::SchemaError;
use std::collections::HashMap;

pub mod builtins;
pub use builtins::{builtin_functions, TransformFunction};

/// Interpreter for the transform DSL.
pub struct Interpreter {
    /// Variables in the current scope
    variables: HashMap<String, Value>,

    /// Built-in functions
    functions: HashMap<String, TransformFunction>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            functions: builtin_functions(),
        }
    }
}

impl Interpreter {
    /// Creates a new interpreter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new interpreter with the given variables.
    pub fn with_variables(variables: HashMap<String, Value>) -> Self {
        let mut interpreter = Self::new();
        interpreter.variables = variables;
        interpreter
    }

    /// Evaluates an expression.
    pub fn evaluate(&mut self, expr: &Expression) -> Result<Value, SchemaError> {
        match expr {
            Expression::Literal(value) => self.evaluate_literal(value),
            Expression::Variable(name) => self.evaluate_variable(name),
            Expression::FieldAccess { object, field } => self.evaluate_field_access(object, field),
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => self.evaluate_binary_op(left, operator, right),
            Expression::UnaryOp { operator, expr } => self.evaluate_unary_op(operator, expr),
            Expression::FunctionCall { name, args } => self.evaluate_function_call(name, args),
            Expression::IfElse {
                condition,
                then_branch,
                else_branch,
            } => self.evaluate_if_else(condition, then_branch, else_branch),
            Expression::LetBinding { name, value, body } => {
                self.evaluate_let_binding(name, value, body)
            }
            Expression::Return(expr) => self.evaluate_return(expr),
        }
    }

    /// Evaluates a literal value.
    fn evaluate_literal(&self, value: &Value) -> Result<Value, SchemaError> {
        Ok(value.clone())
    }

    /// Evaluates a variable reference.
    fn evaluate_variable(&self, name: &str) -> Result<Value, SchemaError> {
        self.variables
            .get(name)
            .cloned()
            .ok_or_else(|| SchemaError::InvalidField(format!("Variable not found: {}", name)))
    }

    /// Evaluates field access expressions (object.field).
    fn evaluate_field_access(
        &mut self,
        object: &Expression,
        field: &str,
    ) -> Result<Value, SchemaError> {
        // Handle schema.field references
        if let Expression::Variable(schema_name) = object {
            return self.evaluate_schema_field_access(schema_name, field);
        }

        // Handle regular object field access
        let obj = self.evaluate(object)?;
        match obj {
            Value::Object(map) => {
                if let Some(value) = map.get(field) {
                    Ok(Value::from(value.clone()))
                } else {
                    Err(SchemaError::InvalidField(format!(
                        "Field not found: {}",
                        field
                    )))
                }
            }
            _ => Err(SchemaError::InvalidField(format!(
                "Cannot access field {} on non-object value",
                field
            ))),
        }
    }

    /// Evaluates schema.field access patterns.
    fn evaluate_schema_field_access(
        &self,
        schema_name: &str,
        field: &str,
    ) -> Result<Value, SchemaError> {
        // Look up schema.field in variables
        let key = format!("{}.{}", schema_name, field);
        if let Some(value) = self.variables.get(&key) {
            return Ok(value.clone());
        }

        // Check if the schema name is a variable containing an object
        if let Some(Value::Object(map)) = self.variables.get(schema_name) {
            if let Some(value) = map.get(field) {
                return Ok(Value::from(value.clone()));
            }
        }

        // Fall back to looking up just the field name
        self.variables
            .get(field)
            .cloned()
            .ok_or_else(|| SchemaError::InvalidField(format!("Field not found: {}", field)))
    }

    /// Evaluates binary operations.
    fn evaluate_binary_op(
        &mut self,
        left: &Expression,
        operator: &Operator,
        right: &Expression,
    ) -> Result<Value, SchemaError> {
        let left_val = self.evaluate(left)?;
        let right_val = self.evaluate(right)?;

        match operator {
            Operator::Add => self.add(&left_val, &right_val),
            Operator::Subtract => self.subtract(&left_val, &right_val),
            Operator::Multiply => self.multiply(&left_val, &right_val),
            Operator::Divide => self.divide(&left_val, &right_val),
            Operator::Power => self.power(&left_val, &right_val),
            Operator::Equal => self.equal(&left_val, &right_val),
            Operator::NotEqual => self.not_equal(&left_val, &right_val),
            Operator::LessThan => self.less_than(&left_val, &right_val),
            Operator::LessThanOrEqual => self.less_than_or_equal(&left_val, &right_val),
            Operator::GreaterThan => self.greater_than(&left_val, &right_val),
            Operator::GreaterThanOrEqual => self.greater_than_or_equal(&left_val, &right_val),
            Operator::And => self.and(&left_val, &right_val),
            Operator::Or => self.or(&left_val, &right_val),
        }
    }

    /// Evaluates unary operations.
    fn evaluate_unary_op(
        &mut self,
        operator: &UnaryOperator,
        expr: &Expression,
    ) -> Result<Value, SchemaError> {
        let val = self.evaluate(expr)?;

        match operator {
            UnaryOperator::Negate => self.negate(&val),
            UnaryOperator::Not => self.not(&val),
        }
    }

    /// Evaluates function calls.
    fn evaluate_function_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<Value, SchemaError> {
        let mut evaluated_args = Vec::new();

        for arg in args {
            evaluated_args.push(self.evaluate(arg)?);
        }

        if let Some(func) = self.functions.get(name) {
            func(evaluated_args).map_err(SchemaError::InvalidField)
        } else {
            Err(SchemaError::InvalidField(format!(
                "Function not found: {}",
                name
            )))
        }
    }

    /// Evaluates if-else conditional expressions.
    fn evaluate_if_else(
        &mut self,
        condition: &Expression,
        then_branch: &Expression,
        else_branch: &Option<Box<Expression>>,
    ) -> Result<Value, SchemaError> {
        let cond = self.evaluate(condition)?;

        match cond {
            Value::Boolean(true) => self.evaluate(then_branch),
            Value::Boolean(false) => {
                if let Some(else_expr) = else_branch {
                    self.evaluate(else_expr)
                } else {
                    Ok(Value::Null)
                }
            }
            _ => Err(SchemaError::InvalidField(
                "Condition must be a boolean".to_string(),
            )),
        }
    }

    /// Evaluates let binding expressions.
    fn evaluate_let_binding(
        &mut self,
        name: &str,
        value: &Expression,
        body: &Expression,
    ) -> Result<Value, SchemaError> {
        let val = self.evaluate(value)?;

        // Save the old value if it exists
        let _old_value = self.variables.get(name).cloned();

        // Set the new value
        self.variables.insert(name.to_string(), val.clone());

        // If the body is Null, just return the value (for sequential evaluation)
        // Don't restore the old value or remove the variable for sequential evaluation
        // This allows variables to persist between statements in the transform logic
        if let Expression::Literal(Value::Null) = body {
            Ok(val)
        } else {
            // Otherwise evaluate the body
            self.evaluate(body)
        }
    }

    /// Evaluates return expressions.
    fn evaluate_return(&mut self, expr: &Expression) -> Result<Value, SchemaError> {
        self.evaluate(expr)
    }

    // Operator implementations

    fn add(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(a), Value::String(b)) => {
                // Try to parse strings as numbers first
                if let (Ok(num_a), Ok(num_b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                    Ok(Value::Number(num_a + num_b))
                } else {
                    // Fall back to string concatenation
                    Ok(Value::String(format!("{}{}", a, b)))
                }
            }
            (Value::Number(a), Value::String(b)) => {
                if let Ok(num_b) = b.parse::<f64>() {
                    Ok(Value::Number(a + num_b))
                } else {
                    Err(SchemaError::InvalidField(
                        "Cannot add number and non-numeric string".to_string(),
                    ))
                }
            }
            (Value::String(a), Value::Number(b)) => {
                if let Ok(num_a) = a.parse::<f64>() {
                    Ok(Value::Number(num_a + b))
                } else {
                    Err(SchemaError::InvalidField(
                        "Cannot add non-numeric string and number".to_string(),
                    ))
                }
            }
            _ => Err(SchemaError::InvalidField(
                "Cannot add values of these types".to_string(),
            )),
        }
    }

    fn subtract(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot subtract non-numeric values".to_string(),
            )),
        }
    }

    fn multiply(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot multiply non-numeric values".to_string(),
            )),
        }
    }

    fn divide(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if *b == 0.0 {
                    Err(SchemaError::InvalidField("Division by zero".to_string()))
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            _ => Err(SchemaError::InvalidField(
                "Cannot divide non-numeric values".to_string(),
            )),
        }
    }

    fn power(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.powf(*b))),
            _ => Err(SchemaError::InvalidField(
                "Cannot raise non-numeric values to a power".to_string(),
            )),
        }
    }

    fn equal(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a == b)),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a == b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a == b)),
            (Value::Null, Value::Null) => Ok(Value::Boolean(true)),
            _ => Ok(Value::Boolean(false)),
        }
    }

    fn not_equal(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        self.equal(left, right).map(|v| match v {
            Value::Boolean(b) => Value::Boolean(!b),
            _ => unreachable!(),
        })
    }

    fn less_than(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a < b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot compare non-comparable values".to_string(),
            )),
        }
    }

    fn less_than_or_equal(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a <= b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a <= b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot compare non-comparable values".to_string(),
            )),
        }
    }

    fn greater_than(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a > b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot compare non-comparable values".to_string(),
            )),
        }
    }

    fn greater_than_or_equal(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a >= b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a >= b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot compare non-comparable values".to_string(),
            )),
        }
    }

    fn and(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(*a && *b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot perform logical AND on non-boolean values".to_string(),
            )),
        }
    }

    fn or(&self, left: &Value, right: &Value) -> Result<Value, SchemaError> {
        match (left, right) {
            (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(*a || *b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot perform logical OR on non-boolean values".to_string(),
            )),
        }
    }

    fn negate(&self, value: &Value) -> Result<Value, SchemaError> {
        match value {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err(SchemaError::InvalidField(
                "Cannot negate non-numeric value".to_string(),
            )),
        }
    }

    fn not(&self, value: &Value) -> Result<Value, SchemaError> {
        match value {
            Value::Boolean(b) => Ok(Value::Boolean(!b)),
            _ => Err(SchemaError::InvalidField(
                "Cannot perform logical NOT on non-boolean value".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value as JsonValue;

    #[test]
    fn test_evaluate_simple_expression() {
        // Create a simple binary expression manually
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Literal(Value::Number(1.0))),
            operator: Operator::Add,
            right: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Literal(Value::Number(2.0))),
                operator: Operator::Multiply,
                right: Box::new(Expression::Literal(Value::Number(3.0))),
            }),
        };

        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => assert_eq!(n, 7.0), // 1 + (2 * 3) = 7
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_evaluate_let_expression() {
        // Create a let expression manually
        let expr = Expression::LetBinding {
            name: "x".to_string(),
            value: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Literal(Value::Number(1.0))),
                operator: Operator::Add,
                right: Box::new(Expression::Literal(Value::Number(2.0))),
            }),
            body: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Variable("x".to_string())),
                operator: Operator::Multiply,
                right: Box::new(Expression::Literal(Value::Number(3.0))),
            }),
        };

        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => assert_eq!(n, 9.0), // (1 + 2) * 3 = 9
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_evaluate_if_expression() {
        // Create an if expression manually
        let expr = Expression::IfElse {
            condition: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Literal(Value::Number(5.0))),
                operator: Operator::GreaterThan,
                right: Box::new(Expression::Literal(Value::Number(0.0))),
            }),
            then_branch: Box::new(Expression::Literal(Value::Number(10.0))),
            else_branch: Some(Box::new(Expression::Literal(Value::Number(20.0)))),
        };

        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => assert_eq!(n, 10.0), // 5 > 0, so result is 10
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_evaluate_function_call() {
        // Create a function call expression manually
        let expr = Expression::FunctionCall {
            name: "min".to_string(),
            args: vec![
                Expression::Literal(Value::Number(5.0)),
                Expression::Literal(Value::Number(3.0)),
            ],
        };

        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => assert_eq!(n, 3.0), // min(5, 3) = 3
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_evaluate_field_access() {
        // Create a field access expression manually
        let expr = Expression::FieldAccess {
            object: Box::new(Expression::Variable("input".to_string())),
            field: "value".to_string(),
        };

        // Create a variable with an object value
        let mut variables = HashMap::new();
        let mut object_map = HashMap::new();
        object_map.insert(
            "value".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(42.0).unwrap()),
        );
        variables.insert("input".to_string(), Value::Object(object_map));

        let mut interpreter = Interpreter::with_variables(variables);
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => assert_eq!(n, 42.0), // input.value = 42
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_evaluate_complex_expression() {
        // Create a complex expression manually
        let expr = Expression::LetBinding {
            name: "bmi".to_string(),
            value: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Variable("weight".to_string())),
                operator: Operator::Divide,
                right: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::Variable("height".to_string())),
                    operator: Operator::Power,
                    right: Box::new(Expression::Literal(Value::Number(2.0))),
                }),
            }),
            body: Box::new(Expression::LetBinding {
                name: "risk".to_string(),
                value: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::BinaryOp {
                        left: Box::new(Expression::Literal(Value::Number(0.5))),
                        operator: Operator::Multiply,
                        right: Box::new(Expression::Variable("blood_pressure".to_string())),
                    }),
                    operator: Operator::Add,
                    right: Box::new(Expression::BinaryOp {
                        left: Box::new(Expression::Literal(Value::Number(1.2))),
                        operator: Operator::Multiply,
                        right: Box::new(Expression::Variable("bmi".to_string())),
                    }),
                }),
                body: Box::new(Expression::FunctionCall {
                    name: "clamp".to_string(),
                    args: vec![
                        Expression::Variable("risk".to_string()),
                        Expression::Literal(Value::Number(0.0)),
                        Expression::Literal(Value::Number(100.0)),
                    ],
                }),
            }),
        };

        // Create variables
        let mut variables = HashMap::new();
        variables.insert("weight".to_string(), Value::Number(70.0)); // 70 kg
        variables.insert("height".to_string(), Value::Number(1.75)); // 1.75 m
        variables.insert("blood_pressure".to_string(), Value::Number(120.0)); // 120 mmHg

        let mut interpreter = Interpreter::with_variables(variables);
        let result = interpreter.evaluate(&expr).unwrap();

        match result {
            Value::Number(n) => {
                // Calculate expected result:
                // bmi = 70 / (1.75^2) = 70 / 3.0625 = 22.857
                // risk = 0.5 * 120 + 1.2 * 22.857 = 60 + 27.428 = 87.428
                // clamp(87.428, 0, 100) = 87.428
                assert!((n - 87.428).abs() < 0.001);
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_builtin_function_registration() {
        let functions = builtin_functions();
        let expected = [
            "min",
            "max",
            "clamp",
            "concat",
            "to_string",
            "to_number",
            "to_boolean",
        ];

        for name in expected.iter() {
            assert!(functions.contains_key(*name), "builtin {} not found", name);
        }
    }
}
