use std::collections::HashMap;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;

/// Format query results into hash->range->fields JSON
pub fn format_hash_range_fields(
    results: &HashMap<String, HashMap<KeyValue, FieldValue>>,
) -> Value {
    let mut by_key: HashMap<(Option<String>, Option<String>), HashMap<String, Value>> = HashMap::new();

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

        let range_map = top.entry(hash_key).or_insert_with(|| Value::Object(serde_json::Map::new()));
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

/// Represents a single logical record keyed by `KeyValue`.
/// The `fields` map stores field_name -> value. Atom metadata is omitted here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub fields: HashMap<String, Value>,
}

/// Convert field->(key->value) map into key->Record with field->value.
/// Does not return JSON; this is a typed structure for backend consumption.
pub fn records_from_field_map(
    results: &HashMap<String, HashMap<KeyValue, FieldValue>>,
) -> HashMap<KeyValue, Record> {
    let mut by_key: HashMap<KeyValue, HashMap<String, Value>> = HashMap::new();

    for (field_name, key_map) in results.iter() {
        for (key_value, field_val) in key_map.iter() {
            let entry = by_key.entry(key_value.clone()).or_default();
            entry.insert(field_name.clone(), field_val.value.clone());
        }
    }

    by_key
        .into_iter()
        .map(|(k, fields)| (k, Record { fields }))
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
        f1_map.insert(key1.clone(), FieldValue { value: Value::from(1), atom_uuid: "a1".to_string() });
        f1_map.insert(key2.clone(), FieldValue { value: Value::from(2), atom_uuid: "a2".to_string() });

        let mut f2_map = HashMap::new();
        f2_map.insert(key1.clone(), FieldValue { value: Value::from("x"), atom_uuid: "b1".to_string() });

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
}


