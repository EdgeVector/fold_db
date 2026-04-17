//! Trust domains integration tests.
//!
//! Verifies that AccessMap (HashMap<String, AccessTier>) per-domain storage works:
//! grant/revoke trust, domain independence, well-known constants, org domains.

use std::collections::HashMap;

use fold_db::access::types::{
    org_domain, AccessTier, DOMAIN_FAMILY, DOMAIN_FINANCIAL, DOMAIN_HEALTH, DOMAIN_MEDICAL,
    DOMAIN_PERSONAL,
};
use fold_db::access::AccessMap;
use fold_db::fold_db_core::FoldDB;

async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

// ===== Load/Store AccessMap =====

#[tokio::test]
async fn load_empty_trust_map_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let map = ops
        .load_trust_map_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert!(map.is_empty());
}

#[tokio::test]
async fn store_and_load_trust_map() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let mut map: AccessMap = HashMap::new();
    map.insert("bob".to_string(), AccessTier::Trusted);
    map.insert("charlie".to_string(), AccessTier::Inner);

    ops.store_trust_map_for_domain(DOMAIN_PERSONAL, &map)
        .await
        .unwrap();

    let loaded = ops
        .load_trust_map_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(loaded.get("bob"), Some(&AccessTier::Trusted));
    assert_eq!(loaded.get("charlie"), Some(&AccessTier::Inner));
    assert_eq!(loaded.get("dave"), None);
}

// ===== Grant and Revoke Trust =====

#[tokio::test]
async fn grant_trust_inserts_into_map() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let mut map: AccessMap = ops.load_trust_map_for_domain(DOMAIN_HEALTH).await.unwrap();
    assert!(map.is_empty());

    // Grant trust
    map.insert("doctor".to_string(), AccessTier::Inner);
    ops.store_trust_map_for_domain(DOMAIN_HEALTH, &map)
        .await
        .unwrap();

    let loaded = ops.load_trust_map_for_domain(DOMAIN_HEALTH).await.unwrap();
    assert_eq!(loaded.get("doctor"), Some(&AccessTier::Inner));
}

#[tokio::test]
async fn revoke_trust_removes_from_map() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let mut map: AccessMap = HashMap::new();
    map.insert("bob".to_string(), AccessTier::Trusted);
    map.insert("charlie".to_string(), AccessTier::Outer);
    ops.store_trust_map_for_domain(DOMAIN_PERSONAL, &map)
        .await
        .unwrap();

    // Revoke bob
    map.remove("bob");
    ops.store_trust_map_for_domain(DOMAIN_PERSONAL, &map)
        .await
        .unwrap();

    let loaded = ops
        .load_trust_map_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(loaded.get("bob"), None);
    assert_eq!(loaded.get("charlie"), Some(&AccessTier::Outer));
}

// ===== Multiple Domains Stored Independently =====

#[tokio::test]
async fn domains_are_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Grant trust in personal domain
    let mut personal: AccessMap = HashMap::new();
    personal.insert("bob".to_string(), AccessTier::Trusted);
    ops.store_trust_map_for_domain(DOMAIN_PERSONAL, &personal)
        .await
        .unwrap();

    // Grant trust in health domain
    let mut health: AccessMap = HashMap::new();
    health.insert("doctor".to_string(), AccessTier::Inner);
    ops.store_trust_map_for_domain(DOMAIN_HEALTH, &health)
        .await
        .unwrap();

    // Verify: bob is in personal, not in health
    let personal_loaded = ops
        .load_trust_map_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(personal_loaded.get("bob"), Some(&AccessTier::Trusted));
    assert_eq!(personal_loaded.get("doctor"), None);

    // Verify: doctor is in health, not in personal
    let health_loaded = ops.load_trust_map_for_domain(DOMAIN_HEALTH).await.unwrap();
    assert_eq!(health_loaded.get("doctor"), Some(&AccessTier::Inner));
    assert_eq!(health_loaded.get("bob"), None);
}

#[tokio::test]
async fn many_domains_coexist() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let domains = [
        DOMAIN_PERSONAL,
        DOMAIN_FAMILY,
        DOMAIN_FINANCIAL,
        DOMAIN_HEALTH,
        DOMAIN_MEDICAL,
    ];

    // Store distinct users in each domain with different tiers
    let tiers = [
        AccessTier::Outer,
        AccessTier::Trusted,
        AccessTier::Inner,
        AccessTier::Trusted,
        AccessTier::Inner,
    ];

    for (i, domain) in domains.iter().enumerate() {
        let mut map: AccessMap = HashMap::new();
        let user = format!("user_{}", i);
        map.insert(user, tiers[i]);
        ops.store_trust_map_for_domain(domain, &map).await.unwrap();
    }

    // Verify each domain has only its own user
    for (i, domain) in domains.iter().enumerate() {
        let map = ops.load_trust_map_for_domain(domain).await.unwrap();
        let expected_user = format!("user_{}", i);
        assert_eq!(
            map.get(&expected_user),
            Some(&tiers[i]),
            "Domain {} should have user_{} at tier {:?}",
            domain,
            i,
            tiers[i]
        );

        // No other users should be present
        for j in 0..domains.len() {
            if j != i {
                let other_user = format!("user_{}", j);
                assert_eq!(
                    map.get(&other_user),
                    None,
                    "Domain {} should NOT have user_{}",
                    domain,
                    j
                );
            }
        }
    }
}

// ===== List Trust Domains =====

#[tokio::test]
async fn list_domains_initially_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let domains = ops.list_trust_domains().await.unwrap();
    assert!(domains.is_empty());
}

#[tokio::test]
async fn list_domains_after_storing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let empty: AccessMap = HashMap::new();
    ops.store_trust_map_for_domain(DOMAIN_PERSONAL, &empty)
        .await
        .unwrap();
    ops.store_trust_map_for_domain(DOMAIN_FAMILY, &empty)
        .await
        .unwrap();
    ops.store_trust_map_for_domain(DOMAIN_FINANCIAL, &empty)
        .await
        .unwrap();

    let domains = ops.list_trust_domains().await.unwrap();
    assert_eq!(domains.len(), 3);
    assert!(domains.contains(&DOMAIN_PERSONAL.to_string()));
    assert!(domains.contains(&DOMAIN_FAMILY.to_string()));
    assert!(domains.contains(&DOMAIN_FINANCIAL.to_string()));
}

#[tokio::test]
async fn delete_domain_removes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let mut map: AccessMap = HashMap::new();
    map.insert("bob".to_string(), AccessTier::Inner);
    ops.store_trust_map_for_domain(DOMAIN_HEALTH, &map)
        .await
        .unwrap();

    // Verify it exists
    let loaded = ops.load_trust_map_for_domain(DOMAIN_HEALTH).await.unwrap();
    assert_eq!(loaded.get("bob"), Some(&AccessTier::Inner));

    // Delete it
    ops.delete_trust_domain(DOMAIN_HEALTH).await.unwrap();

    // Should return empty map now
    let loaded = ops.load_trust_map_for_domain(DOMAIN_HEALTH).await.unwrap();
    assert!(loaded.is_empty());
}

// ===== Well-Known Domain Constants =====

#[test]
fn well_known_domain_constants() {
    assert_eq!(DOMAIN_PERSONAL, "personal");
    assert_eq!(DOMAIN_FAMILY, "family");
    assert_eq!(DOMAIN_FINANCIAL, "financial");
    assert_eq!(DOMAIN_HEALTH, "health");
    assert_eq!(DOMAIN_MEDICAL, "medical");
}

// ===== Org Domain Construction =====

#[test]
fn org_domain_construction() {
    assert_eq!(org_domain("abc123"), "org:abc123");
    assert_eq!(org_domain("deadbeef"), "org:deadbeef");
}

#[tokio::test]
async fn org_domain_stored_and_listed() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let domain = org_domain("my_org_hash");
    let mut map: AccessMap = HashMap::new();
    map.insert("member1".to_string(), AccessTier::Trusted);
    map.insert("member2".to_string(), AccessTier::Inner);
    ops.store_trust_map_for_domain(&domain, &map).await.unwrap();

    // Load it back
    let loaded = ops.load_trust_map_for_domain(&domain).await.unwrap();
    assert_eq!(loaded.get("member1"), Some(&AccessTier::Trusted));
    assert_eq!(loaded.get("member2"), Some(&AccessTier::Inner));

    // Verify it shows up in domain list
    let domains = ops.list_trust_domains().await.unwrap();
    assert!(domains.contains(&domain));
}

// ===== Backwards-Compatible Load/Store =====

#[tokio::test]
async fn backwards_compatible_load_store_uses_personal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Use the backwards-compatible methods (no domain param)
    let mut map = ops.load_trust_map().await.unwrap();
    map.insert("bob".to_string(), AccessTier::Trusted);
    ops.store_trust_map(&map).await.unwrap();

    // Should be stored in "personal" domain
    let personal = ops
        .load_trust_map_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(personal.get("bob"), Some(&AccessTier::Trusted));
}
