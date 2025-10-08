use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use super::{key_value::KeyValue, operations::MutationType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mutation {
    pub uuid: String,
    pub schema_name: String,
    pub fields_and_values: HashMap<String, Value>,
    pub key_value: KeyValue,
    pub pub_key: String,
    pub trust_distance: u32,
    pub mutation_type: MutationType,
    pub synchronous: Option<bool>,
    /// Optional backfill hash for tracking backfill completion
    pub backfill_hash: Option<String>,
}

impl Mutation {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        pub_key: String,
        trust_distance: u32,
        mutation_type: MutationType,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            schema_name,
            fields_and_values,
            key_value,
            pub_key,
            trust_distance,
            mutation_type,
            synchronous: None,
            backfill_hash: None,
        }
    }

    #[must_use]
    pub fn with_backfill_hash(mut self, backfill_hash: String) -> Self {
        self.backfill_hash = Some(backfill_hash);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_clone_preserves_backfill_hash() {
        let mut mutation = Mutation::new(
            "TestSchema".to_string(),
            HashMap::new(),
            KeyValue::new(None, None),
            "test_key".to_string(),
            0,
            MutationType::Update,
        );
        
        mutation.backfill_hash = Some("test_hash_123".to_string());
        
        let cloned = mutation.clone();
        
        assert_eq!(cloned.backfill_hash, Some("test_hash_123".to_string()));
        println!("✅ Clone preserves backfill_hash");
    }
    
    #[test]
    fn test_with_backfill_hash() {
        let mutation = Mutation::new(
            "TestSchema".to_string(),
            HashMap::new(),
            KeyValue::new(None, None),
            "test_key".to_string(),
            0,
            MutationType::Update,
        ).with_backfill_hash("test_hash_456".to_string());
        
        assert_eq!(mutation.backfill_hash, Some("test_hash_456".to_string()));
        println!("✅ with_backfill_hash sets the field correctly");
    }
}