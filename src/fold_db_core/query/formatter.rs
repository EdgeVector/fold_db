use std::collections::HashMap;
use serde_json::Value;
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


