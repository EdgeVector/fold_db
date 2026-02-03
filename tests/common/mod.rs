use fold_db::schema::types::{KeyConfig, KeyValue, Mutation, MutationType};
use std::collections::HashMap;

#[allow(dead_code)]
pub fn create_test_mutation(
    schema_json: &serde_json::Value,
    mutation_json: serde_json::Value,
) -> Mutation {
    let key_config: KeyConfig = serde_json::from_value(schema_json["key"].clone())
        .expect("Failed to parse KeyConfig from schema");

    let schema_name = mutation_json["schema_name"]
        .as_str()
        .expect("Missing schema_name")
        .to_string();

    let pub_key = mutation_json["pub_key"]
        .as_str()
        .unwrap_or("default_key")
        .to_string();

    let fields_and_values: HashMap<String, serde_json::Value> =
        serde_json::from_value(mutation_json["fields_and_values"].clone())
            .expect("Failed to parse fields_and_values");

    let key_value = KeyValue::from_mutation(&fields_and_values, &key_config);

    // Allow overriding mutation type from JSON, default to Update
    let mutation_type = if let Some(type_str) = mutation_json["mutation_type"].as_str() {
        match type_str {
            "Create" => MutationType::Create,
            "Update" => MutationType::Update,
            "Delete" => MutationType::Delete,
            _ => MutationType::Update,
        }
    } else {
        MutationType::Update
    };

    let mut mutation = Mutation::new(
        schema_name,
        fields_and_values,
        key_value,
        pub_key,
        0, // trust_distance
        mutation_type,
    );

    if let Some(uuid) = mutation_json["uuid"].as_str() {
        mutation.uuid = uuid.to_string();
    }

    mutation
}
