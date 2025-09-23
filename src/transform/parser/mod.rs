//! Better parser for the transform DSL.
//!
//! This module implements a cleaner, more robust parser for the transform DSL using PEST.
//! It converts a string of DSL code into an Abstract Syntax Tree (AST).

use super::ast::{Expression, Operator, TransformDeclaration, UnaryOperator, Value};
use crate::schema::types::SchemaError;
use pest::iterators::Pair;
use pest::Parser;

mod grammar;
mod helpers;

use grammar::Rule;
pub use grammar::TransformParser;

impl TransformParser {
    /// Creates a new parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses the input into an expression AST.
    pub fn parse_expression(&self, input: &str) -> Result<Expression, SchemaError> {
        // Parse the input using the complete_expr rule
        let pairs = Self::parse(Rule::complete_expr, input)
            .map_err(|e| SchemaError::InvalidField(format!("Parse error: {}", e)))?;

        // Get the expression from the parse result
        let expr_pair = pairs.into_iter().next().ok_or_else(|| {
            SchemaError::InvalidField("No expression found in parse result".to_string())
        })?;

        // Convert the parse tree to an AST
        self.build_ast(expr_pair)
    }

    /// Parses the input into a transform declaration AST.
    pub fn parse_transform(&self, input: &str) -> Result<TransformDeclaration, SchemaError> {
        // Parse the input using the complete_transform rule
        let pairs = Self::parse(Rule::complete_transform, input)
            .map_err(|e| SchemaError::InvalidField(format!("Parse error: {}", e)))?;

        // Get the transform declaration from the parse result
        let transform_pair = pairs.into_iter().next().ok_or_else(|| {
            SchemaError::InvalidField("No transform declaration found in parse result".to_string())
        })?;

        // Convert the parse tree to a TransformDeclaration
        self.build_transform_decl(transform_pair)
    }

    /// Builds an AST from a parse tree.
    fn build_ast(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        match pair.as_rule() {
            Rule::complete_expr => {
                // Get the expression inside the complete_expr
                let expr_pair = pair.into_inner().next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No expression found inside complete_expr".to_string(),
                    )
                })?;
                self.build_ast(expr_pair)
            }
            Rule::expr => {
                // Get the expression inside the expr
                let inner_pair = pair.into_inner().next().ok_or_else(|| {
                    SchemaError::InvalidField("No expression found inside expr".to_string())
                })?;
                self.build_ast(inner_pair)
            }
            Rule::logic_expr => self.parse_logic_expr(pair),
            Rule::comp_expr => self.parse_comp_expr(pair),
            Rule::add_expr => self.parse_add_expr(pair),
            Rule::mul_expr => self.parse_mul_expr(pair),
            Rule::pow_expr => self.parse_pow_expr(pair),
            Rule::unary_expr => self.parse_unary_expr(pair),
            Rule::atom => self.parse_atom(pair),
            _ => Err(SchemaError::InvalidField(format!(
                "Unexpected rule: {:?}",
                pair.as_rule()
            ))),
        }
    }

    /// Parses a logic expression (&&, ||).
    fn parse_logic_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Parse the first comparison expression
        let first = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No first comparison expression found".to_string())
        })?;
        let mut expr = self.build_ast(first)?;

        // Parse any additional logic operations
        while let Some(op_pair) = pairs.next() {
            if op_pair.as_rule() == Rule::logic_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "&&" => Operator::And,
                    "||" => Operator::Or,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown logic operator: {}",
                            op_str
                        )))
                    }
                };

                let right_pair = pairs.next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No right operand found in logic expression".to_string(),
                    )
                })?;
                let right = self.build_ast(right_pair)?;

                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: op,
                    right: Box::new(right),
                };
            }
        }

        Ok(expr)
    }

    /// Parses a comparison expression (==, !=, <, <=, >, >=).
    fn parse_comp_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Parse the first addition expression
        let first = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No first addition expression found".to_string())
        })?;
        let mut expr = self.build_ast(first)?;

        // Parse any additional comparison operations
        while let Some(op_pair) = pairs.next() {
            if op_pair.as_rule() == Rule::comp_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "==" => Operator::Equal,
                    "!=" => Operator::NotEqual,
                    "<" => Operator::LessThan,
                    "<=" => Operator::LessThanOrEqual,
                    ">" => Operator::GreaterThan,
                    ">=" => Operator::GreaterThanOrEqual,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown comparison operator: {}",
                            op_str
                        )))
                    }
                };

                let right_pair = pairs.next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No right operand found in comparison expression".to_string(),
                    )
                })?;
                let right = self.build_ast(right_pair)?;

                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: op,
                    right: Box::new(right),
                };
            }
        }

        Ok(expr)
    }

    /// Parses an addition expression (+, -).
    fn parse_add_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Parse the first multiplication expression
        let first = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No first multiplication expression found".to_string())
        })?;
        let mut expr = self.build_ast(first)?;

        // Parse any additional addition operations
        while let Some(op_pair) = pairs.next() {
            if op_pair.as_rule() == Rule::add_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "+" => Operator::Add,
                    "-" => Operator::Subtract,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown addition operator: {}",
                            op_str
                        )))
                    }
                };

                let right_pair = pairs.next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No right operand found in addition expression".to_string(),
                    )
                })?;
                let right = self.build_ast(right_pair)?;

                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: op,
                    right: Box::new(right),
                };
            }
        }

        Ok(expr)
    }

    /// Parses a multiplication expression (*, /, %).
    fn parse_mul_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Parse the first power expression
        let first = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No first power expression found".to_string())
        })?;
        let mut expr = self.build_ast(first)?;

        // Parse any additional multiplication operations
        while let Some(op_pair) = pairs.next() {
            if op_pair.as_rule() == Rule::mul_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "*" => Operator::Multiply,
                    "/" => Operator::Divide,
                    "%" => Operator::Modulo,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown multiplication operator: {}",
                            op_str
                        )))
                    }
                };

                let right_pair = pairs.next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No right operand found in multiplication expression".to_string(),
                    )
                })?;
                let right = self.build_ast(right_pair)?;

                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: op,
                    right: Box::new(right),
                };
            }
        }

        Ok(expr)
    }

    /// Parses a power expression (^).
    fn parse_pow_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Parse the first unary expression
        let first = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No first unary expression found".to_string())
        })?;
        let mut expr = self.build_ast(first)?;

        // Parse any additional power operations
        while let Some(op_pair) = pairs.next() {
            if op_pair.as_rule() == Rule::pow_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "^" => Operator::Power,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown power operator: {}",
                            op_str
                        )))
                    }
                };

                let right_pair = pairs.next().ok_or_else(|| {
                    SchemaError::InvalidField(
                        "No right operand found in power expression".to_string(),
                    )
                })?;
                let right = self.build_ast(right_pair)?;

                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: op,
                    right: Box::new(right),
                };
            }
        }

        Ok(expr)
    }

    /// Parses a unary expression (-, !).
    fn parse_unary_expr(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut pairs = pair.into_inner();

        // Collect all unary operators
        let mut unary_ops = Vec::new();
        while let Some(op_pair) = pairs.peek() {
            if op_pair.as_rule() == Rule::unary_op {
                let op_str = op_pair.as_str();
                let op = match op_str {
                    "-" => UnaryOperator::Negate,
                    "!" => UnaryOperator::Not,
                    _ => {
                        return Err(SchemaError::InvalidField(format!(
                            "Unknown unary operator: {}",
                            op_str
                        )))
                    }
                };
                unary_ops.push(op);
                pairs.next(); // Consume the operator
            } else {
                break;
            }
        }

        // Parse the atom
        let atom_pair = pairs.next().ok_or_else(|| {
            SchemaError::InvalidField("No atom found in unary expression".to_string())
        })?;
        let mut expr = self.build_ast(atom_pair)?;

        // Apply unary operators in reverse order (right to left)
        for op in unary_ops.into_iter().rev() {
            expr = Expression::UnaryOp {
                operator: op,
                expr: Box::new(expr),
            };
        }

        Ok(expr)
    }

    /// Parses an atom (number, string, boolean, null, function call, field access, identifier, or parenthesized expression).
    fn parse_atom(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let inner = pair.into_inner().next().ok_or_else(|| {
            SchemaError::InvalidField("No inner content found in atom".to_string())
        })?;

        match inner.as_rule() {
            Rule::number => {
                let n = inner
                    .as_str()
                    .parse::<f64>()
                    .map_err(|e| SchemaError::InvalidField(format!("Invalid number: {}", e)))?;
                Ok(Expression::Literal(Value::Number(n)))
            }
            Rule::string => {
                // Remove the surrounding quotes
                let s = inner.as_str();
                let s = &s[1..s.len() - 1];
                Ok(Expression::Literal(Value::String(s.to_string())))
            }
            Rule::boolean => match inner.as_str() {
                "true" => Ok(Expression::Literal(Value::Boolean(true))),
                "false" => Ok(Expression::Literal(Value::Boolean(false))),
                _ => Err(SchemaError::InvalidField(format!(
                    "Invalid boolean: {}",
                    inner.as_str()
                ))),
            },
            Rule::null => Ok(Expression::Literal(Value::Null)),
            Rule::array => self.parse_array(inner),
            Rule::function_call => self.parse_function_call(inner),
            Rule::field_access => self.parse_field_access(inner),
            Rule::identifier => Ok(Expression::Variable(inner.as_str().to_string())),
            Rule::expr => self.build_ast(inner),
            _ => Err(SchemaError::InvalidField(format!(
                "Unexpected rule in atom: {:?}",
                inner.as_rule()
            ))),
        }
    }

    /// Parses an array literal [expr1, expr2, ...]
    fn parse_array(&self, pair: Pair<Rule>) -> Result<Expression, SchemaError> {
        let mut elements = Vec::new();

        for inner_pair in pair.into_inner() {
            if inner_pair.as_rule() == Rule::expr {
                let element = self.build_ast(inner_pair)?;
                elements.push(element);
            }
        }

        Ok(Expression::Literal(Value::Array(
            elements.into_iter().map(|expr| {
                match expr {
                    Expression::Literal(value) => value.into(),
                    _ => serde_json::Value::Null, // Fallback for non-literal expressions in array
                }
            }).collect()
        )))
    }
}

impl Default for TransformParser {
    fn default() -> Self {
        Self::new()
    }
}
