use std::collections::HashMap;

use super::super::super::ast::Value;
use super::TransformFunction;

pub fn string_functions() -> HashMap<String, TransformFunction> {
    let mut functions: HashMap<String, TransformFunction> = HashMap::new();

    functions.insert(
        "concat".to_string(),
        Box::new(|args| {
            let mut result = String::new();

            for arg in args {
                match arg {
                    Value::String(s) => result.push_str(&s),
                    _ => return Err("concat() requires string arguments".to_string()),
                }
            }

            Ok(Value::String(result))
        }),
    );

    functions
}
