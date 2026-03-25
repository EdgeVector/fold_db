use fold_db::access::{
    AccessContext, FieldAccessPolicy, PaymentGate, SecurityLabel, TrustDistancePolicy, TrustGraph,
};
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::Field;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use serde_json::json;
use std::collections::HashMap;

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap()).await.unwrap()
}

fn notes_schema_json() -> &'static str {
    r#"{
        "name": "Notes",
        "key": { "range_field": "created_at" },
        "fields": {
            "title": {},
            "content": {},
            "created_at": {}
        }
    }"#
}

async fn setup_db_with_notes() -> FoldDB {
    let mut db = setup_db().await;

    db.load_schema_from_json(notes_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("Notes", SchemaState::Approved)
        .await
        .unwrap();

    // Insert some data
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Secret Note"));
    fields.insert("content".to_string(), json!("Classified content"));
    fields.insert("created_at".to_string(), json!("2026-01-01"));
    let mutation = Mutation::new(
        "Notes".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "owner_pub_key".to_string(),
        MutationType::Create,
    );
    db.mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await
        .unwrap();

    db
}

/// Set access policy on a field in a loaded schema
async fn set_field_policy(
    db: &FoldDB,
    schema_name: &str,
    field_name: &str,
    policy: FieldAccessPolicy,
) {
    let mut schema = db
        .schema_manager
        .get_schema(schema_name)
        .await
        .unwrap()
        .unwrap();

    if let Some(field_variant) = schema.runtime_fields.get_mut(field_name) {
        field_variant.common_mut().access_policy = Some(policy);
    }

    // Persist and reload
    db.db_ops.store_schema(schema_name, &schema).await.unwrap();
    db.schema_manager
        .load_schema_internal(schema)
        .await
        .unwrap();
}

// ===== Query Access Control Tests =====

#[tokio::test]
async fn query_with_no_access_context_returns_all_fields() {
    let db = setup_db_with_notes().await;

    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db.query_executor.query(query).await.unwrap();

    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn query_with_no_policy_returns_all_fields() {
    let db = setup_db_with_notes().await;

    // No policies set — legacy behavior, all fields accessible even for remote users
    let ctx = AccessContext::remote("bob", 5);
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();

    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn owner_always_has_access() {
    let db = setup_db_with_notes().await;

    // Set owner-only policy on content
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::owner_only(),
            ..Default::default()
        },
    )
    .await;

    let ctx = AccessContext::owner("owner");
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();

    // Owner has access to everything
    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn remote_user_blocked_by_trust_distance() {
    let db = setup_db_with_notes().await;

    // Set owner-only on content, public read on title
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::owner_only(),
            ..Default::default()
        },
    )
    .await;

    set_field_policy(
        &db,
        "Notes",
        "title",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::public_read(),
            ..Default::default()
        },
    )
    .await;

    let ctx = AccessContext::remote("bob", 3);
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();

    // title is public, content is owner-only → filtered out
    assert!(results.contains_key("title"));
    assert!(!results.contains_key("content"));
}

#[tokio::test]
async fn trust_distance_within_range_grants_access() {
    let db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(5, 0),
            ..Default::default()
        },
    )
    .await;

    // Distance 3 <= read_max 5 → granted
    let ctx = AccessContext::remote("bob", 3);
    let query = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();
    assert!(results.contains_key("content"));

    // Distance 6 > read_max 5 → denied
    let ctx2 = AccessContext::remote("charlie", 6);
    let query2 = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results2 = db
        .query_executor
        .query_with_access(query2, &ctx2, None)
        .await
        .unwrap();
    assert!(!results2.contains_key("content"));
}

#[tokio::test]
async fn payment_gate_blocks_unpaid_access() {
    let db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::public_read(),
            ..Default::default()
        },
    )
    .await;

    let gate = PaymentGate::Fixed(5.0);
    let ctx = AccessContext::remote("bob", 1);

    let query = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results = db
        .query_executor
        .query_with_access(query, &ctx, Some(&gate))
        .await
        .unwrap();

    // Not paid → filtered out
    assert!(!results.contains_key("content"));
}

// ===== Mutation Access Control Tests =====

#[tokio::test]
async fn mutation_blocked_by_write_trust_distance() {
    let mut db = setup_db_with_notes().await;

    // Set write_max = 0 (owner only) on content
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(u64::MAX, 0),
            ..Default::default()
        },
    )
    .await;

    let ctx = AccessContext::remote("bob", 1);
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("hacked!"));
    let mutation = Mutation::new(
        "Notes".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-02".to_string())),
        "bob_pub_key".to_string(),
        MutationType::Create,
    );

    let result = db
        .mutation_manager
        .write_mutations_with_access(vec![mutation], &ctx, None)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Permission denied"), "Error was: {}", err);
}

#[tokio::test]
async fn mutation_allowed_for_owner() {
    let mut db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(u64::MAX, 0),
            ..Default::default()
        },
    )
    .await;

    let ctx = AccessContext::owner("owner");
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("owner update"));
    fields.insert("created_at".to_string(), json!("2026-01-02"));
    let mutation = Mutation::new(
        "Notes".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-02".to_string())),
        "owner_pub_key".to_string(),
        MutationType::Create,
    );

    let result = db
        .mutation_manager
        .write_mutations_with_access(vec![mutation], &ctx, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn mutation_without_access_context_bypasses_checks() {
    let mut db = setup_db_with_notes().await;

    // Set strict policy
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::owner_only(),
            ..Default::default()
        },
    )
    .await;

    // Legacy path (no access context) — always succeeds
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("no context write"));
    fields.insert("created_at".to_string(), json!("2026-01-03"));
    let mutation = Mutation::new(
        "Notes".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-03".to_string())),
        "any_pub_key".to_string(),
        MutationType::Create,
    );

    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(result.is_ok());
}

// ===== Trust Graph Persistence Tests =====

#[tokio::test]
async fn trust_graph_persists_to_sled() {
    let db = setup_db().await;

    let mut graph = TrustGraph::new();
    graph.assign_trust("alice", "bob", 2);
    graph.assign_trust("alice", "charlie", 5);

    db.db_ops.store_trust_graph(&graph).await.unwrap();

    let loaded = db.db_ops.load_trust_graph().await.unwrap();
    assert_eq!(loaded.resolve("bob", "alice"), Some(2));
    assert_eq!(loaded.resolve("charlie", "alice"), Some(5));
    assert_eq!(loaded.resolve("dave", "alice"), None);
}

#[tokio::test]
async fn audit_log_persists_to_sled() {
    use fold_db::access::{AccessDecision, AuditAction, AuditEvent};

    let db = setup_db().await;

    let event = AuditEvent::new(
        "alice",
        AuditAction::Read {
            schema_name: "Notes".into(),
            fields: vec!["content".into()],
        },
        Some(0),
        &AccessDecision::Granted,
    );

    db.db_ops.append_audit_event(event).await.unwrap();

    let log = db.db_ops.load_audit_log().await.unwrap();
    assert_eq!(log.total_events(), 1);
    assert!(log.events()[0].decision_granted);
}

// ===== Security Label Tests =====

#[tokio::test]
async fn security_label_blocks_low_clearance() {
    let db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::public_read(),
            security_label: Some(SecurityLabel::new(3, "classified")),
            ..Default::default()
        },
    )
    .await;

    // Clearance 1 < label level 3 → denied
    let mut ctx = AccessContext::remote("bob", 0);
    ctx.clearance_level = 1;
    let query = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();
    assert!(!results.contains_key("content"));

    // Clearance 5 >= label level 3 → granted
    ctx.clearance_level = 5;
    let query2 = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results2 = db
        .query_executor
        .query_with_access(query2, &ctx, None)
        .await
        .unwrap();
    assert!(results2.contains_key("content"));
}
