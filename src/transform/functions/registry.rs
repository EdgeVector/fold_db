//! Function registry for transform operations
//!
//! Defines traits for iterator and reducer functions and provides a global
//! registry mapping function names to their metadata and executors.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::schema::types::field::FieldValue;
use crate::transform::iterator_stack_typed::types::IterationItem;

/// Result type for iterator function execution
pub type IteratorResult = Vec<IterationItem>;

/// Result type for reducer function execution
pub type ReducerResult = String;

/// Metadata about a registered function
#[derive(Clone, Debug)]
pub struct FunctionMetadata {
    pub name: String,
    pub function_type: FunctionType,
    pub description: String,
}

/// Type of function
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionType {
    Iterator,
    Reducer,
}

/// Result of executing an iterator function
#[derive(Clone, Debug)]
pub enum IteratorExecutionResult {
    /// Returns new iteration items (normal case)
    Items(Vec<IterationItem>),
    /// Returns text tokens (for split_by_word)
    TextTokens(Vec<String>),
}

/// Trait for iterator functions that expand items into multiple items
pub trait IteratorFunction: Send + Sync {
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult;
    fn metadata(&self) -> FunctionMetadata;
}

/// Trait for reducer functions that aggregate multiple items into one
pub trait ReducerFunction: Send + Sync {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult;
    fn metadata(&self) -> FunctionMetadata;
}

/// Generates a reducer struct with ReducerFunction impl from a name, description, and body.
macro_rules! define_reducer {
    ($struct_name:ident, $name:expr, $desc:expr, |$items:ident| $body:expr) => {
        struct $struct_name;
        impl ReducerFunction for $struct_name {
            fn execute(&self, $items: &[IterationItem]) -> ReducerResult { $body }
            fn metadata(&self) -> FunctionMetadata {
                FunctionMetadata {
                    name: $name.to_string(),
                    function_type: FunctionType::Reducer,
                    description: $desc.to_string(),
                }
            }
        }
    };
}

/// Global function registry
pub struct FunctionRegistry {
    iterators: HashMap<String, Box<dyn IteratorFunction>>,
    reducers: HashMap<String, Box<dyn ReducerFunction>>,
}

impl FunctionRegistry {
    fn new() -> Self {
        let mut registry = Self {
            iterators: HashMap::new(),
            reducers: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.register_iterator(Box::new(SplitByWordFunction));
        self.register_iterator(Box::new(SplitArrayFunction));
        self.register_reducer(Box::new(FirstReducer));
        self.register_reducer(Box::new(LastReducer));
        self.register_reducer(Box::new(CountReducer));
        self.register_reducer(Box::new(JoinReducer));
        self.register_reducer(Box::new(SumReducer));
        self.register_reducer(Box::new(AverageReducer));
        self.register_reducer(Box::new(MaxReducer));
        self.register_reducer(Box::new(MinReducer));
    }

    pub fn register_iterator(&mut self, func: Box<dyn IteratorFunction>) {
        let name = func.metadata().name.clone();
        self.iterators.insert(name, func);
    }

    pub fn register_reducer(&mut self, func: Box<dyn ReducerFunction>) {
        let name = func.metadata().name.clone();
        self.reducers.insert(name, func);
    }

    pub fn get_iterator(&self, name: &str) -> Option<&dyn IteratorFunction> {
        self.iterators.get(name).map(|b| b.as_ref())
    }

    pub fn get_reducer(&self, name: &str) -> Option<&dyn ReducerFunction> {
        self.reducers.get(name).map(|b| b.as_ref())
    }

    pub fn is_iterator(&self, name: &str) -> bool {
        self.iterators.contains_key(name)
    }

    pub fn is_reducer(&self, name: &str) -> bool {
        self.reducers.contains_key(name)
    }

    pub fn is_registered(&self, name: &str) -> bool {
        self.is_iterator(name) || self.is_reducer(name)
    }

    pub fn get_function_type(&self, name: &str) -> Option<FunctionType> {
        if self.is_iterator(name) {
            Some(FunctionType::Iterator)
        } else if self.is_reducer(name) {
            Some(FunctionType::Reducer)
        } else {
            None
        }
    }

    pub fn iterator_names(&self) -> Vec<String> {
        self.iterators.keys().cloned().collect()
    }

    pub fn reducer_names(&self) -> Vec<String> {
        self.reducers.keys().cloned().collect()
    }
}

/// Global registry instance
static REGISTRY: OnceLock<FunctionRegistry> = OnceLock::new();

/// Get the global function registry
pub fn registry() -> &'static FunctionRegistry {
    REGISTRY.get_or_init(FunctionRegistry::new)
}

// ============================================================================
// Built-in Iterator Functions
// ============================================================================

struct SplitByWordFunction;

impl IteratorFunction for SplitByWordFunction {
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult {
        let words: Vec<String> = extract_text_value(&item.value)
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        IteratorExecutionResult::TextTokens(words)
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "split_by_word".to_string(),
            function_type: FunctionType::Iterator,
            description: "Split text into individual words".to_string(),
        }
    }
}

struct SplitArrayFunction;

impl IteratorFunction for SplitArrayFunction {
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult {
        match &item.value.value {
            serde_json::Value::Array(arr) => {
                let items = arr
                    .iter()
                    .map(|val| {
                        let mut new_item = item.clone();
                        new_item.value.value = val.clone();
                        new_item
                    })
                    .collect();
                IteratorExecutionResult::Items(items)
            }
            _ => IteratorExecutionResult::Items(vec![item.clone()]),
        }
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "split_array".to_string(),
            function_type: FunctionType::Iterator,
            description: "Split an array into individual elements".to_string(),
        }
    }
}

// ============================================================================
// Built-in Reducer Functions
// ============================================================================

define_reducer!(FirstReducer, "first", "Return the first item", |items| {
    sorted_items(items).first().map(|item| extract_text_value(&item.value)).unwrap_or_default()
});

define_reducer!(LastReducer, "last", "Return the last item", |items| {
    sorted_items(items).last().map(|item| extract_text_value(&item.value)).unwrap_or_default()
});

define_reducer!(CountReducer, "count", "Count the number of items", |items| {
    items.len().to_string()
});

define_reducer!(JoinReducer, "join", "Join items into a comma-separated string", |items| {
    sorted_items(items).into_iter().map(|item| extract_text_value(&item.value)).collect::<Vec<_>>().join(", ")
});

define_reducer!(SumReducer, "sum", "Sum numeric values", |items| {
    format_number(extract_numbers(items).sum())
});

define_reducer!(AverageReducer, "average", "Calculate average of numeric values", |items| {
    let nums: Vec<f64> = extract_numbers(items).collect();
    if nums.is_empty() { return "0".to_string(); }
    format_number(nums.iter().sum::<f64>() / nums.len() as f64)
});

define_reducer!(MaxReducer, "max", "Find maximum numeric value", |items| {
    extract_numbers(items)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|v| v.to_string())
        .unwrap_or_default()
});

define_reducer!(MinReducer, "min", "Find minimum numeric value", |items| {
    extract_numbers(items)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|v| v.to_string())
        .unwrap_or_default()
});

// ============================================================================
// Helper Functions
// ============================================================================

fn json_scalar_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn extract_text_value(field_value: &FieldValue) -> String {
    match &field_value.value {
        serde_json::Value::Object(map) => {
            map.get("value").map(json_scalar_to_string).unwrap_or_default()
        }
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.get("value"))
            .map(json_scalar_to_string)
            .unwrap_or_default(),
        serde_json::Value::Null => String::new(),
        other => json_scalar_to_string(other),
    }
}

fn extract_numbers(items: &[IterationItem]) -> impl Iterator<Item = f64> + '_ {
    items.iter().filter_map(|item| match &item.value.value {
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    })
}

fn format_number(v: f64) -> String {
    if v.abs() < f64::EPSILON {
        return "0".to_string();
    }
    let s = v.to_string();
    if s.ends_with(".0") { s[..s.len() - 2].to_string() } else { s }
}

fn sorted_items(items: &[IterationItem]) -> Vec<&IterationItem> {
    let mut sorted: Vec<&IterationItem> = items.iter().collect();
    sorted.sort_by(|a, b| {
        let key = |item: &IterationItem| (
            item.key.hash.as_deref().unwrap_or("").to_string(),
            item.key.range.as_deref().unwrap_or("").to_string(),
        );
        key(a).cmp(&key(b))
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::key_value::KeyValue;

    fn make_item(value: serde_json::Value) -> IterationItem {
        IterationItem {
            key: KeyValue::new(Some("test".to_string()), None),
            value: FieldValue {
                value,
                atom_uuid: "test-uuid".to_string(),
                source_file_name: None,
                metadata: None,
                molecule_uuid: None,
                molecule_version: None,
            },
            is_text_token: false,
        }
    }

    fn text_item(s: &str) -> IterationItem {
        make_item(serde_json::Value::String(s.to_string()))
    }

    fn num_item(n: f64) -> IterationItem {
        make_item(serde_json::Value::Number(
            serde_json::Number::from_f64(n).expect("finite number"),
        ))
    }

    #[test]
    fn test_average_reducer() {
        let reducer = registry().get_reducer("average").unwrap();
        assert_eq!(reducer.execute(&[num_item(10.0), num_item(20.0), num_item(30.0)]), "20");
        assert_eq!(reducer.execute(&[num_item(10.5), num_item(20.5)]), "15.5");
        assert_eq!(reducer.execute(&[]), "0");
    }

    #[test]
    fn test_registry_initialization() {
        let reg = registry();
        assert!(reg.is_iterator("split_by_word"));
        assert!(reg.is_iterator("split_array"));
        for name in ["first", "last", "count", "join", "sum", "max", "min"] {
            assert!(reg.is_reducer(name));
        }
        assert!(!reg.is_registered("unknown_function"));
    }

    #[test]
    fn test_function_type_detection() {
        let reg = registry();
        assert_eq!(reg.get_function_type("split_by_word"), Some(FunctionType::Iterator));
        assert_eq!(reg.get_function_type("first"), Some(FunctionType::Reducer));
        assert_eq!(reg.get_function_type("unknown"), None);
    }

    #[test]
    fn test_count_reducer() {
        let reducer = registry().get_reducer("count").unwrap();
        assert_eq!(reducer.execute(&[text_item("one"), text_item("two"), text_item("three")]), "3");
    }

    #[test]
    fn test_join_reducer() {
        let reducer = registry().get_reducer("join").unwrap();
        assert_eq!(reducer.execute(&[text_item("hello"), text_item("world")]), "hello, world");
    }

    #[test]
    fn test_sum_reducer() {
        let reducer = registry().get_reducer("sum").unwrap();
        assert_eq!(reducer.execute(&[num_item(10.0), num_item(20.0)]), "30");
    }

    #[test]
    fn test_split_by_word_execution() {
        let func = registry().get_iterator("split_by_word").unwrap();
        let result = func.execute(&text_item("hello world test"));
        match result {
            IteratorExecutionResult::TextTokens(tokens) => {
                assert_eq!(tokens, vec!["hello", "world", "test"]);
            }
            _ => panic!("Expected TextTokens result"),
        }
        let meta = func.metadata();
        assert_eq!(meta.name, "split_by_word");
        assert_eq!(meta.function_type, FunctionType::Iterator);
    }
}
