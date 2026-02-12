use serde_json::Value;
use std::collections::HashSet;

use super::NativeIndexManager;

impl NativeIndexManager {
    pub(super) fn normalize_search_term(term: &str) -> Option<String> {
        let lowered = term.trim().to_lowercase();
        if lowered.len() < 2 {
            return None;
        }
        Some(lowered)
    }

    pub(super) fn extract_words(&self, value: &Value) -> Vec<String> {
        let mut words = HashSet::new();
        Self::extract_words_recursive(value, &mut words);
        let mut result: Vec<String> = words.into_iter().collect();
        result.sort_unstable();
        result
    }

    fn extract_words_recursive(value: &Value, acc: &mut HashSet<String>) {
        match value {
            Value::String(text) => {
                for word in text.split(|c: char| !c.is_alphanumeric()) {
                    let lowered = word.trim().to_lowercase();
                    if lowered.len() >= 2 {
                        acc.insert(lowered);
                    }
                }
            }
            Value::Number(n) => {
                let s = n.to_string();
                if s.len() >= 2 {
                    acc.insert(s);
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_words_recursive(item, acc);
                }
            }
            Value::Object(obj) => {
                for (_, nested_value) in obj {
                    Self::extract_words_recursive(nested_value, acc);
                }
            }
            _ => {}
        }
    }
}
