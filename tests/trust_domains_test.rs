//! Trust domains integration tests.
//!
//! Verifies that trust domains provide independent trust contexts:
//! each domain has its own TrustGraph, and trust in one domain
//! doesn't leak to another.

use fold_db::access::types::{
    org_domain, DOMAIN_FAMILY, DOMAIN_FINANCIAL, DOMAIN_HEALTH, DOMAIN_MEDICAL, DOMAIN_PERSONAL,
};
use fold_db::access::TrustGraph;
use fold_db::fold_db_core::FoldDB;

async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

#[tokio::test]
async fn test_domains_are_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Grant trust in personal domain
    let mut personal = ops
        .load_trust_graph_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    personal.assign_trust("alice", "bob", 1);
    ops.store_trust_graph_for_domain(DOMAIN_PERSONAL, &personal)
        .await
        .unwrap();

    // Grant trust in health domain
    let mut health = ops
        .load_trust_graph_for_domain(DOMAIN_HEALTH)
        .await
        .unwrap();
    health.assign_trust("alice", "doctor", 1);
    ops.store_trust_graph_for_domain(DOMAIN_HEALTH, &health)
        .await
        .unwrap();

    // Verify: bob is trusted in personal, not in health
    let personal_loaded = ops
        .load_trust_graph_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(personal_loaded.resolve("bob", "alice"), Some(1));
    assert_eq!(personal_loaded.resolve("doctor", "alice"), None);

    let health_loaded = ops
        .load_trust_graph_for_domain(DOMAIN_HEALTH)
        .await
        .unwrap();
    assert_eq!(health_loaded.resolve("doctor", "alice"), Some(1));
    assert_eq!(health_loaded.resolve("bob", "alice"), None);
}

#[tokio::test]
async fn test_list_domains() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Initially no domains
    let domains = ops.list_trust_domains().await.unwrap();
    assert!(domains.is_empty());

    // Create domains
    let empty = TrustGraph::new();
    ops.store_trust_graph_for_domain(DOMAIN_PERSONAL, &empty)
        .await
        .unwrap();
    ops.store_trust_graph_for_domain(DOMAIN_FAMILY, &empty)
        .await
        .unwrap();
    ops.store_trust_graph_for_domain(DOMAIN_FINANCIAL, &empty)
        .await
        .unwrap();

    let domains = ops.list_trust_domains().await.unwrap();
    assert_eq!(domains.len(), 3);
    assert!(domains.contains(&DOMAIN_PERSONAL.to_string()));
    assert!(domains.contains(&DOMAIN_FAMILY.to_string()));
    assert!(domains.contains(&DOMAIN_FINANCIAL.to_string()));
}

#[tokio::test]
async fn test_delete_domain() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    let mut graph = TrustGraph::new();
    graph.assign_trust("alice", "bob", 1);
    ops.store_trust_graph_for_domain(DOMAIN_HEALTH, &graph)
        .await
        .unwrap();

    // Verify it exists
    let loaded = ops
        .load_trust_graph_for_domain(DOMAIN_HEALTH)
        .await
        .unwrap();
    assert_eq!(loaded.resolve("bob", "alice"), Some(1));

    // Delete it
    ops.delete_trust_domain(DOMAIN_HEALTH).await.unwrap();

    // Should return empty graph now
    let loaded = ops
        .load_trust_graph_for_domain(DOMAIN_HEALTH)
        .await
        .unwrap();
    assert_eq!(loaded.resolve("bob", "alice"), None);
}

#[tokio::test]
async fn test_org_domain_naming() {
    assert_eq!(org_domain("abc123"), "org:abc123");
    assert_eq!(org_domain("deadbeef"), "org:deadbeef");

    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Create an org domain with members
    let domain = org_domain("my_org_hash");
    let mut graph = TrustGraph::new();
    graph.assign_trust("org_admin", "member1", 1);
    graph.assign_trust("org_admin", "member2", 1);
    ops.store_trust_graph_for_domain(&domain, &graph)
        .await
        .unwrap();

    // Load it back
    let loaded = ops.load_trust_graph_for_domain(&domain).await.unwrap();
    assert_eq!(loaded.resolve("member1", "org_admin"), Some(1));
    assert_eq!(loaded.resolve("member2", "org_admin"), Some(1));

    // Verify it shows up in domain list
    let domains = ops.list_trust_domains().await.unwrap();
    assert!(domains.contains(&domain));
}

#[tokio::test]
async fn test_backwards_compatible_load_store() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Use the backwards-compatible methods (no domain param)
    let mut graph = ops.load_trust_graph().await.unwrap();
    graph.assign_trust("alice", "bob", 3);
    ops.store_trust_graph(&graph).await.unwrap();

    // Should be stored in "personal" domain
    let personal = ops
        .load_trust_graph_for_domain(DOMAIN_PERSONAL)
        .await
        .unwrap();
    assert_eq!(personal.resolve("bob", "alice"), Some(3));
}

#[tokio::test]
async fn test_well_known_domains() {
    assert_eq!(DOMAIN_PERSONAL, "personal");
    assert_eq!(DOMAIN_FAMILY, "family");
    assert_eq!(DOMAIN_FINANCIAL, "financial");
    assert_eq!(DOMAIN_HEALTH, "health");
    assert_eq!(DOMAIN_MEDICAL, "medical");
}

#[tokio::test]
async fn test_many_domains_coexist() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let ops = db.get_db_ops();

    // Create graphs with distinct trust relationships across 5 domains
    let domains = vec![
        DOMAIN_PERSONAL,
        DOMAIN_FAMILY,
        DOMAIN_FINANCIAL,
        DOMAIN_HEALTH,
        DOMAIN_MEDICAL,
    ];

    for (i, domain) in domains.iter().enumerate() {
        let mut graph = TrustGraph::new();
        let user = format!("user_{}", i);
        graph.assign_trust("alice", &user, (i + 1) as u64);
        ops.store_trust_graph_for_domain(domain, &graph)
            .await
            .unwrap();
    }

    // Verify each domain has only its own user
    for (i, domain) in domains.iter().enumerate() {
        let graph = ops.load_trust_graph_for_domain(domain).await.unwrap();
        let expected_user = format!("user_{}", i);
        assert_eq!(
            graph.resolve(&expected_user, "alice"),
            Some((i + 1) as u64),
            "Domain {} should have user_{} at distance {}",
            domain,
            i,
            i + 1
        );

        // No other users should be reachable
        for j in 0..5 {
            if j != i {
                let other_user = format!("user_{}", j);
                assert_eq!(
                    graph.resolve(&other_user, "alice"),
                    None,
                    "Domain {} should NOT have user_{} (that's in {})",
                    domain,
                    j,
                    domains[j]
                );
            }
        }
    }
}
