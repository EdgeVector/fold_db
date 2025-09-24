use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub pub_key: String,
    pub trust_distance: u32,
    pub filter: Option<Value>,
}

impl Query {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields: Vec<String>,
        pub_key: String,
        trust_distance: u32,
    ) -> Self {
        Self {
            schema_name,
            fields,
            pub_key,
            trust_distance,
            filter: None,
        }
    }

    #[must_use]
    pub fn new_with_filter(
        schema_name: String,
        fields: Vec<String>,
        pub_key: String,
        trust_distance: u32,
        filter: Option<Value>,
    ) -> Self {
        Self {
            schema_name,
            fields,
            pub_key,
            trust_distance,
            filter,
        }
    }
}

#[derive(Debug, Clone, Serialize, ValueEnum, PartialEq)]
pub enum MutationType {
    Create,
    Update,
    Delete,
}

impl<'de> Deserialize<'de> for MutationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "create" => Ok(MutationType::Create),
            "update" => Ok(MutationType::Update),
            "delete" => Ok(MutationType::Delete),
            _ => Err(serde::de::Error::custom("unknown mutation type")),
        }
    }
}

// Re-export Mutation from the dedicated mutation module
pub use super::mutation::Mutation;


