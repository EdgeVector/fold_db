use fold_db::access::{
    check_access, AccessContext, AccessDecision, AccessDenialReason, CapabilityConstraint,
    CapabilityKind, FieldAccessPolicy, PaymentGate, TrustTier,
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

// ===== Unit-level check_access Tests =====

#[test]
fn owner_always_has_access_in_any_domain() {
    let ctx = AccessContext::owner("alice");

    // Owner-only policy in personal domain
    let policy = FieldAccessPolicy::default();
    assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());
    assert!(check_access(Some(&policy), &ctx, "schema", None, true).is_granted());

    // Health domain policy requiring Inner
    let health_policy = FieldAccessPolicy {
        trust_domain: "health".to_string(),
        min_read_tier: TrustTier::Inner,
        min_write_tier: TrustTier::Inner,
        capabilities: vec![],
    };
    assert!(check_access(Some(&health_policy), &ctx, "schema", None, false).is_granted());
    assert!(check_access(Some(&health_policy), &ctx, "schema", None, true).is_granted());

    // Even with no explicit policy (defaults to owner-only)
    assert!(check_access(None, &ctx, "schema", None, false).is_granted());
    assert!(check_access(None, &ctx, "schema", None, true).is_granted());
}

#[test]
fn remote_caller_granted_when_tier_gte_min() {
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Trusted);

    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Trusted, // exact match
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());

    // Higher tier than required
    let ctx_inner = AccessContext::remote_single("bob", "personal", TrustTier::Inner);
    let policy_outer = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Outer,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    assert!(check_access(Some(&policy_outer), &ctx_inner, "schema", None, false).is_granted());
}

#[test]
fn remote_caller_denied_when_tier_lt_min() {
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Outer);

    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Trusted, // Outer(1) < Trusted(2)
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    let result = check_access(Some(&policy), &ctx, "schema", None, false);
    assert!(result.is_denied());
    if let AccessDecision::Denied(AccessDenialReason::InsufficientTrust {
        domain,
        required,
        actual,
    }) = result
    {
        assert_eq!(domain, "personal");
        assert_eq!(required, TrustTier::Trusted);
        assert_eq!(actual, TrustTier::Outer);
    } else {
        panic!("expected InsufficientTrust denial");
    }
}

#[test]
fn remote_caller_denied_when_no_domain_entry() {
    // Bob has no domain tiers at all
    let ctx = AccessContext::remote("bob", HashMap::new());

    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Public,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    let result = check_access(Some(&policy), &ctx, "schema", None, false);
    assert!(result.is_denied());
    if let AccessDecision::Denied(AccessDenialReason::NoDomainTrust { domain }) = result {
        assert_eq!(domain, "personal");
    } else {
        panic!("expected NoDomainTrust denial");
    }
}

#[test]
fn multi_domain_context_independence() {
    let mut tiers = HashMap::new();
    tiers.insert("health".to_string(), TrustTier::Inner);
    tiers.insert("personal".to_string(), TrustTier::Trusted);
    let ctx = AccessContext::remote("bob", tiers);

    // Health field requiring Trusted — Bob has Inner(3) >= Trusted(2) -> granted
    let health_policy = FieldAccessPolicy {
        trust_domain: "health".to_string(),
        min_read_tier: TrustTier::Trusted,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    assert!(check_access(Some(&health_policy), &ctx, "schema", None, false).is_granted());

    // Personal field requiring Inner — Bob has Trusted(2) < Inner(3) -> denied
    let personal_policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Inner,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    assert!(check_access(Some(&personal_policy), &ctx, "schema", None, false).is_denied());

    // Financial field — Bob has no entry -> NoDomainTrust
    let financial_policy = FieldAccessPolicy {
        trust_domain: "financial".to_string(),
        min_read_tier: TrustTier::Outer,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    let result = check_access(Some(&financial_policy), &ctx, "schema", None, false);
    assert!(matches!(
        result,
        AccessDecision::Denied(AccessDenialReason::NoDomainTrust { .. })
    ));
}

#[test]
fn unified_check_access_read_vs_write() {
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Trusted);
    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Outer,  // Trusted(2) >= Outer(1)
        min_write_tier: TrustTier::Inner, // Trusted(2) < Inner(3)
        capabilities: vec![],
    };

    // Read: granted
    assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());
    // Write: denied
    let result = check_access(Some(&policy), &ctx, "schema", None, true);
    assert!(result.is_denied());
    if let AccessDecision::Denied(AccessDenialReason::InsufficientTrust {
        required, actual, ..
    }) = result
    {
        assert_eq!(required, TrustTier::Inner);
        assert_eq!(actual, TrustTier::Trusted);
    } else {
        panic!("expected InsufficientTrust for write");
    }
}

#[test]
fn payment_gate_fixed_blocks_unpaid() {
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Inner);
    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Public,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![],
    };
    let gate = PaymentGate::Fixed(5.0);

    // Unpaid -> denied
    let result = check_access(Some(&policy), &ctx, "paid_schema", Some(&gate), false);
    assert!(result.is_denied());
    if let AccessDecision::Denied(AccessDenialReason::PaymentRequired { cost }) = result {
        assert!((cost - 5.0).abs() < f64::EPSILON);
    } else {
        panic!("expected PaymentRequired denial");
    }

    // Paid -> granted
    let mut paid_ctx = AccessContext::remote_single("bob", "personal", TrustTier::Inner);
    paid_ctx.paid_schemas.insert("paid_schema".to_string());
    assert!(check_access(Some(&policy), &paid_ctx, "paid_schema", Some(&gate), false).is_granted());
}

#[test]
fn combined_trust_capability_payment() {
    let policy = FieldAccessPolicy {
        trust_domain: "personal".to_string(),
        min_read_tier: TrustTier::Outer,
        min_write_tier: TrustTier::Owner,
        capabilities: vec![CapabilityConstraint::new(
            "pk_bob",
            CapabilityKind::Read,
            10,
        )],
    };
    let gate = PaymentGate::Fixed(1.0);

    // All three layers satisfied
    let mut ctx = AccessContext::remote_single("bob", "personal", TrustTier::Inner);
    ctx.public_keys = vec!["pk_bob".to_string()];
    ctx.paid_schemas.insert("schema".to_string());
    assert!(check_access(Some(&policy), &ctx, "schema", Some(&gate), false).is_granted());

    // Remove payment -> denied (PaymentRequired)
    ctx.paid_schemas.clear();
    let result = check_access(Some(&policy), &ctx, "schema", Some(&gate), false);
    assert!(matches!(
        result,
        AccessDecision::Denied(AccessDenialReason::PaymentRequired { .. })
    ));

    // Restore payment, remove capability key -> denied (CapabilityMissing)
    ctx.paid_schemas.insert("schema".to_string());
    ctx.public_keys.clear();
    let result = check_access(Some(&policy), &ctx, "schema", Some(&gate), false);
    assert!(matches!(
        result,
        AccessDecision::Denied(AccessDenialReason::CapabilityMissing { .. })
    ));

    // Restore capability, drop tier below min -> denied (InsufficientTrust)
    ctx.public_keys = vec!["pk_bob".to_string()];
    let mut low_tiers = HashMap::new();
    low_tiers.insert("personal".to_string(), TrustTier::Public);
    ctx.tiers = low_tiers;
    let result = check_access(Some(&policy), &ctx, "schema", Some(&gate), false);
    assert!(matches!(
        result,
        AccessDecision::Denied(AccessDenialReason::InsufficientTrust { .. })
    ));
}

// ===== Integration Tests: Query Access Through FoldDB =====

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
async fn query_default_policy_owner_only() {
    let db = setup_db_with_notes().await;

    // No explicit policies — defaults to owner-only, remote users denied
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Trusted);
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();

    assert!(!results.contains_key("title"));
    assert!(!results.contains_key("content"));

    // Owner still has access
    let owner_ctx = AccessContext::owner("owner");
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &owner_ctx, None)
        .await
        .unwrap();

    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn query_owner_always_has_access() {
    let db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Owner,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
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

    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn query_remote_filtered_by_trust_tier() {
    let db = setup_db_with_notes().await;

    // content: owner-only
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Owner,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
        },
    )
    .await;

    // title: public read
    set_field_policy(
        &db,
        "Notes",
        "title",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Public,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
        },
    )
    .await;

    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Trusted);
    let query = Query::new(
        "Notes".to_string(),
        vec!["title".to_string(), "content".to_string()],
    );
    let results = db
        .query_executor
        .query_with_access(query, &ctx, None)
        .await
        .unwrap();

    // title is public -> included, content is owner-only -> filtered out
    assert!(results.contains_key("title"));
    assert!(!results.contains_key("content"));
}

#[tokio::test]
async fn query_payment_gate_blocks_unpaid() {
    let db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Public,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
        },
    )
    .await;

    let gate = PaymentGate::Fixed(5.0);
    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Inner);

    let query = Query::new("Notes".to_string(), vec!["content".to_string()]);
    let results = db
        .query_executor
        .query_with_access(query, &ctx, Some(&gate))
        .await
        .unwrap();

    // Not paid -> filtered out
    assert!(!results.contains_key("content"));
}

// ===== Integration Tests: Mutation Access Through FoldDB =====

#[tokio::test]
async fn mutation_blocked_by_insufficient_tier() {
    let mut db = setup_db_with_notes().await;

    // content: readable by anyone, writable by owner only
    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Public,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
        },
    )
    .await;

    let ctx = AccessContext::remote_single("bob", "personal", TrustTier::Inner);
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
    assert!(
        err.contains("denied") || err.contains("Permission"),
        "Error was: {}",
        err
    );
}

#[tokio::test]
async fn mutation_allowed_for_owner() {
    let mut db = setup_db_with_notes().await;

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: TrustTier::Public,
            min_write_tier: TrustTier::Owner,
            capabilities: vec![],
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

    set_field_policy(
        &db,
        "Notes",
        "content",
        FieldAccessPolicy::default(), // owner-only
    )
    .await;

    // Legacy path (no access context) always succeeds
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
