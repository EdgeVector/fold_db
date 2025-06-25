use std::collections::HashMap;

use super::super::super::ast::Value;
use super::TransformFunction;

pub fn math_functions() -> HashMap<String, TransformFunction> {
    let mut functions: HashMap<String, TransformFunction> = HashMap::new();

    functions.insert(
        "min".to_string(),
        Box::new(|args| {
            if args.len() != 2 {
                return Err("min() requires exactly 2 arguments".to_string());
            }

            let a = match &args[0] {
                Value::Number(n) => *n,
                _ => return Err("min() requires numeric arguments".to_string()),
            };

            let b = match &args[1] {
                Value::Number(n) => *n,
                _ => return Err("min() requires numeric arguments".to_string()),
            };

            Ok(Value::Number(a.min(b)))
        }),
    );

    functions.insert(
        "max".to_string(),
        Box::new(|args| {
            if args.len() != 2 {
                return Err("max() requires exactly 2 arguments".to_string());
            }

            let a = match &args[0] {
                Value::Number(n) => *n,
                _ => return Err("max() requires numeric arguments".to_string()),
            };

            let b = match &args[1] {
                Value::Number(n) => *n,
                _ => return Err("max() requires numeric arguments".to_string()),
            };

            Ok(Value::Number(a.max(b)))
        }),
    );

    functions.insert(
        "clamp".to_string(),
        Box::new(|args| {
            if args.len() != 3 {
                return Err("clamp() requires exactly 3 arguments".to_string());
            }

            let value = match &args[0] {
                Value::Number(n) => *n,
                _ => return Err("clamp() requires numeric arguments".to_string()),
            };

            let min = match &args[1] {
                Value::Number(n) => *n,
                _ => return Err("clamp() requires numeric arguments".to_string()),
            };

            let max = match &args[2] {
                Value::Number(n) => *n,
                _ => return Err("clamp() requires numeric arguments".to_string()),
            };

            Ok(Value::Number(value.max(min).min(max)))
        }),
    );

    functions
}

