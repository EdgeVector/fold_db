//! Function registry for transform operations
//!
//! Defines traits for iterator and reducer functions and provides a global
//! registry mapping function names to their metadata and executors.
//!
//! TODO: Reducer functions are registered but not yet integrated into the execution engine.
//! The engine currently only executes iterator functions. Reducer integration requires:
//! - Chain parser to extract reducer operations from parsed chains
//! - Engine to collect items at appropriate depth and call reducer.execute()
//! - Result handling to convert ReducerResult back to appropriate field values

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
    /// Execute the iterator function on a single item
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult;
    
    /// Get metadata about this function
    fn metadata(&self) -> FunctionMetadata;
}

/// Trait for reducer functions that aggregate multiple items into one
pub trait ReducerFunction: Send + Sync {
    /// Execute the reducer function on a collection of items
    fn execute(&self, items: &[IterationItem]) -> ReducerResult;
    
    /// Get metadata about this function
    fn metadata(&self) -> FunctionMetadata;
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

    /// Register all built-in functions
    fn register_builtins(&mut self) {
        // Register iterator functions
        self.register_iterator(Box::new(SplitByWordFunction));
        self.register_iterator(Box::new(SplitArrayFunction));
        
        // Register reducer functions
        self.register_reducer(Box::new(FirstReducer));
        self.register_reducer(Box::new(LastReducer));
        self.register_reducer(Box::new(CountReducer));
        self.register_reducer(Box::new(JoinReducer));
        self.register_reducer(Box::new(SumReducer));
        self.register_reducer(Box::new(MaxReducer));
        self.register_reducer(Box::new(MinReducer));
    }

    /// Register an iterator function
    pub fn register_iterator(&mut self, func: Box<dyn IteratorFunction>) {
        let name = func.metadata().name.clone();
        self.iterators.insert(name, func);
    }

    /// Register a reducer function
    pub fn register_reducer(&mut self, func: Box<dyn ReducerFunction>) {
        let name = func.metadata().name.clone();
        self.reducers.insert(name, func);
    }

    /// Get an iterator function by name
    pub fn get_iterator(&self, name: &str) -> Option<&dyn IteratorFunction> {
        self.iterators.get(name).map(|b| b.as_ref())
    }

    /// Get a reducer function by name
    pub fn get_reducer(&self, name: &str) -> Option<&dyn ReducerFunction> {
        self.reducers.get(name).map(|b| b.as_ref())
    }

    /// Check if a function name is registered as an iterator
    pub fn is_iterator(&self, name: &str) -> bool {
        self.iterators.contains_key(name)
    }

    /// Check if a function name is registered as a reducer
    pub fn is_reducer(&self, name: &str) -> bool {
        self.reducers.contains_key(name)
    }

    /// Check if a function name is registered (either type)
    pub fn is_registered(&self, name: &str) -> bool {
        self.is_iterator(name) || self.is_reducer(name)
    }

    /// Get the type of a registered function
    pub fn get_function_type(&self, name: &str) -> Option<FunctionType> {
        if self.is_iterator(name) {
            Some(FunctionType::Iterator)
        } else if self.is_reducer(name) {
            Some(FunctionType::Reducer)
        } else {
            None
        }
    }

    /// Get all registered iterator names
    pub fn iterator_names(&self) -> Vec<String> {
        self.iterators.keys().cloned().collect()
    }

    /// Get all registered reducer names
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

/// Split a text value into words
struct SplitByWordFunction;

impl IteratorFunction for SplitByWordFunction {
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult {
        let text = extract_text_value(&item.value);
        let words = split_words(&text);
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

/// Split an array into elements
struct SplitArrayFunction;

impl IteratorFunction for SplitArrayFunction {
    fn execute(&self, item: &IterationItem) -> IteratorExecutionResult {
        // TODO: Implement actual array splitting logic
        // Currently treats array as single item (identity operation)
        // Should extract array elements from item.value and return Vec<IterationItem>
        IteratorExecutionResult::Items(vec![item.clone()])
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

struct FirstReducer;

impl ReducerFunction for FirstReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        sorted_items(items)
            .first()
            .map(|item| extract_text_value(&item.value))
            .unwrap_or_default()
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "first".to_string(),
            function_type: FunctionType::Reducer,
            description: "Return the first item".to_string(),
        }
    }
}

struct LastReducer;

impl ReducerFunction for LastReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        sorted_items(items)
            .last()
            .map(|item| extract_text_value(&item.value))
            .unwrap_or_default()
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "last".to_string(),
            function_type: FunctionType::Reducer,
            description: "Return the last item".to_string(),
        }
    }
}

struct CountReducer;

impl ReducerFunction for CountReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        items.len().to_string()
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "count".to_string(),
            function_type: FunctionType::Reducer,
            description: "Count the number of items".to_string(),
        }
    }
}

struct JoinReducer;

impl ReducerFunction for JoinReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        sorted_items(items)
            .into_iter()
            .map(|item| extract_text_value(&item.value))
            .collect::<Vec<_>>()
            .join(", ")
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "join".to_string(),
            function_type: FunctionType::Reducer,
            description: "Join items into a comma-separated string".to_string(),
        }
    }
}

struct SumReducer;

impl ReducerFunction for SumReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        let sum: f64 = items.iter()
            .filter_map(|item| {
                match &item.value.value {
                    serde_json::Value::Number(n) => n.as_f64(),
                    _ => None,
                }
            })
            .sum();
        if sum.abs() < f64::EPSILON {
            "0".to_string()
        } else {
            let mut value = sum.to_string();
            if value.ends_with(".0") {
                value.truncate(value.len() - 2);
            }
            value
        }
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "sum".to_string(),
            function_type: FunctionType::Reducer,
            description: "Sum numeric values".to_string(),
        }
    }
}

struct MaxReducer;

impl ReducerFunction for MaxReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        items.iter()
            .filter_map(|item| {
                match &item.value.value {
                    serde_json::Value::Number(n) => n.as_f64(),
                    _ => None,
                }
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "max".to_string(),
            function_type: FunctionType::Reducer,
            description: "Find maximum numeric value".to_string(),
        }
    }
}

struct MinReducer;

impl ReducerFunction for MinReducer {
    fn execute(&self, items: &[IterationItem]) -> ReducerResult {
        items.iter()
            .filter_map(|item| {
                match &item.value.value {
                    serde_json::Value::Number(n) => n.as_f64(),
                    _ => None,
                }
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
    
    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "min".to_string(),
            function_type: FunctionType::Reducer,
            description: "Find minimum numeric value".to_string(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_text_value(field_value: &FieldValue) -> String {
    match &field_value.value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Object(map) => map
            .get("value")
            .map(|v| match v {
                serde_json::Value::String(s) => s.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default(),
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.get("value"))
            .map(|v| match v {
                serde_json::Value::String(s) => s.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default(),
        serde_json::Value::Null => String::new(),
    }
}

fn split_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(|s| s.to_string()).collect()
}

/// Public helper to split words - used by engine
pub fn split_text_into_words(text: &str) -> Vec<String> {
    split_words(text)
}

/// Public helper to extract text - used by engine
pub fn extract_field_text(field_value: &FieldValue) -> String {
    extract_text_value(field_value)
}

fn sorted_items(items: &[IterationItem]) -> Vec<&IterationItem> {
    let mut sorted: Vec<&IterationItem> = items.iter().collect();
    sorted.sort_by(|a, b| {
        let a_key = (
            a.key.hash.as_deref().unwrap_or(""),
            a.key.range.as_deref().unwrap_or(""),
        );
        let b_key = (
            b.key.hash.as_deref().unwrap_or(""),
            b.key.range.as_deref().unwrap_or(""),
        );
        a_key.cmp(&b_key)
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::key_value::KeyValue;

    fn create_test_item(text: &str) -> IterationItem {
        IterationItem {
            key: KeyValue::new(Some("test".to_string()), None),
            value: FieldValue {
                value: serde_json::Value::String(text.to_string()),
                atom_uuid: "test-uuid".to_string(),
            },
            is_text_token: false,
        }
    }

    #[test]
    fn test_registry_initialization() {
        let reg = registry();
        
        // Check iterator functions
        assert!(reg.is_iterator("split_by_word"));
        assert!(reg.is_iterator("split_array"));
        
        // Check reducer functions
        assert!(reg.is_reducer("first"));
        assert!(reg.is_reducer("last"));
        assert!(reg.is_reducer("count"));
        assert!(reg.is_reducer("join"));
        assert!(reg.is_reducer("sum"));
        assert!(reg.is_reducer("max"));
        assert!(reg.is_reducer("min"));
        
        // Check unregistered
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
        let reg = registry();
        let reducer = reg.get_reducer("count").expect("count reducer should exist");
        
        let items = vec![
            create_test_item("one"),
            create_test_item("two"),
            create_test_item("three"),
        ];
        
        let result = reducer.execute(&items);
        assert_eq!(result, "3");
    }

    #[test]
    fn test_join_reducer() {
        let reg = registry();
        let reducer = reg.get_reducer("join").expect("join reducer should exist");
        
        let items = vec![
            create_test_item("hello"),
            create_test_item("world"),
        ];
        
        let result = reducer.execute(&items);
        assert_eq!(result, "hello, world");
    }

    #[test]
    fn test_sum_reducer() {
        let reg = registry();
        let reducer = reg.get_reducer("sum").expect("sum reducer should exist");
        
        let items = vec![
            IterationItem {
                key: KeyValue::new(Some("test".to_string()), None),
                value: FieldValue {
                    value: serde_json::Value::Number(serde_json::Number::from(10)),
                    atom_uuid: "test-uuid".to_string(),
                },
                is_text_token: false,
            },
            IterationItem {
                key: KeyValue::new(Some("test".to_string()), None),
                value: FieldValue {
                    value: serde_json::Value::Number(serde_json::Number::from(20)),
                    atom_uuid: "test-uuid".to_string(),
                },
                is_text_token: false,
            },
        ];
        
        let result = reducer.execute(&items);
        assert_eq!(result, "30");
    }

    #[test]
    fn test_split_by_word_execution() {
        let reg = registry();
        let func = reg.get_iterator("split_by_word").expect("split_by_word should exist");
        
        let item = create_test_item("hello world test");
        let result = func.execute(&item);
        
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

