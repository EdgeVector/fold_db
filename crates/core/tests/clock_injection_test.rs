//! Integration test: drive `TriggerRunner` deterministically with a
//! `MockClock` injected via [`fold_db::fold_db_core::factory::create_fold_db_with_clock`].
//!
//! Guards against regressing the clock-injection seam that consumers
//! (notably fold_dev_node) rely on for testing `Trigger::Scheduled`
//! behavior without `tokio::time::sleep`. A break here means a downstream
//! test that does `MockClock::advance(60_000)` to step a 60-second cron
//! tick will silently hang or flake.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use fold_db::crypto::E2eKeys;
use fold_db::fold_db_core::factory::create_fold_db_with_clock;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::Query;
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::SchemaState;
use fold_db::security::Ed25519KeyPair;
use fold_db::storage::config::DatabaseConfig;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::triggers::clock::{Clock, MockClock};
use fold_db::triggers::types::Trigger;
use fold_db::triggers::{fields, TRIGGER_FIRING_SCHEMA_NAME};
use fold_db::view::types::TransformView;

fn source_schema_json(name: &str) -> String {
    TestSchemaBuilder::new(name)
        .fields(&["f1"])
        .range_key("f_key")
        .build_json()
}

fn scheduled_view(name: &str, source: &str, cron: &str) -> TransformView {
    let mut view = TransformView::new(
        name,
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("f_key".to_string()))),
        vec![Query::new(source.to_string(), vec!["f1".to_string()])],
        None,
        HashMap::from([("f1".to_string(), FieldValueType::Any)]),
    );
    view.triggers = vec![Trigger::Scheduled {
        cron: cron.to_string(),
        timezone: "UTC".to_string(),
        max_catch_up_age: None,
        skip_if_idle: false,
        schemas: vec![source.to_string()],
    }];
    view
}

async fn scan_trigger_firings(
    db: &FoldDB<MockClock>,
    view_name: &str,
) -> Vec<HashMap<String, serde_json::Value>> {
    let q = Query::new(
        TRIGGER_FIRING_SCHEMA_NAME.to_string(),
        vec![
            fields::TRIGGER_ID.to_string(),
            fields::VIEW_NAME.to_string(),
            fields::FIRED_AT.to_string(),
            fields::STATUS.to_string(),
        ],
    );
    let results = db
        .query_executor()
        .query(q)
        .await
        .expect("query TriggerFiring");

    let mut rows: HashMap<(Option<String>, Option<String>), HashMap<String, serde_json::Value>> =
        HashMap::new();
    for (field_name, entries) in results {
        for (kv, fv) in entries {
            let key = (kv.hash.clone(), kv.range.clone());
            rows.entry(key)
                .or_default()
                .insert(field_name.clone(), fv.value.clone());
        }
    }
    rows.into_values()
        .filter(|row| {
            row.get(fields::VIEW_NAME)
                .and_then(|v| v.as_str())
                .map(|s| s == view_name)
                .unwrap_or(false)
        })
        .collect()
}

/// Poll `pred` every 5 ms up to `timeout`. Useful while waiting for the
/// scheduler task to react to a `MockClock::advance` — we can't tie into
/// the scheduler's wakeup cycle directly, so we observe its side
/// effects (parked sleepers, persisted firing rows).
async fn wait_until<F: Fn() -> bool>(pred: F, label: &str, timeout: Duration) {
    let start = std::time::Instant::now();
    while !pred() {
        if start.elapsed() > timeout {
            panic!("timeout waiting for: {label}");
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Async-predicate variant of [`wait_until`] for cases where the
/// observation requires hitting an async API (e.g. querying TriggerFiring
/// rows through the executor).
async fn wait_for<F, Fut>(pred: F, label: &str, timeout: Duration)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    loop {
        if pred().await {
            return;
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for: {label}");
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mock_clock_advance_fires_scheduled_trigger() {
    let tmp = tempfile::tempdir().unwrap();
    let secret = [42u8; 32];
    let e2e_keys = E2eKeys::from_secret(&secret);
    let signer = Arc::new(Ed25519KeyPair::generate().unwrap());
    let config = DatabaseConfig::local(tmp.path().to_path_buf());

    // MockClock starts at the unix epoch. With cron `* * * * *` the
    // first fire lands at 60_000 ms; after the runner discovers the view
    // (idle 60s wake), the next fire computes to 120_000 ms.
    let clock = Arc::new(MockClock::new(0));
    let db = create_fold_db_with_clock(&config, &e2e_keys, signer, Arc::clone(&clock))
        .await
        .expect("create_fold_db_with_clock");

    // Production fold_db loads the TriggerFiring schema via fold_db_node's
    // schema-fetch path. Tests that boot a bare FoldDB seed it directly so
    // the audit-row writer has a target schema.
    fold_db::triggers::register_trigger_firing_schema(&db.schema_manager())
        .await
        .unwrap();

    db.load_schema_from_json(&source_schema_json("S1"))
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("S1", SchemaState::Approved)
        .await
        .unwrap();

    db.schema_manager()
        .register_view(scheduled_view("V1", "S1", "* * * * *"))
        .await
        .unwrap();

    // Wait for the scheduler's idle 60s sleeper to register so we know
    // it's parked before we advance.
    wait_until(
        || clock.pending_sleeps() == 1,
        "scheduler parked on initial idle sleep",
        Duration::from_secs(2),
    )
    .await;

    // No firings yet — clock has not advanced.
    assert!(
        scan_trigger_firings(&db, "V1").await.is_empty(),
        "expected no firings before advance"
    );

    // Wake 1: the runner's idle 60s poll fires, the populate pass
    // discovers V1 and queues the next fire at 120_000 ms.
    clock.advance(60_000);
    wait_until(
        || clock.now_ms() == 60_000 && clock.pending_sleeps() == 1,
        "scheduler re-parked after registry populate",
        Duration::from_secs(2),
    )
    .await;

    // Wake 2: now == fire deadline, dispatch_nonblocking fires the view
    // and writes a TriggerFiring audit row through MutationManager.
    clock.advance(60_000);

    // Poll for the firing row — the dispatch path is async (spawned)
    // and the audit-row write goes through MutationManager.
    wait_for(
        || async { !scan_trigger_firings(&db, "V1").await.is_empty() },
        "TriggerFiring row for V1",
        Duration::from_secs(5),
    )
    .await;
    let rows = scan_trigger_firings(&db, "V1").await;

    assert_eq!(
        rows.len(),
        1,
        "expected exactly one TriggerFiring row for V1, got {:?}",
        rows
    );

    let row = &rows[0];
    assert_eq!(
        row.get(fields::VIEW_NAME),
        Some(&serde_json::json!("V1")),
        "view_name"
    );
    assert_eq!(
        row.get(fields::TRIGGER_ID),
        Some(&serde_json::json!("V1:0")),
        "trigger_id"
    );
}
