//! Structural decomposition for nested JSON ingestion.
//!
//! Before AI analysis, this module decomposes nested JSON by extracting fields
//! that are arrays of objects into separate "child groups." Each child group
//! recursively goes through the full ingestion pipeline (decompose -> AI ->
//! schema service -> mutations). A structure cache keyed by field-name hash
//! ensures the AI is called only once per unique structure.

use sha2::{Digest, Sha256};
use serde_json::Value;

/// A group of child items extracted from a parent field, all sharing the same structure.
pub struct ChildGroup {
    /// Field name in the parent (e.g., "posts", "comments")
    pub field_name: String,
    /// Structure hash for deduplication (SHA256 of sorted field names)
    pub structure_hash: String,
    /// The extracted array items
    pub items: Vec<Value>,
}

/// Result of decomposing a JSON object.
pub struct DecompositionResult {
    /// Parent object with array-of-object fields removed
    pub parent: Value,
    /// Child groups extracted from the parent
    pub children: Vec<ChildGroup>,
}

/// Compute a structure hash from sorted field names of a JSON object.
/// Used for deduplication of structures with the same fields.
pub fn compute_structure_hash(value: &Value) -> String {
    let mut field_names: Vec<&str> = if let Some(obj) = value.as_object() {
        obj.keys().map(|k| k.as_str()).collect()
    } else {
        Vec::new()
    };
    field_names.sort();
    let combined = field_names.join(",");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Decompose a JSON object by extracting fields that are arrays of objects.
///
/// Walks each field of the object. If a field's value is an array where the
/// first element is an object, it is extracted as a ChildGroup and removed
/// from the parent. Primitive fields, primitive arrays, and nested objects
/// (non-array) are left in the parent.
///
/// Returns the parent (with array-of-object fields removed) and child groups.
/// If data is not an object, returns it unchanged with no children.
pub fn decompose(data: &Value) -> DecompositionResult {
    let obj = match data.as_object() {
        Some(obj) => obj,
        None => {
            return DecompositionResult {
                parent: data.clone(),
                children: Vec::new(),
            };
        }
    };

    let mut parent_map = obj.clone();
    let mut children = Vec::new();

    // Collect field names that are arrays of objects
    let fields_to_extract: Vec<String> = obj
        .iter()
        .filter_map(|(field_name, value)| {
            if let Some(arr) = value.as_array() {
                if arr.first().is_some_and(|first| first.is_object()) {
                    return Some(field_name.clone());
                }
            }
            None
        })
        .collect();

    for field_name in fields_to_extract {
        let Some(value) = parent_map.remove(&field_name) else {
            continue;
        };
        let Some(arr) = value.as_array() else {
            continue;
        };
        let Some(representative) = arr.first() else {
            continue;
        };

        let structure_hash = compute_structure_hash(representative);

        children.push(ChildGroup {
            field_name,
            structure_hash,
            items: arr.iter().filter(|v| v.is_object()).cloned().collect(),
        });
    }

    DecompositionResult {
        parent: Value::Object(parent_map),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_object_with_no_arrays() {
        let data = json!({"name": "Alice", "age": 30});
        let result = decompose(&data);

        assert_eq!(result.parent, json!({"name": "Alice", "age": 30}));
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_object_with_array_of_objects() {
        let data = json!({
            "name": "Alice",
            "posts": [
                {"title": "Hi", "body": "Hello world"},
                {"title": "Bye", "body": "See ya"}
            ]
        });
        let result = decompose(&data);

        // Parent should have "posts" removed
        assert_eq!(result.parent, json!({"name": "Alice"}));
        assert_eq!(result.children.len(), 1);

        let group = &result.children[0];
        assert_eq!(group.field_name, "posts");
        assert_eq!(group.items.len(), 2);
        assert!(!group.structure_hash.is_empty());
    }

    #[test]
    fn test_object_with_array_of_primitives_left_in_parent() {
        let data = json!({
            "name": "Alice",
            "tags": ["rust", "database", "ai"]
        });
        let result = decompose(&data);

        // Primitive arrays stay in the parent
        assert_eq!(
            result.parent,
            json!({"name": "Alice", "tags": ["rust", "database", "ai"]})
        );
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_mixed_fields() {
        let data = json!({
            "name": "Alice",
            "age": 30,
            "tags": ["a", "b"],
            "posts": [
                {"title": "Post1"},
                {"title": "Post2"}
            ],
            "comments": [
                {"text": "Great!"},
                {"text": "Thanks!"}
            ]
        });
        let result = decompose(&data);

        // Parent should keep name, age, tags but not posts or comments
        let parent_obj = result.parent.as_object().unwrap();
        assert!(parent_obj.contains_key("name"));
        assert!(parent_obj.contains_key("age"));
        assert!(parent_obj.contains_key("tags"));
        assert!(!parent_obj.contains_key("posts"));
        assert!(!parent_obj.contains_key("comments"));

        assert_eq!(result.children.len(), 2);
    }

    #[test]
    fn test_nested_object_left_in_parent() {
        let data = json!({
            "name": "Alice",
            "address": {
                "city": "Portland",
                "state": "OR"
            }
        });
        let result = decompose(&data);

        // Nested objects (not arrays) stay in the parent
        assert_eq!(result.parent, data);
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_empty_array_left_in_parent() {
        let data = json!({
            "name": "Alice",
            "posts": []
        });
        let result = decompose(&data);

        // Empty arrays have no representative to hash, so left in parent
        assert_eq!(result.parent, data);
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_non_object_input_returned_unchanged() {
        let data = json!("just a string");
        let result = decompose(&data);

        assert_eq!(result.parent, json!("just a string"));
        assert!(result.children.is_empty());

        let data = json!(42);
        let result = decompose(&data);
        assert_eq!(result.parent, json!(42));
        assert!(result.children.is_empty());

        let data = json!([1, 2, 3]);
        let result = decompose(&data);
        assert_eq!(result.parent, json!([1, 2, 3]));
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_structure_hash_consistency() {
        // Two arrays with the same field structure should produce the same hash,
        // even though they have different parent field names and different values.
        let data1 = json!({
            "items": [{"a": 1, "b": "x"}]
        });
        let data2 = json!({
            "things": [{"a": 2, "b": "y"}]
        });

        let r1 = decompose(&data1);
        let r2 = decompose(&data2);

        assert_eq!(r1.children[0].structure_hash, r2.children[0].structure_hash);
    }

    #[test]
    fn test_different_structures_produce_different_hashes() {
        let data1 = json!({
            "items": [{"a": 1, "b": "x"}]
        });
        let data2 = json!({
            "items": [{"c": true, "d": 3.15}]
        });

        let r1 = decompose(&data1);
        let r2 = decompose(&data2);

        assert_ne!(r1.children[0].structure_hash, r2.children[0].structure_hash);
    }

    #[test]
    fn test_array_of_numbers_not_extracted() {
        let data = json!({
            "scores": [1, 2, 3, 4, 5]
        });
        let result = decompose(&data);

        assert_eq!(result.parent, data);
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_deeply_nested_array_of_objects_in_child() {
        // The decomposer only looks at the top level — nested arrays inside
        // child items will be handled by recursive decomposition.
        let data = json!({
            "user": "Alice",
            "posts": [
                {
                    "title": "Hi",
                    "comments": [
                        {"author": "Bob", "text": "Great!"}
                    ]
                }
            ]
        });
        let result = decompose(&data);

        assert_eq!(result.parent, json!({"user": "Alice"}));
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].field_name, "posts");

        // The child items still contain their nested arrays — recursive
        // decomposition of each item is handled by the ingestion service.
        let post = &result.children[0].items[0];
        assert!(post.get("comments").is_some());
    }
}
