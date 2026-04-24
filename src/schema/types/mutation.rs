use super::{key_value::KeyValue, operations::MutationType};
use crate::atom::provenance::Provenance;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mutation {
    pub uuid: String,
    pub schema_name: String,
    pub fields_and_values: HashMap<String, Value>,
    pub key_value: KeyValue,
    pub pub_key: String,
    pub mutation_type: MutationType,
    pub synchronous: Option<bool>,
    /// Optional source filename for atoms created from file uploads
    pub source_file_name: Option<String>,
    /// General-purpose metadata (e.g., file_hash, provenance info).
    /// Excluded from content_hash — metadata doesn't affect deduplication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Writer identity and verifiability info. Additive during the
    /// `projects/molecule-provenance-dag` migration: `None` on mutations
    /// constructed before provenance wire-through; `Some(Provenance::User{..})`
    /// once a signature is available at construction. Kept alongside
    /// `pub_key` (not in place of) until the full wire-through lands and
    /// a follow-up PR removes `pub_key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl Mutation {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        pub_key: String,
        mutation_type: MutationType,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            schema_name,
            fields_and_values,
            key_value,
            pub_key,
            mutation_type,
            synchronous: None,
            source_file_name: None,
            metadata: None,
            provenance: None,
        }
    }

    #[must_use]
    pub fn with_source_file_name(mut self, file_name: String) -> Self {
        self.source_file_name = Some(file_name);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Compute a deterministic content hash of this mutation's semantic fields.
    /// Excludes uuid (random), synchronous (execution mode), source_file_name, metadata.
    ///
    /// `provenance` contributes to the hash only when `Some`. When `None`, the
    /// output bytes are identical to pre-PR-4 mutations (`molecule-provenance-dag`
    /// PR 4 additive field). This is a non-negotiable backward-compatibility
    /// guarantee — the idempotency cache and sync log hold pre-PR-4 hashes, and
    /// breaking them breaks deduplication and replay.
    #[must_use]
    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.schema_name.as_bytes());
        hasher.update(
            serde_json::to_string(&self.mutation_type)
                .expect("MutationType is always serializable")
                .as_bytes(),
        );
        hasher.update(
            serde_json::to_string(&self.key_value)
                .expect("KeyValue is always serializable")
                .as_bytes(),
        );
        // Sort keys for deterministic ordering of HashMap
        let mut sorted_fields: Vec<_> = self.fields_and_values.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| (*k).clone());
        for (k, v) in sorted_fields {
            hasher.update(k.as_bytes());
            hasher.update(
                serde_json::to_string(v)
                    .expect("serde_json::Value is always serializable")
                    .as_bytes(),
            );
        }
        hasher.update(self.pub_key.as_bytes());
        if let Some(p) = &self.provenance {
            hasher.update(
                serde_json::to_string(p)
                    .expect("Provenance is always serializable")
                    .as_bytes(),
            );
        }
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Person".to_string(),
            fields,
            KeyValue::new(None, None),
            "pub_key_abc".to_string(),
            MutationType::Update,
        );

        // UUIDs are different but hash should be same
        assert_eq!(m1.content_hash(), m2.content_hash());
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
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields_b,
            KeyValue::new(None, None),
            "key".to_string(),
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
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields_b,
            KeyValue::new(None, None),
            "key".to_string(),
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
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "user_b_key".to_string(),
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
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "key".to_string(),
            MutationType::Update,
        )
        .with_source_file_name("file.json".to_string());

        // source_file_name should not affect the content hash
        assert_eq!(m1.content_hash(), m2.content_hash());
    }

    #[test]
    fn test_content_hash_excludes_general_metadata() {
        let fields = HashMap::new();

        let m1 = Mutation::new(
            "Schema".to_string(),
            fields.clone(),
            KeyValue::new(None, None),
            "key".to_string(),
            MutationType::Update,
        );

        let m2 = Mutation::new(
            "Schema".to_string(),
            fields,
            KeyValue::new(None, None),
            "key".to_string(),
            MutationType::Update,
        )
        .with_metadata(HashMap::from([(
            "file_hash".to_string(),
            "abc123def456".to_string(),
        )]));

        assert_eq!(m1.content_hash(), m2.content_hash());
    }

    // ------------------------------------------------------------------
    // molecule-provenance-dag PR 4 — additive `provenance: Option<Provenance>`.
    // ------------------------------------------------------------------

    /// Fixed-shape Mutation used by the golden JSON / golden hash tests below.
    /// UUID is hard-coded so the serialized form is stable.
    fn golden_mutation() -> Mutation {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_json::json!("Alice"));
        Mutation {
            uuid: "00000000-0000-0000-0000-000000000001".to_string(),
            schema_name: "Person".to_string(),
            fields_and_values: fields,
            key_value: KeyValue::new(None, None),
            pub_key: "pk".to_string(),
            mutation_type: MutationType::Update,
            synchronous: None,
            source_file_name: None,
            metadata: None,
            provenance: None,
        }
    }

    /// Pre-PR-4 serialized shape: same `golden_mutation()` minus the
    /// `provenance` field, byte-for-byte what the old code path wrote.
    const GOLDEN_PRE_PR4_JSON: &str = r#"{"uuid":"00000000-0000-0000-0000-000000000001","schema_name":"Person","fields_and_values":{"name":"Alice"},"key_value":{"hash":null,"range":null},"pub_key":"pk","mutation_type":"Update","synchronous":null,"source_file_name":null}"#;

    /// content_hash of `golden_mutation()`. Computed once via the existing
    /// hash formula; pinned here so any accidental change to the hash input
    /// ordering or to backward-compat treatment of `provenance: None` will
    /// break this test loudly.
    const GOLDEN_CONTENT_HASH: &str =
        "481ae8881607d674d9d857db4c9a82b0656a6056f35b6c9110e1b5f8fb473c71";

    #[test]
    fn provenance_round_trips_through_serde() {
        let mut m = golden_mutation();
        m.provenance = Some(Provenance::user(
            "pk-b64".to_string(),
            "sig-b64".to_string(),
        ));
        let json = serde_json::to_string(&m).expect("serialize");
        let back: Mutation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, back);
    }

    /// A pre-PR-4 JSON (no `provenance` field) deserializes into a
    /// `Mutation` with `provenance: None` and re-serializes identically.
    /// This is the "serde roll-forward" guarantee — a mutation already in
    /// the sync log must not change shape after upgrade.
    #[test]
    fn pre_pr4_json_round_trips_unchanged() {
        let parsed: Mutation =
            serde_json::from_str(GOLDEN_PRE_PR4_JSON).expect("deserialize pre-PR-4 shape");
        assert_eq!(parsed.provenance, None);
        let reserialized = serde_json::to_string(&parsed).expect("serialize");
        assert_eq!(reserialized, GOLDEN_PRE_PR4_JSON);
    }

    /// `provenance: None` must produce the *exact same* content_hash a
    /// pre-PR-4 mutation would have produced. The idempotency cache and
    /// sync log already hold these hashes; breaking this breaks dedup and
    /// replay. Pinned to a hex golden so an accidental change (e.g.
    /// unconditionally hashing serialized `None`) fails loudly.
    #[test]
    fn content_hash_without_provenance_matches_golden() {
        let m = golden_mutation();
        assert_eq!(m.content_hash(), GOLDEN_CONTENT_HASH);
    }

    #[test]
    fn content_hash_is_sensitive_to_provenance_pubkey() {
        let mut m1 = golden_mutation();
        m1.provenance = Some(Provenance::user("pk-a".to_string(), "sig".to_string()));

        let mut m2 = golden_mutation();
        m2.provenance = Some(Provenance::user("pk-b".to_string(), "sig".to_string()));

        // Every other field is identical; only the User.pubkey differs.
        assert_ne!(m1.content_hash(), m2.content_hash());
    }

    /// `provenance: Some(...)` must produce a *different* content_hash
    /// from `provenance: None`. Together with the golden-hash test above,
    /// this pins both halves of the additive-hash contract: None → legacy
    /// bytes unchanged; Some → hash includes the new field.
    #[test]
    fn content_hash_some_differs_from_none() {
        let without = golden_mutation();
        let mut with = golden_mutation();
        with.provenance = Some(Provenance::user("pk".to_string(), "sig".to_string()));
        assert_ne!(without.content_hash(), with.content_hash());
    }
}
