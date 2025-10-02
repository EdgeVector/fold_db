use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use crate::schema::types::field::HashRangeFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub filter: Option<HashRangeFilter>,
}

impl Query {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields: Vec<String>,
    ) -> Self {
        Self {
            schema_name,
            fields,
            filter: None,
        }
    }

    #[must_use]
    pub fn new_with_filter(
        schema_name: String,
        fields: Vec<String>,
        filter: Option<HashRangeFilter>,
    ) -> Self {
        Self {
            schema_name,
            fields,
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


