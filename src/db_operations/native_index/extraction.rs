use serde_json::Value;
use std::collections::HashSet;

use super::NativeIndexManager;

impl NativeIndexManager {
    pub(super) fn normalize_search_term(&self, term: &str) -> Option<String> {
        let lowered = term.trim().to_lowercase();
        if lowered.len() < 2 {
            return None;
        }
        Some(lowered)
    }

    fn collect_words(&self, value: &Value) -> Vec<String> {
        let mut words = HashSet::new();
        Self::collect_words_recursive(value, &mut words);
        let mut result: Vec<String> = words.into_iter().collect();
        result.sort_unstable();
        result
    }

    fn collect_words_recursive(value: &Value, acc: &mut HashSet<String>) {
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
                    Self::collect_words_recursive(item, acc);
                }
            }
            Value::Object(obj) => {
                for (_, nested_value) in obj {
                    Self::collect_words_recursive(nested_value, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_hashtags(&self, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_hashtags_recursive(value, &mut results);
        results
    }

    fn extract_hashtags_recursive(value: &Value, acc: &mut Vec<(String, String)>) {
        match value {
            Value::String(text) => {
                if let Some(tag) = text.strip_prefix('#') {
                    let normalized = tag.trim().to_ascii_lowercase();
                    if !normalized.is_empty() {
                        acc.push((format!("hashtag:{}", normalized), normalized));
                    }
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_hashtags_recursive(item, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_emails(&self, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_emails_recursive(value, &mut results);
        results
    }

    fn extract_emails_recursive(value: &Value, acc: &mut Vec<(String, String)>) {
        match value {
            Value::String(text) => {
                if text.contains('@') && text.contains('.') {
                    let normalized = text.trim().to_ascii_lowercase();
                    acc.push((format!("email:{}", normalized), normalized));
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_emails_recursive(item, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_whole_values(&self, classification: &str, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_whole_values_recursive(classification, value, &mut results);
        results
    }

    fn extract_whole_values_recursive(
        classification: &str,
        value: &Value,
        acc: &mut Vec<(String, String)>,
    ) {
        match value {
            Value::String(text) => {
                let normalized = text.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    acc.push((format!("{}:{}", classification, normalized), normalized));
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_whole_values_recursive(classification, item, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_by_classification(
        &self,
        classification: &str,
        value: &Value,
    ) -> Vec<(String, String)> {
        match classification {
            "word" => {
                let words = self.collect_words(value);
                words
                    .into_iter()
                    .map(|w| (format!("word:{}", w), w))
                    .collect()
            }
            c if c.starts_with("hashtag") => self.extract_hashtags(value),
            c if c.starts_with("email") => self.extract_emails(value),
            c if c.starts_with("name:")
                || c.starts_with("username")
                || c.starts_with("phone")
                || c.starts_with("url")
                || c.starts_with("date") =>
            {
                self.extract_whole_values(c, value)
            }
            _ => {
                let words = self.collect_words(value);
                words
                    .into_iter()
                    .map(|w| (format!("word:{}", w), w))
                    .collect()
            }
        }
    }

    /// Extract terms from a value for indexing
    pub(super) fn extract_terms(
        &self,
        classifications: &[String],
        value: &Value,
    ) -> Vec<(String, String)> {
        let mut results = Vec::new();

        for classification in classifications {
            let entries = self.extract_by_classification(classification, value);
            for (index_key, _normalized) in entries {
                // index_key is like "word:hello" or "email:test@example.com"
                results.push((index_key, classification.clone()));
            }
        }

        results
    }
}
