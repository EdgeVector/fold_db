#[cfg(test)]
mod typed_engine_tests {
    use std::collections::HashMap;

    use crate::schema::types::field::FieldValue;
    use crate::schema::types::key_value::KeyValue;

    use crate::transform::iterator_stack_typed::engine::TypedEngine;
    use crate::transform::iterator_stack_typed::types::{IteratorSpec, TypedInput};

    fn kv(hash: &str, range: &str) -> KeyValue {
        KeyValue { hash: Some(hash.to_string()), range: Some(range.to_string()) }
    }

    #[test]
    fn test_passthrough_emits_atom_uuid() {
        let mut input: TypedInput = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(kv("h1", "r1"), FieldValue { value: serde_json::json!("hello world"), atom_uuid: "atom-1".to_string() });
        input.insert("BlogPost.content".to_string(), field_map);

        let engine = TypedEngine::new();
        let specs = vec![IteratorSpec::Schema { field_name: "BlogPost.content".to_string() }];
        let out = engine.execute_chain(&specs, &input, "BlogPost.content");
        let entries = out.get("BlogPost.content").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].atom_uuid, "atom-1");
        assert!(entries[0].value_text.is_none());
    }

    #[test]
    fn test_word_split_emits_words_with_atom_uuid() {
        let mut input: TypedInput = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(kv("h1", "r1"), FieldValue { value: serde_json::json!("alpha beta gamma"), atom_uuid: "atom-2".to_string() });
        input.insert("BlogPost.content".to_string(), field_map);

        let engine = TypedEngine::new();
        let specs = vec![
            IteratorSpec::Schema { field_name: "BlogPost.content".to_string() },
            IteratorSpec::IteratorFunction { 
                name: "split_by_word".to_string(),
                params: Vec::new(),
                field_name: "BlogPost.content".to_string() 
            }
        ];
        let out = engine.execute_chain(&specs, &input, "BlogPostWordIndex.word");
        let entries = out.get("BlogPostWordIndex.word").unwrap();
        let words: Vec<String> = entries
            .iter()
            .filter_map(|e| e.value_text.clone())
            .collect();
        assert_eq!(words, vec!["alpha", "beta", "gamma"]);
        for e in entries {
            assert_eq!(e.atom_uuid, "atom-2");
        }
    }

    #[test]
    fn test_count_reducer_execution() {
        let mut input: TypedInput = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(kv("h1", "r1"), FieldValue { value: serde_json::json!("one"), atom_uuid: "atom-1".to_string() });
        field_map.insert(kv("h1", "r2"), FieldValue { value: serde_json::json!("two"), atom_uuid: "atom-2".to_string() });
        field_map.insert(kv("h1", "r3"), FieldValue { value: serde_json::json!("three"), atom_uuid: "atom-3".to_string() });
        input.insert("BlogPost.content".to_string(), field_map);

        let engine = TypedEngine::new();
        let specs = vec![
            IteratorSpec::Schema { field_name: "BlogPost.content".to_string() },
            IteratorSpec::ReducerFunction { 
                name: "count".to_string(),
                params: Vec::new(),
                field_name: "BlogPost.content".to_string() 
            }
        ];
        let out = engine.execute_chain(&specs, &input, "BlogPostSummary.count");
        let entries = out.get("BlogPostSummary.count").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value_text, Some("3".to_string()));
        // HashMap iteration order is non-deterministic, so just check that we get one of the valid atom_uuids
        assert!(entries[0].atom_uuid == "atom-1" || entries[0].atom_uuid == "atom-2" || entries[0].atom_uuid == "atom-3");
    }

    #[test]
    fn test_join_reducer_execution() {
        let mut input: TypedInput = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(kv("h1", "r1"), FieldValue { value: serde_json::json!("hello"), atom_uuid: "atom-1".to_string() });
        field_map.insert(kv("h1", "r2"), FieldValue { value: serde_json::json!("world"), atom_uuid: "atom-2".to_string() });
        input.insert("BlogPost.content".to_string(), field_map);

        let engine = TypedEngine::new();
        let specs = vec![
            IteratorSpec::Schema { field_name: "BlogPost.content".to_string() },
            IteratorSpec::ReducerFunction { 
                name: "join".to_string(),
                params: Vec::new(),
                field_name: "BlogPost.content".to_string() 
            }
        ];
        let out = engine.execute_chain(&specs, &input, "BlogPostSummary.joined");
        let entries = out.get("BlogPostSummary.joined").unwrap();
        assert_eq!(entries.len(), 1);
        // HashMap iteration order is not deterministic, so we check that both words are present
        let result = entries[0].value_text.as_ref().unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(result.contains(", "));
    }

    #[test]
    fn test_iterator_then_reducer_chain() {
        let mut input: TypedInput = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(kv("h1", "r1"), FieldValue { value: serde_json::json!("alpha beta gamma"), atom_uuid: "atom-1".to_string() });
        input.insert("BlogPost.content".to_string(), field_map);

        let engine = TypedEngine::new();
        let specs = vec![
            IteratorSpec::Schema { field_name: "BlogPost.content".to_string() },
            IteratorSpec::IteratorFunction { 
                name: "split_by_word".to_string(),
                params: Vec::new(),
                field_name: "BlogPost.content".to_string() 
            },
            IteratorSpec::ReducerFunction { 
                name: "count".to_string(),
                params: Vec::new(),
                field_name: "BlogPost.content".to_string() 
            }
        ];
        let out = engine.execute_chain(&specs, &input, "BlogPostWordCount.count");
        let entries = out.get("BlogPostWordCount.count").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value_text, Some("3".to_string())); // Should count the 3 words from split_by_word
    }
}


