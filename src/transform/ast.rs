//! Abstract Syntax Tree (AST) definitions for the transform DSL.
//!
//! This module defines the data structures that represent the parsed
//! transform DSL code. The AST is a tree-like structure that captures
//! the hierarchical nature of expressions in the DSL.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;

/// Represents a value in the transform DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A numeric value (floating point)
    Number(f64),
    /// A boolean value
    Boolean(bool),
    /// A string value
    String(String),
    /// A null value
    Null,
    /// A JSON object value
    Object(HashMap<String, JsonValue>),
    /// A JSON array value
    Array(Vec<JsonValue>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Null => write!(f, "null"),
            Value::Object(_) => write!(f, "<object>"),
            Value::Array(_) => write!(f, "<array>"),
        }
    }
}

impl From<Value> for JsonValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Number(n) => JsonValue::Number(
                serde_json::Number::from_f64(n).unwrap_or(serde_json::Number::from(0)),
            ),
            Value::Boolean(b) => JsonValue::Bool(b),
            Value::String(s) => JsonValue::String(s),
            Value::Null => JsonValue::Null,
            Value::Object(o) => {
                let mut map = serde_json::Map::new();
                for (k, v) in o {
                    map.insert(k, v);
                }
                JsonValue::Object(map)
            }
            Value::Array(a) => JsonValue::Array(a),
        }
    }
}

impl From<JsonValue> for Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
            JsonValue::Bool(b) => Value::Boolean(b),
            JsonValue::String(s) => Value::String(s),
            JsonValue::Null => Value::Null,
            JsonValue::Object(o) => {
                let mut map = HashMap::new();
                for (k, v) in o {
                    map.insert(k, v);
                }
                Value::Object(map)
            }
            JsonValue::Array(a) => Value::Array(a),
        }
    }
}

/// Represents a binary operator in the transform DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Subtract,
    /// Multiplication (*)
    Multiply,
    /// Division (/)
    Divide,
    /// Modulo (%)
    Modulo,
    /// Power (^)
    Power,
    /// Equality (==)
    Equal,
    /// Inequality (!=)
    NotEqual,
    /// Less than (<)
    LessThan,
    /// Less than or equal (<=)
    LessThanOrEqual,
    /// Greater than (>)
    GreaterThan,
    /// Greater than or equal (>=)
    GreaterThanOrEqual,
    /// Logical AND (&&)
    And,
    /// Logical OR (||)
    Or,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Add => write!(f, "+"),
            Operator::Subtract => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::Modulo => write!(f, "%"),
            Operator::Power => write!(f, "^"),
            Operator::Equal => write!(f, "=="),
            Operator::NotEqual => write!(f, "!="),
            Operator::LessThan => write!(f, "<"),
            Operator::LessThanOrEqual => write!(f, "<="),
            Operator::GreaterThan => write!(f, ">"),
            Operator::GreaterThanOrEqual => write!(f, ">="),
            Operator::And => write!(f, "&&"),
            Operator::Or => write!(f, "||"),
        }
    }
}

/// Represents a unary operator in the transform DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    /// Negation (-)
    Negate,
    /// Logical NOT (!)
    Not,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOperator::Negate => write!(f, "-"),
            UnaryOperator::Not => write!(f, "!"),
        }
    }
}

/// Represents an expression in the transform DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A literal value
    Literal(Value),

    /// A variable reference
    Variable(String),

    /// A field access expression (e.g., obj.field)
    FieldAccess {
        /// The object being accessed
        object: Box<Expression>,
        /// The field name
        field: String,
    },

    /// A binary operation (e.g., a + b)
    BinaryOp {
        /// The left operand
        left: Box<Expression>,
        /// The operator
        operator: Operator,
        /// The right operand
        right: Box<Expression>,
    },

    /// A unary operation (e.g., -a, !b)
    UnaryOp {
        /// The operator
        operator: UnaryOperator,
        /// The operand
        expr: Box<Expression>,
    },

    /// A function call (e.g., min(a, b))
    FunctionCall {
        /// The function name
        name: String,
        /// The function arguments
        args: Vec<Expression>,
    },

    /// An if-else expression (e.g., if a > b then a else b)
    IfElse {
        /// The condition
        condition: Box<Expression>,
        /// The then branch
        then_branch: Box<Expression>,
        /// The optional else branch
        else_branch: Option<Box<Expression>>,
    },

    /// A let binding (e.g., let x = a + b; x * 2)
    LetBinding {
        /// The variable name
        name: String,
        /// The value to bind
        value: Box<Expression>,
        /// The body expression where the binding is in scope
        body: Box<Expression>,
    },

    /// A return statement
    Return(Box<Expression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(value) => write!(f, "{}", value),
            Expression::Variable(name) => write!(f, "{}", name),
            Expression::FieldAccess { object, field } => write!(f, "{}.{}", object, field),
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => write!(f, "({} {} {})", left, operator, right),
            Expression::UnaryOp { operator, expr } => write!(f, "{}({})", operator, expr),
            Expression::FunctionCall { name, args } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expression::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                write!(f, "if {} then {}", condition, then_branch)?;
                if let Some(else_expr) = else_branch {
                    write!(f, " else {}", else_expr)?;
                }
                Ok(())
            }
            Expression::LetBinding { name, value, body } => {
                write!(f, "let {} = {}; {}", name, value, body)
            }
            Expression::Return(expr) => write!(f, "return {}", expr),
        }
    }
}

/// Represents a transform declaration in the DSL.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformDeclaration {
    /// The name of the transform
    pub name: String,

    /// Whether the transform is reversible
    pub reversible: bool,

    /// The signature for verification
    pub signature: Option<String>,

    /// The transform logic
    pub logic: Vec<Expression>,
}

impl fmt::Display for TransformDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "transform {} {{", self.name)?;
        writeln!(f, "  reversible: {}", self.reversible)?;

        if let Some(sig) = &self.signature {
            writeln!(f, "  signature: {}", sig)?;
        }

        writeln!(f, "  logic: {{")?;
        for expr in &self.logic {
            writeln!(f, "    {}", expr)?;
        }
        writeln!(f, "  }}")?;

        write!(f, "}}")
    }
}

// InputType and OutputType structs have been removed as they are no longer needed
