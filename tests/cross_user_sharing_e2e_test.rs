//! End-to-end test for cross-user data sharing (ShareRule / ShareSubscription).
//!
//! This test exercises the full Alice -> Bob sharing flow for the feature merged
//! in PR #545. It's modeled after `org_sync_e2e_test.rs` (key copy simulating
//! the sync layer) but adds key rewriting from `share:{sender}:{recipient}:`
//! into `from:{sender}:` (the translation `SyncEngine::rewrite_key_if_needed`
//! performs at download time).
//!
//! The test is deliberately permissive about intermediate steps — its goal is
//! reconnaissance, i.e. "what actually works and what doesn't" in the current
//! implementation. When an assertion fails, the test prints enough context to
//! pinpoint the gap.

use fold_db::access::AccessContext;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::sharing::store as share_store;
use fold_db::sharing::types::{ShareRule, ShareScope, ShareSubscription};
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers (mirroring org_sync_e2e_test.rs)
// ---------------------------------------------------------------------------

async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

async fn register_personal_schema(db: &FoldDB, name: &str) {
    let json_str = TestSchemaBuilder::new(name)
        .fields(&["body"])
        .hash_key("title")
        .range_key("date")
        .build_json();
    db.load_schema_from_json(&json_str).await.unwrap();
    db.schema_manager()
        .set_schema_state(name, SchemaState::Approved)
        .await
        .unwrap();
}

async fn write_note(db: &FoldDB, schema: &str, title: &str, date: &str, body: &str) {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("body".to_string(), json!(body));
    fields.insert("date".to_string(), json!(date));
    let mutation = Mutation::new(
        schema.to_string(),
        fields,
        KeyValue::new(Some(title.to_string()), Some(date.to_string())),
        "alice_pubkey".to_string(),
        MutationType::Create,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("Failed to write mutation");
}

async fn query_bodies(db: &FoldDB, schema: &str) -> Vec<serde_json::Value> {
    let query = Query::new(schema.to_string(), vec!["body".to_string()]);
    let access = AccessContext::owner("test-owner");
    let result = db
        .query_executor()
        .query_with_access(query, &access, None)
        .await
        .expect("Query failed");
    match result.get("body") {
        Some(field_map) => field_map.values().map(|fv| fv.value.clone()).collect(),
        None => vec![],
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Copy all keys with the given prefix from src sled into dst sled, optionally
/// rewriting the prefix. This simulates what `SyncEngine::rewrite_key_if_needed`
/// does for share: prefixes (translates `share:{sender}:{recipient}:` into
/// `from:{sender}:` at replay time).
fn copy_and_rewrite_prefixed_keys(
    src_tree: &sled::Tree,
    dst_tree: &sled::Tree,
    src_prefix: &str,
    dst_prefix: &str,
) -> usize {
    let mut count = 0;
    for result in src_tree.iter() {
        let (key, value) = result.expect("Failed to read sled key");
        let key_str = String::from_utf8_lossy(&key);
        if key_str.starts_with(src_prefix) {
            let rest = &key_str[src_prefix.len()..];
            let new_key = format!("{}{}", dst_prefix, rest);
            dst_tree
                .insert(new_key.as_bytes(), value)
                .expect("Failed to insert key");
            count += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Test: Alice shares her personal "Notes" schema with Bob
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_alice_shares_notes_with_bob() {
    // 1. SETUP — two FoldDB instances, each with their own Sled
    let alice_tmp = tempfile::tempdir().unwrap();
    let bob_tmp = tempfile::tempdir().unwrap();
    let alice_db = make_folddb(&alice_tmp).await;
    let bob_db = make_folddb(&bob_tmp).await;

    let alice_pool = alice_db.sled_pool().cloned().unwrap();
    let bob_pool = bob_db.sled_pool().cloned().unwrap();

    // 2. SCHEMA — register "Notes" (personal, org_hash = None) on both sides.
    //    Bob needs to know the schema to decode Alice's molecules.
    register_personal_schema(&alice_db, "Notes").await;
    register_personal_schema(&bob_db, "Notes").await;

    // Define the share prefix up front so we can use it for both ends.
    let share_prefix = "share:alice_pubkey:bob_pubkey".to_string();
    let share_secret = vec![42u8; 32];

    // 3. ALICE creates a ShareRule for Bob BEFORE writing data so multiplex is active.
    let rule = ShareRule {
        rule_id: "rule-1".to_string(),
        recipient_pubkey: "bob_pubkey".to_string(),
        recipient_display_name: "Bob".to_string(),
        scope: ShareScope::Schema("Notes".to_string()),
        share_prefix: share_prefix.clone(),
        share_e2e_secret: share_secret.clone(),
        active: true,
        created_at: now_secs(),
        writer_pubkey: "alice_pubkey".to_string(),
        signature: String::new(),
    };
    share_store::create_share_rule(&alice_pool, rule.clone())
        .expect("Failed to create share rule on Alice");

    // NOTE: there is no `reconfigure_sync` / no way to rebuild an in-memory
    // partitioner or multiplex table here. The `mutation_manager.rs` lookup
    // calls `list_share_rules(&pool)` on every batch write, so the rule is
    // picked up automatically. This is good news.

    // 4. BOB subscribes to Alice's share
    let sub = ShareSubscription {
        sender_pubkey: "alice_pubkey".to_string(),
        share_prefix: share_prefix.clone(),
        share_e2e_secret: share_secret.clone(),
        accepted_at: now_secs(),
        active: true,
    };
    share_store::create_share_subscription(&bob_pool, sub)
        .expect("Failed to create share subscription on Bob");

    // 5. ALICE writes a note
    write_note(&alice_db, "Notes", "Hello", "2026-04-17", "World").await;
    write_note(&alice_db, "Notes", "Second", "2026-04-18", "More").await;

    // Sanity: Alice can query her own data
    let alice_bodies = query_bodies(&alice_db, "Notes").await;
    assert_eq!(
        alice_bodies.len(),
        2,
        "Alice should see her own 2 notes; got: {:?}",
        alice_bodies
    );

    // 6. Verify the multiplex happened — Alice's sled should have BOTH
    //    the personal atom/ref keys AND the share-prefixed ones.
    let alice_guard = alice_pool.acquire_arc().unwrap();
    let alice_sled = alice_guard.db();
    let main_alice = alice_sled.open_tree("main").unwrap();

    let share_prefixed_keys: Vec<String> = main_alice
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .filter(|k| k.starts_with(&format!("{}:", share_prefix)))
        .collect();
    assert!(
        !share_prefixed_keys.is_empty(),
        "Expected share-prefixed keys in Alice's sled (mutation_manager multiplex). Got none. \
         Personal-only keys visible: {:?}",
        main_alice
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
            .take(10)
            .collect::<Vec<_>>()
    );
    println!(
        "Share-prefixed keys in Alice's sled: {} keys, sample: {:?}",
        share_prefixed_keys.len(),
        share_prefixed_keys.iter().take(3).collect::<Vec<_>>()
    );

    // 7. SYNC — simulate the download-side rewrite: copy share-prefixed keys
    //    from Alice's sled to Bob's sled, translating
    //    `share:alice_pubkey:bob_pubkey:...` -> `from:alice_pubkey:...`
    let bob_guard = bob_pool.acquire_arc().unwrap();
    let bob_sled = bob_guard.db();
    let main_bob = bob_sled.open_tree("main").unwrap();

    let src_full_prefix = format!("{}:", share_prefix);
    let dst_full_prefix = "from:alice_pubkey:".to_string();
    let copied = copy_and_rewrite_prefixed_keys(
        &main_alice,
        &main_bob,
        &src_full_prefix,
        &dst_full_prefix,
    );
    assert!(copied > 0, "Expected to copy some share-prefixed keys");
    println!("Copied + rewrote {} keys into Bob's sled", copied);

    // 8. BOB queries — this is the moneymaker.
    //    Does Bob's existing query path find Alice's data under `from:alice_pubkey:`?
    //    Note: Bob's schema has org_hash=None, so query paths look for bare
    //    `atom:...` / `ref:...` keys. The rewritten data lives under
    //    `from:alice_pubkey:atom:...` / `from:alice_pubkey:ref:...`.
    //    There is no read-side integration yet (ripgrep shows `from:` only
    //    referenced in sync/engine.rs).
    let bob_bodies = query_bodies(&bob_db, "Notes").await;
    println!("Bob queried Notes.body -> {:?}", bob_bodies);
    assert_eq!(
        bob_bodies.len(),
        2,
        "EXPECTED: Bob to see Alice's 2 shared notes via `from:alice_pubkey:` namespace. \
         ACTUAL: got {} rows: {:?}. \
         Gap: there is no read-side integration that routes Bob's queries to the \
         `from:{{sender}}:` namespace where rewritten data lives.",
        bob_bodies.len(),
        bob_bodies
    );
}
