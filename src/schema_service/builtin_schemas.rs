//! Built-in system schemas that every schema service ships with.
//!
//! ## Why built-in, not proposed
//!
//! The twelve fingerprint-system schemas (Fingerprint, Mention, Edge,
//! Identity, IdentityReceipt, Persona, EdgeByFingerprint,
//! MentionByFingerprint, MentionBySource, IngestionError,
//! ExtractionStatus, ReceivedShare) are system primitives, not user
//! data. Every fold_db_node would propose byte-identical copies of
//! them at startup, triggering the schema service's propose-verify-
//! canonicalize dance for no reason — the dedup flow exists to serve
//! user-data schemas where multiple clients might submit similar-but-
//! not-identical definitions. For system primitives, proposing is
//! theatrical.
//!
//! So instead: the schema service **owns** these schemas. The service
//! binary ships with the definitions, seeds them into storage at
//! startup via `seed(&state)`, and clients **fetch** them via
//! `GET /schemas/available` — no propose, no canonicalization
//! round-trip, no identity_hash recomputation on the client side.
//!
//! This matches the invariant "the schema service is the canonical
//! definition of all schemas" more literally than the propose-based
//! flow: for these twelve, the service doesn't just approve them,
//! it defines them.
//!
//! ## Field names are still hardcoded in fold_db_node code
//!
//! The extractor, resolver, and writer reference specific field
//! names (`"id"`, `"kind"`, `"weight"`, etc.) as string literals.
//! Those are the consumer-side contract with these schemas and
//! they live in fold_db_node. Moving the schema *definitions* here
//! doesn't change that; it just means the source of truth for the
//! schema structure is one module, not duplicated between
//! `fold_db_node` and the schema service deployment.
//!
//! ## Seeding is idempotent
//!
//! `seed(&state)` iterates every built-in schema. For each, it
//! computes the identity_hash and checks whether the schema service
//! already has a schema by that hash. If present: no-op. If absent:
//! add. This makes seeding safe to re-run across restarts, against
//! fresh Sled stores, and against existing stores that may have
//! picked up the same schemas via the legacy propose flow.

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use crate::schema::types::Schema;
use crate::schema_service::state::SchemaServiceState;
use crate::schema_service::types::SchemaAddOutcome;
use std::collections::HashMap;

// ── Descriptive-name constants ─────────────────────────────────────
//
// These are the `descriptive_name` values we set on each built-in
// schema. The schema service renames every schema to its
// identity_hash on insert, so `schema.name` will be a hash after
// seeding; `schema.descriptive_name` preserves the human-readable
// label.
//
// Clients look schemas up by descriptive_name via
// canonical_names::lookup(), which the fingerprints subsystem
// populates at startup from the `descriptive_name → canonical_name`
// map returned by the schema service.

pub const FINGERPRINT: &str = "Fingerprint";
pub const MENTION: &str = "Mention";
pub const EDGE: &str = "Edge";
pub const IDENTITY: &str = "Identity";
pub const IDENTITY_RECEIPT: &str = "IdentityReceipt";
pub const PERSONA: &str = "Persona";

pub const EDGE_BY_FINGERPRINT: &str = "EdgeByFingerprint";
pub const MENTION_BY_FINGERPRINT: &str = "MentionByFingerprint";
pub const MENTION_BY_SOURCE: &str = "MentionBySource";

pub const INGESTION_ERROR: &str = "IngestionError";
pub const EXTRACTION_STATUS: &str = "ExtractionStatus";
pub const RECEIVED_SHARE: &str = "ReceivedShare";

/// Build a `Schema` value with the standard configuration:
/// - Hash schema_type by default (override where needed)
/// - descriptive_name = name (we keep them aligned for readability)
/// - field_types populated from the caller
/// - field_descriptions populated from the caller (required by schema service)
/// - sensitivity defaults to 0 unless specified per field
/// - identity_hash computed before return
struct SchemaBuilder {
    name: &'static str,
    schema_type: SchemaType,
    key: KeyConfig,
    fields: Vec<(&'static str, FieldValueType, &'static str, u8)>, // (name, type, desc, sensitivity)
}

impl SchemaBuilder {
    fn hash(name: &'static str, hash_field: &'static str) -> Self {
        Self {
            name,
            schema_type: SchemaType::Hash,
            key: KeyConfig::new(Some(hash_field.to_string()), None),
            fields: Vec::new(),
        }
    }

    fn hash_range(name: &'static str, hash_field: &'static str, range_field: &'static str) -> Self {
        Self {
            name,
            schema_type: SchemaType::HashRange,
            key: KeyConfig::new(Some(hash_field.to_string()), Some(range_field.to_string())),
            fields: Vec::new(),
        }
    }

    fn field(mut self, name: &'static str, ty: FieldValueType, description: &'static str) -> Self {
        self.fields.push((name, ty, description, 0));
        self
    }

    fn sensitive_field(
        mut self,
        name: &'static str,
        ty: FieldValueType,
        description: &'static str,
        sensitivity: u8,
    ) -> Self {
        self.fields.push((name, ty, description, sensitivity));
        self
    }

    fn build(self) -> Schema {
        let field_names: Vec<String> = self.fields.iter().map(|f| f.0.to_string()).collect();

        let mut schema = Schema::new(
            self.name.to_string(),
            self.schema_type,
            Some(self.key),
            Some(field_names),
            None,
            None,
        );

        schema.descriptive_name = Some(self.name.to_string());

        for (name, ty, description, sensitivity) in self.fields {
            schema.field_types.insert(name.to_string(), ty);
            schema
                .field_descriptions
                .insert(name.to_string(), description.to_string());
            schema.field_data_classifications.insert(
                name.to_string(),
                DataClassification {
                    sensitivity_level: sensitivity,
                    data_domain: "general".to_string(),
                },
            );
            // Default classification so the schema service doesn't reject on
            // the classification-validation path used elsewhere in the codebase.
            schema
                .field_classifications
                .insert(name.to_string(), vec!["word".to_string()]);
        }

        schema.compute_identity_hash();
        schema
    }
}

// ────────────────────────────────────────────────────────────────────
//  Primary schemas
// ────────────────────────────────────────────────────────────────────

/// Raw identity signal — unverified, content-keyed by (kind, value).
///
/// Key: `fp_<sha256(kind, canonical_value)>`
pub fn fingerprint_schema() -> Schema {
    SchemaBuilder::hash(FINGERPRINT, "id")
        .field("id", FieldValueType::String, "Stable content-derived primary key fp_<sha256(kind, value)>")
        .field("kind", FieldValueType::String, "Fingerprint kind: email, phone, face_embedding, full_name, first_name, handle, node_pub_key, ...")
        .sensitive_field("value", FieldValueType::Any, "Canonical form for scalar kinds, or a JSON-serialized 512-float vector for face embeddings", 1)
        .field("first_seen", FieldValueType::String, "ISO-8601 timestamp of the first Mention that produced this fingerprint")
        .field("last_seen", FieldValueType::String, "ISO-8601 timestamp of the most recent Mention that produced this fingerprint")
        .build()
}

/// Extracted signal on a source record. Many Mentions can reference
/// the same Fingerprint.
pub fn mention_schema() -> Schema {
    SchemaBuilder::hash(MENTION, "id")
        .field("id", FieldValueType::String, "Mention UUID mn_<uuid>")
        .field("source_schema", FieldValueType::String, "Schema name of the record the mention was extracted from")
        .field("source_key", FieldValueType::String, "Primary key of the source record")
        .field("source_field", FieldValueType::String, "Field within the source record that produced this mention")
        .field("fingerprint_ids", FieldValueType::Array(Box::new(FieldValueType::String)), "Fingerprint IDs extracted from this source signal")
        .field("extractor", FieldValueType::String, "Which extractor produced this mention: face_detect, ner_llm, email_header, calendar_attendee, contact, manual")
        .field("confidence", FieldValueType::Float, "Extractor's own confidence 0.0-1.0")
        .field("created_at", FieldValueType::String, "ISO-8601 timestamp of when the Mention was written")
        .build()
}

/// Observed relationship between two fingerprints. Content-keyed so
/// concurrent observations of the same relationship dedupe.
///
/// Key: `eg_<sha256(a, b, kind)>`
pub fn edge_schema() -> Schema {
    SchemaBuilder::hash(EDGE, "id")
        .field("id", FieldValueType::String, "Stable content-derived primary key eg_<sha256(a, b, kind)>")
        .field("a", FieldValueType::String, "First fingerprint ID. Ordering (a, b) is canonical so (a=X, b=Y) and (a=Y, b=X) produce the same key")
        .field("b", FieldValueType::String, "Second fingerprint ID")
        .field("kind", FieldValueType::String, "Edge kind: StrongMatch, CoOccurrence, UserAsserted, TemporalCoincidence, UserForbidden")
        .field("weight", FieldValueType::Float, "Edge strength 0.0-1.0. Persona resolver filters by this against Persona.threshold")
        .field("evidence_mention_ids", FieldValueType::Array(Box::new(FieldValueType::String)), "Mentions that produced this edge, for auditability")
        .field("created_at", FieldValueType::String, "ISO-8601 timestamp when the edge was first observed")
        .build()
}

/// Verified identity — a signed Identity Card anchored to a public key.
/// Content-keyed by the pubkey so re-receiving the same card dedupes.
///
/// Key: `id_<pub_key>`
pub fn identity_schema() -> Schema {
    SchemaBuilder::hash(IDENTITY, "id")
        .field("id", FieldValueType::String, "Primary key id_<pub_key>")
        .field(
            "pub_key",
            FieldValueType::String,
            "Ed25519 public key, the identity anchor",
        )
        .field(
            "display_name",
            FieldValueType::String,
            "Self-attested display name from the card",
        )
        .field(
            "birthday",
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "Self-attested birthday (optional), ISO-8601 date",
        )
        .sensitive_field(
            "face_embedding",
            FieldValueType::OneOf(vec![
                FieldValueType::Array(Box::new(FieldValueType::Float)),
                FieldValueType::Null,
            ]),
            "Self-attested face embedding (optional)",
            1,
        )
        .field(
            "node_id",
            FieldValueType::String,
            "The pubkey duplicated here for explicit readability",
        )
        .field(
            "card_signature",
            FieldValueType::String,
            "Ed25519 signature over the card payload, by the same pubkey",
        )
        .field(
            "issued_at",
            FieldValueType::String,
            "ISO-8601 timestamp the card was issued",
        )
        .build()
}

/// Receive-metadata sidecar for Identity. One per receive event.
/// Separate from Identity so duplicate-received cards stay deduped
/// at the Identity layer while receive metadata remains per-act.
pub fn identity_receipt_schema() -> Schema {
    SchemaBuilder::hash(IDENTITY_RECEIPT, "id")
        .field("id", FieldValueType::String, "Primary key ir_<uuid>")
        .field("identity_id", FieldValueType::String, "FK to Identity.id")
        .field(
            "received_at",
            FieldValueType::String,
            "ISO-8601 timestamp of the receive event",
        )
        .field(
            "received_via",
            FieldValueType::String,
            "Channel: QRScan, NFC, DirectMessage, URL, PasteImport, Self",
        )
        .field(
            "received_from",
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "Forwarding contact pubkey if received via messaging",
        )
        .field(
            "trust_level",
            FieldValueType::String,
            "HighInPerson | MediumForwarded | LowUnverified | Self",
        )
        .build()
}

/// A named cluster — the user-facing lens over the fingerprint graph.
/// Persona is mutable: user edits threshold, excluded_mention_ids,
/// excluded_edge_ids, identity_id, aliases, etc.
pub fn persona_schema() -> Schema {
    SchemaBuilder::hash(PERSONA, "id")
        .field("id", FieldValueType::String, "Primary key ps_<uuid>")
        .field(
            "name",
            FieldValueType::String,
            "User-facing name of the persona, e.g. 'Tom Tang'",
        )
        .field(
            "seed_fingerprint_ids",
            FieldValueType::Array(Box::new(FieldValueType::String)),
            "Seed fingerprints from which the cluster resolves via graph traversal",
        )
        .field(
            "threshold",
            FieldValueType::Float,
            "Minimum edge weight to include in the resolved set (user-controlled slider)",
        )
        .field(
            "excluded_mention_ids",
            FieldValueType::Array(Box::new(FieldValueType::String)),
            "Mentions the user explicitly removed from this persona",
        )
        .field(
            "excluded_edge_ids",
            FieldValueType::Array(Box::new(FieldValueType::String)),
            "Edges the user explicitly removed from this persona — the merge-undo path",
        )
        .field(
            "included_mention_ids",
            FieldValueType::Array(Box::new(FieldValueType::String)),
            "Mentions the user explicitly added to this persona beyond what the resolver found",
        )
        .field(
            "aliases",
            FieldValueType::Array(Box::new(FieldValueType::String)),
            "Alternate names for display",
        )
        .field(
            "relationship",
            FieldValueType::String,
            "self | family | colleague | friend | acquaintance | unknown",
        )
        .field(
            "trust_tier",
            FieldValueType::Integer,
            "Trust tier 0-4 (0=Public, 4=Owner)",
        )
        .field(
            "identity_id",
            FieldValueType::OneOf(vec![
                FieldValueType::SchemaRef("Identity".to_string()),
                FieldValueType::Null,
            ]),
            "Optional link to a verified Identity. Required for trust-gated operations.",
        )
        .field(
            "user_confirmed",
            FieldValueType::Boolean,
            "True when the user has explicitly confirmed this persona (vs system-proposed)",
        )
        .field(
            "built_in",
            FieldValueType::Boolean,
            "True for the built-in Me persona. Backend rejects mutation of built_in personas.",
        )
        .field("created_at", FieldValueType::String, "ISO-8601 timestamp")
        .build()
}

// ────────────────────────────────────────────────────────────────────
//  Junction schemas
// ────────────────────────────────────────────────────────────────────

/// Reverse-lookup junction: given a Fingerprint ID, find every Edge
/// that touches it. Written twice per Edge (once per endpoint).
pub fn edge_by_fingerprint_schema() -> Schema {
    SchemaBuilder::hash_range(EDGE_BY_FINGERPRINT, "fingerprint_id", "edge_id")
        .field(
            "fingerprint_id",
            FieldValueType::String,
            "Endpoint fingerprint — the hash key for reverse lookup",
        )
        .field(
            "edge_id",
            FieldValueType::String,
            "Edge touching this endpoint — the range key",
        )
        .build()
}

/// Reverse-lookup junction: given a Fingerprint ID, find every Mention
/// that references it. One row per (Mention, Fingerprint) pair.
pub fn mention_by_fingerprint_schema() -> Schema {
    SchemaBuilder::hash_range(MENTION_BY_FINGERPRINT, "fingerprint_id", "mention_id")
        .field(
            "fingerprint_id",
            FieldValueType::String,
            "Fingerprint referenced by the mention — the hash key",
        )
        .field(
            "mention_id",
            FieldValueType::String,
            "Mention ID — the range key",
        )
        .build()
}

/// Reverse-lookup junction: given a source record (schema, key), find
/// every Mention extracted from it. One row per Mention.
pub fn mention_by_source_schema() -> Schema {
    SchemaBuilder::hash_range(MENTION_BY_SOURCE, "source_composite", "mention_id")
        .field(
            "source_composite",
            FieldValueType::String,
            "Format: '<source_schema>:<source_key>' — the hash key",
        )
        .field(
            "mention_id",
            FieldValueType::String,
            "Mention ID — the range key",
        )
        .build()
}

// ────────────────────────────────────────────────────────────────────
//  Support schemas
// ────────────────────────────────────────────────────────────────────

/// Loud per-item ingestion failure record. Powers the "Failed records"
/// panel in the People tab.
pub fn ingestion_error_schema() -> Schema {
    SchemaBuilder::hash(INGESTION_ERROR, "id")
        .field("id", FieldValueType::String, "Primary key ie_<uuid>")
        .field(
            "source_schema",
            FieldValueType::String,
            "Schema of the record whose ingestion failed",
        )
        .field(
            "source_key",
            FieldValueType::String,
            "Primary key of the failed record",
        )
        .field(
            "extractor",
            FieldValueType::String,
            "Extractor kind that failed (face_detect, ner_llm, ...)",
        )
        .field(
            "error_class",
            FieldValueType::String,
            "Machine-readable error class, e.g. FaceDetectorError",
        )
        .field(
            "error_msg",
            FieldValueType::String,
            "Full error context, not just the short message",
        )
        .field(
            "retry_count",
            FieldValueType::Integer,
            "Number of retry attempts so far",
        )
        .field(
            "resolved",
            FieldValueType::Boolean,
            "True if the user dismissed or retried successfully",
        )
        .field(
            "created_at",
            FieldValueType::String,
            "ISO-8601 timestamp of first failure",
        )
        .field(
            "last_retry_at",
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "ISO-8601 timestamp of most recent retry",
        )
        .build()
}

/// Per-(source, extractor) outcome, so the UI can distinguish
/// 'not yet processed' from 'processed, found nothing' from 'failed'.
pub fn extraction_status_schema() -> Schema {
    SchemaBuilder::hash(EXTRACTION_STATUS, "id")
        .field(
            "id",
            FieldValueType::String,
            "Composite key es_<source_schema>:<source_key>:<extractor>",
        )
        .field(
            "source_schema",
            FieldValueType::String,
            "Source record schema",
        )
        .field("source_key", FieldValueType::String, "Source record key")
        .field("extractor", FieldValueType::String, "Extractor kind")
        .field(
            "status",
            FieldValueType::String,
            "NotRun | RanWithResults | RanEmpty | Failed",
        )
        .field(
            "fingerprint_count",
            FieldValueType::Integer,
            "Number of fingerprints produced (0 if RanEmpty or Failed)",
        )
        .field(
            "ran_at",
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "ISO-8601 timestamp of when this extractor last ran, if ever",
        )
        .field(
            "model_version",
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "Extractor model version, so re-ingest can detect staleness",
        )
        .build()
}

/// Isolated incoming shared Personas from peer nodes. Phase 3.
/// NEVER auto-merged into the recipient's own fingerprint graph.
pub fn received_share_schema() -> Schema {
    SchemaBuilder::hash(RECEIVED_SHARE, "id")
        .field("id", FieldValueType::String, "Primary key rs_<uuid>")
        .field("sender_identity_id", FieldValueType::String, "FK to Identity.id — the sender's verified pubkey")
        .field("sender_persona_name", FieldValueType::String, "Label the sender gave to the shared persona")
        .sensitive_field("payload", FieldValueType::Any, "Full shared payload — persona snapshot + fingerprint/mention/edge snapshots + optional identity snapshot. Opaque to the resolver.", 2)
        .field("received_at", FieldValueType::String, "ISO-8601 timestamp of arrival")
        .field("accepted", FieldValueType::Boolean, "False until user explicitly accepts; default-reject on timeout")
        .field("merged_into", FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]), "If user later imports the shared data into a local Persona, the target Persona ID goes here")
        .build()
}

// ────────────────────────────────────────────────────────────────────
//  Registration order
// ────────────────────────────────────────────────────────────────────

/// Return every Phase 1 schema in dependency order: primaries first,
/// then junctions, then support. The order matters because junctions
/// reference primaries and we want the primaries registered first so
/// a future validator could enforce the relationship.
pub fn all_phase_1_schemas() -> Vec<Schema> {
    vec![
        fingerprint_schema(),
        mention_schema(),
        edge_schema(),
        identity_schema(),
        identity_receipt_schema(),
        persona_schema(),
        edge_by_fingerprint_schema(),
        mention_by_fingerprint_schema(),
        mention_by_source_schema(),
        ingestion_error_schema(),
        extraction_status_schema(),
        received_share_schema(),
    ]
}

/// The list of all descriptive names for the built-in schemas. Clients
/// (fold_db_node's fingerprints subsystem) use this list to fetch
/// every built-in by descriptive name and populate their local
/// canonical-name registry.
pub const PHASE_1_DESCRIPTIVE_NAMES: &[&str] = &[
    FINGERPRINT,
    MENTION,
    EDGE,
    IDENTITY,
    IDENTITY_RECEIPT,
    PERSONA,
    EDGE_BY_FINGERPRINT,
    MENTION_BY_FINGERPRINT,
    MENTION_BY_SOURCE,
    INGESTION_ERROR,
    EXTRACTION_STATUS,
    RECEIVED_SHARE,
];

/// Seed every built-in schema into the given `SchemaServiceState`.
///
/// Idempotent: schemas whose identity_hash is already present in the
/// service are skipped. Schemas that are missing get added via
/// `state.add_schema()`, which is the same path user-data schemas
/// take. The only semantic difference is the origin — these come
/// from the service's own Rust code, not from a client proposal.
///
/// Called from `SchemaServiceServer::new()` so every production
/// schema service instance auto-seeds on boot. Tests that spin up a
/// `SchemaServiceState` directly must call this explicitly before
/// handing the state to an HTTP layer.
///
/// Fails loudly on the first error. Partial seeding is not a valid
/// state for the service — if we can't install the built-ins, the
/// service should refuse to start.
pub async fn seed(state: &SchemaServiceState) -> FoldDbResult<()> {
    for schema in all_phase_1_schemas() {
        let descriptive = schema
            .descriptive_name
            .clone()
            .unwrap_or_else(|| schema.name.clone());
        match state.add_schema(schema, HashMap::new()).await {
            Ok(SchemaAddOutcome::Added(_, _)) | Ok(SchemaAddOutcome::AlreadyExists(_, _)) => {
                // Both outcomes are fine. AlreadyExists fires when the
                // service was previously seeded (or a client
                // previously proposed this schema via the legacy
                // propose flow). Idempotent either way.
            }
            Ok(SchemaAddOutcome::Expanded(old, _, _)) => {
                return Err(FoldDbError::Config(format!(
                    "builtin_schemas: '{}' unexpectedly expanded existing schema '{}'. \
                     Built-in schemas must not collide with user-proposed schemas.",
                    descriptive, old
                )));
            }
            Err(e) => {
                return Err(FoldDbError::Config(format!(
                    "builtin_schemas: failed to seed '{}': {}",
                    descriptive, e
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_schemas_have_computed_identity_hash() {
        for schema in all_phase_1_schemas() {
            assert!(
                schema.get_identity_hash().is_some(),
                "{} missing identity_hash",
                schema.name
            );
        }
    }

    #[test]
    fn all_schemas_have_a_key_config() {
        for schema in all_phase_1_schemas() {
            assert!(schema.key.is_some(), "{} missing key config", schema.name);
        }
    }

    #[test]
    fn all_fields_have_types_and_descriptions() {
        for schema in all_phase_1_schemas() {
            let fields = schema.fields.as_ref().unwrap();
            for field in fields {
                assert!(
                    schema.field_types.contains_key(field),
                    "{}.{} missing field type",
                    schema.name,
                    field
                );
                assert!(
                    schema.field_descriptions.contains_key(field),
                    "{}.{} missing description",
                    schema.name,
                    field
                );
                assert!(
                    schema.field_data_classifications.contains_key(field),
                    "{}.{} missing data classification",
                    schema.name,
                    field
                );
            }
        }
    }

    #[test]
    fn phase_1_produces_twelve_schemas() {
        assert_eq!(all_phase_1_schemas().len(), 12);
    }

    #[test]
    fn all_schema_names_are_unique() {
        let names: HashSet<String> = all_phase_1_schemas()
            .iter()
            .map(|s| s.name.clone())
            .collect();
        assert_eq!(names.len(), 12);
    }

    #[test]
    fn junctions_use_hash_range_type() {
        let hash_range_names: HashSet<&'static str> = [
            EDGE_BY_FINGERPRINT,
            MENTION_BY_FINGERPRINT,
            MENTION_BY_SOURCE,
        ]
        .iter()
        .copied()
        .collect();
        for schema in all_phase_1_schemas() {
            if hash_range_names.contains(schema.name.as_str()) {
                assert_eq!(
                    schema.schema_type,
                    SchemaType::HashRange,
                    "{} should be HashRange",
                    schema.name
                );
                let key = schema.key.as_ref().unwrap();
                assert!(
                    key.hash_field.is_some() && key.range_field.is_some(),
                    "{} should have both hash_field and range_field",
                    schema.name
                );
            }
        }
    }

    #[test]
    fn primaries_use_hash_type() {
        let primary_names: HashSet<&'static str> = [
            FINGERPRINT,
            MENTION,
            EDGE,
            IDENTITY,
            IDENTITY_RECEIPT,
            PERSONA,
            INGESTION_ERROR,
            EXTRACTION_STATUS,
            RECEIVED_SHARE,
        ]
        .iter()
        .copied()
        .collect();
        for schema in all_phase_1_schemas() {
            if primary_names.contains(schema.name.as_str()) {
                assert_eq!(
                    schema.schema_type,
                    SchemaType::Hash,
                    "{} should be Hash",
                    schema.name
                );
            }
        }
    }

    #[test]
    fn persona_has_identity_link_as_schema_ref() {
        let persona = persona_schema();
        let identity_link_type = persona.field_types.get("identity_id").unwrap();
        // Must be OneOf([SchemaRef("Identity"), Null])
        match identity_link_type {
            FieldValueType::OneOf(variants) => {
                let has_schema_ref = variants
                    .iter()
                    .any(|v| matches!(v, FieldValueType::SchemaRef(name) if name == "Identity"));
                assert!(
                    has_schema_ref,
                    "identity_id must include SchemaRef(\"Identity\") variant"
                );
            }
            other => panic!("identity_id should be OneOf, got {:?}", other),
        }
    }
}
