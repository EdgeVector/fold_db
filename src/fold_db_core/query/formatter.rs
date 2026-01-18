use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Format query results into hash->range->fields JSON
pub fn format_hash_range_fields(results: &HashMap<String, HashMap<KeyValue, FieldValue>>) -> Value {
    let mut by_key: HashMap<(Option<String>, Option<String>), HashMap<String, Value>> =
        HashMap::new();

    for (field_name, key_map) in results.iter() {
        for (key_value, field_val) in key_map.iter() {
            let key = (key_value.hash.clone(), key_value.range.clone());
            let entry = by_key.entry(key).or_default();
            entry.insert(field_name.clone(), field_val.value.clone());
        }
    }

    let mut top: serde_json::Map<String, Value> = serde_json::Map::new();
    for ((hash_opt, range_opt), fields_obj) in by_key.into_iter() {
        let hash_key = hash_opt.unwrap_or_default();
        let range_key = range_opt.unwrap_or_default();

        let range_map = top
            .entry(hash_key)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Value::Object(ref mut map) = range_map {
            let mut fields_json: serde_json::Map<String, Value> = serde_json::Map::new();
            for (k, v) in fields_obj.into_iter() {
                fields_json.insert(k, v);
            }
            map.insert(range_key, Value::Object(fields_json));
        }
    }

    Value::Object(top)
}

/// Metadata associated with a field value
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldMetadata {
    pub atom_uuid: String,
    pub source_file_name: Option<String>,
}

/// Represents a single logical record keyed by `KeyValue`.
/// The `fields` map stores field_name -> value.
/// The `metadata` map stores field_name -> atom metadata.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Record {
    pub fields: HashMap<String, Value>,
    pub metadata: HashMap<String, FieldMetadata>,
}

/// Represents a query result record with its key and fields
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QueryResultRecord {
    pub key: KeyValue,
    pub fields: HashMap<String, Value>,
}

/// Convert field->(key->value) map into key->Record with field->value.
/// Does not return JSON; this is a typed structure for backend consumption.
pub fn records_from_field_map(
    results: &HashMap<String, HashMap<KeyValue, FieldValue>>,
) -> HashMap<KeyValue, Record> {
    let mut by_key: HashMap<KeyValue, HashMap<String, Value>> = HashMap::new();
    let mut metadata_by_key: HashMap<KeyValue, HashMap<String, FieldMetadata>> = HashMap::new();

    for (field_name, key_map) in results.iter() {
        for (key_value, field_val) in key_map.iter() {
            let entry = by_key.entry(key_value.clone()).or_default();
            entry.insert(field_name.clone(), field_val.value.clone());

            let metadata_entry = metadata_by_key.entry(key_value.clone()).or_default();
            metadata_entry.insert(
                field_name.clone(),
                FieldMetadata {
                    atom_uuid: field_val.atom_uuid.clone(),
                    source_file_name: field_val.source_file_name.clone(),
                },
            );
        }
    }

    by_key
        .into_iter()
        .map(|(k, fields)| {
            let metadata = metadata_by_key.get(&k).cloned().unwrap_or_default();
            (k, Record { fields, metadata })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_field_map_into_records() {
        let mut results: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();

        let key1 = KeyValue::new(Some("h1".to_string()), Some("r1".to_string()));
        let key2 = KeyValue::new(Some("h2".to_string()), None);

        let mut f1_map = HashMap::new();
        f1_map.insert(
            key1.clone(),
            FieldValue {
                value: Value::from(1),
                atom_uuid: "a1".to_string(),
                source_file_name: None,
            },
        );
        f1_map.insert(
            key2.clone(),
            FieldValue {
                value: Value::from(2),
                atom_uuid: "a2".to_string(),
                source_file_name: None,
            },
        );

        let mut f2_map = HashMap::new();
        f2_map.insert(
            key1.clone(),
            FieldValue {
                value: Value::from("x"),
                atom_uuid: "b1".to_string(),
                source_file_name: None,
            },
        );

        results.insert("f1".to_string(), f1_map);
        results.insert("f2".to_string(), f2_map);

        let records = records_from_field_map(&results);

        let rec1 = records.get(&key1).expect("record for key1");
        assert_eq!(rec1.fields.get("f1").cloned().unwrap(), Value::from(1));
        assert_eq!(rec1.fields.get("f2").cloned().unwrap(), Value::from("x"));

        let rec2 = records.get(&key2).expect("record for key2");
        assert_eq!(rec2.fields.get("f1").cloned().unwrap(), Value::from(2));
        assert!(!rec2.fields.contains_key("f2"));
    }

    #[test]
    fn records_include_metadata() {
        let mut results: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();

        let key1 = KeyValue::new(Some("user1".to_string()), Some("post1".to_string()));

        let mut f1_map = HashMap::new();
        f1_map.insert(
            key1.clone(),
            FieldValue {
                value: Value::from("Hello World"),
                atom_uuid: "atom-123".to_string(),
                source_file_name: Some("tweets.json".to_string()),
            },
        );

        let mut f2_map = HashMap::new();
        f2_map.insert(
            key1.clone(),
            FieldValue {
                value: Value::from(42),
                atom_uuid: "atom-456".to_string(),
                source_file_name: None,
            },
        );

        results.insert("content".to_string(), f1_map);
        results.insert("likes".to_string(), f2_map);

        let records = records_from_field_map(&results);

        let rec1 = records.get(&key1).expect("record for key1");

        // Check fields
        assert_eq!(
            rec1.fields.get("content").cloned().unwrap(),
            Value::from("Hello World")
        );
        assert_eq!(rec1.fields.get("likes").cloned().unwrap(), Value::from(42));

        // Check metadata for content field
        let content_meta = rec1.metadata.get("content").expect("content metadata");
        assert_eq!(content_meta.atom_uuid, "atom-123");
        assert_eq!(
            content_meta.source_file_name,
            Some("tweets.json".to_string())
        );

        // Check metadata for likes field
        let likes_meta = rec1.metadata.get("likes").expect("likes metadata");
        assert_eq!(likes_meta.atom_uuid, "atom-456");
        assert_eq!(likes_meta.source_file_name, None);
    }

    #[test]
    fn metadata_preserved_for_multiple_keys() {
        let mut results: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();

        let key1 = KeyValue::new(Some("user1".to_string()), Some("post1".to_string()));
        let key2 = KeyValue::new(Some("user2".to_string()), Some("post2".to_string()));

        let mut field_map = HashMap::new();
        field_map.insert(
            key1.clone(),
            FieldValue {
                value: Value::from("First post"),
                atom_uuid: "atom-1".to_string(),
                source_file_name: Some("file1.json".to_string()),
            },
        );
        field_map.insert(
            key2.clone(),
            FieldValue {
                value: Value::from("Second post"),
                atom_uuid: "atom-2".to_string(),
                source_file_name: Some("file2.json".to_string()),
            },
        );

        results.insert("content".to_string(), field_map);

        let records = records_from_field_map(&results);

        // Verify key1 metadata
        let rec1 = records.get(&key1).expect("record for key1");
        let meta1 = rec1
            .metadata
            .get("content")
            .expect("content metadata for key1");
        assert_eq!(meta1.atom_uuid, "atom-1");
        assert_eq!(meta1.source_file_name, Some("file1.json".to_string()));

        // Verify key2 metadata
        let rec2 = records.get(&key2).expect("record for key2");
        let meta2 = rec2
            .metadata
            .get("content")
            .expect("content metadata for key2");
        assert_eq!(meta2.atom_uuid, "atom-2");
        assert_eq!(meta2.source_file_name, Some("file2.json".to_string()));
    }

    #[test]
    fn field_metadata_is_serializable() {
        let metadata = FieldMetadata {
            atom_uuid: "test-atom".to_string(),
            source_file_name: Some("test.json".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&metadata).expect("should serialize");
        assert!(json.contains("test-atom"));
        assert!(json.contains("test.json"));

        // Test deserialization
        let deserialized: FieldMetadata = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.atom_uuid, "test-atom");
        assert_eq!(deserialized.source_file_name, Some("test.json".to_string()));
    }
}
