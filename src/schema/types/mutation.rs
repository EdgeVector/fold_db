use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use super::{key_config::KeyConfig, operations::MutationType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mutation {
    pub uuid: String,
    pub schema_name: String,
    pub fields_and_values: HashMap<String, Value>,
    pub key_config: KeyConfig,
    pub pub_key: String,
    pub trust_distance: u32,
    pub mutation_type: MutationType,
    pub synchronous: Option<bool>,
}

impl Mutation {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields_and_values: HashMap<String, Value>,
        key_config: KeyConfig,
        pub_key: String,
        trust_distance: u32,
        mutation_type: MutationType,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            schema_name,
            fields_and_values,
            key_config,
            pub_key,
            trust_distance,
            mutation_type,
            synchronous: None,
        }
    }
}