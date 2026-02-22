use super::{key_value::KeyValue, operations::MutationType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use uuid::Uuid;

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
    /// Optional source filename for atoms created from file uploads
    pub source_file_name: Option<String>,
    /// Pre-extracted index terms (field_name → keywords), attached during ingestion
    /// to enable inline indexing without a separate async LLM call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_terms: Option<HashMap<String, Vec<String>>>,
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
            source_file_name: None,
            index_terms: None,
        }
    }

    #[must_use]
    pub fn with_backfill_hash(mut self, backfill_hash: String) -> Self {
        self.backfill_hash = Some(backfill_hash);
        self
    }

    #[must_use]
    pub fn with_source_file_name(mut self, file_name: String) -> Self {
        self.source_file_name = Some(file_name);
        self
    }

    #[must_use]
    pub fn with_index_terms(mut self, terms: HashMap<String, Vec<String>>) -> Self {
        self.index_terms = Some(terms);
        self
    }

    /// Compute a deterministic content hash of this mutation's semantic fields.
    /// Excludes uuid (random), synchronous (execution mode), backfill_hash, source_file_name (metadata).
    #[must_use]
    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.schema_name.as_bytes());
        hasher.update(serde_json::to_string(&self.mutation_type)
            .expect("MutationType is always serializable").as_bytes());
        hasher.update(serde_json::to_string(&self.key_value)
            .expect("KeyValue is always serializable").as_bytes());
        // Sort keys for deterministic ordering of HashMap
        let mut sorted_fields: Vec<_> = self.fields_and_values.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| (*k).clone());
        for (k, v) in sorted_fields {
            hasher.update(k.as_bytes());
            hasher.update(serde_json::to_string(v)
                .expect("serde_json::Value is always serializable").as_bytes());
        }
        hasher.update(self.pub_key.as_bytes());
        hasher.update(self.trust_distance.to_le_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
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
        )
        .with_backfill_hash("test_hash_456".to_string());

        assert_eq!(mutation.backfill_hash, Some("test_hash_456".to_string()));
        println!("✅ with_backfill_hash sets the field correctly");
    }

    #[test]
    fn test_content_hash_deterministic() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_json::json!("Alice"));
        fields.insert("age".to_string(), serde_json::json!(30));

        let m1 = Mutation::new(
            "Person".to_string(),
            fields.clone(),
            KeyValue::new(None, None),
            "pub_key_abc".to_string(),
            1,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Person".to_string(),
            fields,
            KeyValue::new(None, None),
            "pub_key_abc".to_string(),
            1,
            MutationType::Update,
        );

        assert_eq!(m1.content_hash(), m2.content_hash());
        // UUIDs are different but hash should be same
        assert_ne!(m1.uuid, m2.uuid);
    }

    #[test]
    fn test_content_hash_field_order_independent() {
        let mut fields_a = HashMap::new();
        fields_a.insert("z_field".to_string(), serde_json::json!(1));
        fields_a.insert("a_field".to_string(), serde_json::json!(2));

        let mut fields_b = HashMap::new();
        fields_b.insert("a_field".to_string(), serde_json::json!(2));
        fields_b.insert("z_field".to_string(), serde_json::json!(1));

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields_a,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields_b,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        assert_eq!(m1.content_hash(), m2.content_hash());
    }

    #[test]
    fn test_content_hash_different_content() {
        let mut fields_a = HashMap::new();
        fields_a.insert("name".to_string(), serde_json::json!("Alice"));

        let mut fields_b = HashMap::new();
        fields_b.insert("name".to_string(), serde_json::json!("Bob"));

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields_a,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields_b,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        assert_ne!(m1.content_hash(), m2.content_hash());
    }

    #[test]
    fn test_content_hash_different_pub_keys() {
        let fields = HashMap::new();

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields.clone(),
            KeyValue::new(None, None),
            "user_a_key".to_string(),
            0,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "user_b_key".to_string(),
            0,
            MutationType::Update,
        );

        assert_ne!(m1.content_hash(), m2.content_hash());
    }

    #[test]
    fn test_content_hash_excludes_metadata() {
        let fields = HashMap::new();

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields.clone(),
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        )
        .with_backfill_hash("some_hash".to_string())
        .with_source_file_name("file.json".to_string());

        // backfill_hash and source_file_name should not affect the content hash
        assert_eq!(m1.content_hash(), m2.content_hash());
    }

    #[test]
    fn test_content_hash_excludes_index_terms() {
        let fields = HashMap::new();

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields.clone(),
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "key".to_string(),
            0,
            MutationType::Update,
        )
        .with_index_terms(HashMap::from([
            ("name".to_string(), vec!["alice".to_string(), "johnson".to_string()]),
        ]));

        assert_eq!(m1.content_hash(), m2.content_hash());
    }
}
