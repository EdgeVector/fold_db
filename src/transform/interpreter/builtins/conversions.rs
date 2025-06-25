use std::collections::HashMap;

use super::super::super::ast::Value;
use super::TransformFunction;

pub fn conversion_functions() -> HashMap<String, TransformFunction> {
    let mut functions: HashMap<String, TransformFunction> = HashMap::new();

    functions.insert(
        "to_string".to_string(),
        Box::new(|args| {
            if args.len() != 1 {
                return Err("to_string() requires exactly 1 argument".to_string());
            }

            let result = match &args[0] {
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::String(s) => s.clone(),
                Value::Null => "null".to_string(),
                Value::Object(_) => "<object>".to_string(),
                Value::Array(_) => "<array>".to_string(),
            };

            Ok(Value::String(result))
        }),
    );

    functions.insert(
        "to_number".to_string(),
        Box::new(|args| {
            if args.len() != 1 {
                return Err("to_number() requires exactly 1 argument".to_string());
            }

            let result = match &args[0] {
                Value::Number(n) => *n,
                Value::Boolean(b) => if *b { 1.0 } else { 0.0 },
                Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
                Value::Null => 0.0,
                Value::Object(_) => 0.0,
                Value::Array(_) => 0.0,
            };

            Ok(Value::Number(result))
        }),
    );

    functions.insert(
        "to_boolean".to_string(),
        Box::new(|args| {
            if args.len() != 1 {
                return Err("to_boolean() requires exactly 1 argument".to_string());
            }

            let result = match &args[0] {
                Value::Number(n) => *n != 0.0,
                Value::Boolean(b) => *b,
                Value::String(s) => !s.is_empty(),
                Value::Null => false,
                Value::Object(_) => true,
                Value::Array(_) => true,
            };

            Ok(Value::Boolean(result))
        }),
    );

    functions
}

