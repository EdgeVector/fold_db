use std::collections::HashMap;

use super::super::ast::Value;

mod conversions;
mod math;
mod strings;

pub use conversions::conversion_functions;
pub use math::math_functions;
pub use strings::string_functions;

/// Type for function implementations in the interpreter
pub type TransformFunction = Box<dyn Fn(Vec<Value>) -> Result<Value, String>>;

/// Returns the default set of built-in functions for the interpreter.
pub fn builtin_functions() -> HashMap<String, TransformFunction> {
    let mut functions: HashMap<String, TransformFunction> = HashMap::new();
    functions.extend(math::math_functions());
    functions.extend(strings::string_functions());
    functions.extend(conversions::conversion_functions());
    functions
}
