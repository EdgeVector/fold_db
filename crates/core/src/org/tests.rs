use crate::org::operations;
use crate::org::types::{OrgMemberInfo, OrgRole};
use crate::storage::SledPool;
use std::sync::Arc;

#[allow(deprecated)]
fn temp_pool() -> Arc<SledPool> {
    let dir = tempfile::TempDir::new().unwrap();
    Arc::new(SledPool::new(dir.into_path()))
}

#[test]
fn test_create_org() {
    let pool = temp_pool();
    let membership =
        operations::create_org(&pool, "Edge Vector Foundation", "pubkey_alice", "Alice").unwrap();

    assert_eq!(membership.org_name, "Edge Vector Foundation");
    assert_eq!(membership.role, OrgRole::Admin);
    assert!(membership.org_secret_key.is_some());
    assert!(!membership.org_hash.is_empty());
    assert!(!membership.org_public_key.is_empty());
    assert!(!membership.org_e2e_secret.is_empty());
    assert_eq!(membership.members.len(), 1);
    assert_eq!(membership.members[0].display_name, "Alice");
    assert_eq!(membership.members[0].node_public_key, "pubkey_alice");

    // Verify it was persisted
    let retrieved = operations::get_org(&pool, &membership.org_hash)
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.org_name, "Edge Vector Foundation");
    assert_eq!(retrieved.org_hash, membership.org_hash);
}

#[test]
fn test_join_org() {
    let pool = temp_pool();

    // Alice creates the org
    let created =
        operations::create_org(&pool, "Edge Vector Foundation", "pubkey_alice", "Alice").unwrap();

    // Generate invite
    let invite = operations::generate_invite(&pool, &created.org_hash).unwrap();

    // Bob joins using a separate pool (simulating a different node)
    let pool_bob = temp_pool();
    let bob_membership = operations::join_org(&pool_bob, &invite, "pubkey_bob", "Bob").unwrap();

    assert_eq!(bob_membership.org_name, "Edge Vector Foundation");
    assert_eq!(bob_membership.role, OrgRole::Member);
    assert!(bob_membership.org_secret_key.is_none());
    assert_eq!(bob_membership.org_hash, created.org_hash);
    // Should have Alice (from invite) + Bob (self-added)
    assert_eq!(bob_membership.members.len(), 2);
    assert!(bob_membership
        .members
        .iter()
        .any(|m| m.display_name == "Bob"));
    assert!(bob_membership
        .members
        .iter()
        .any(|m| m.display_name == "Alice"));

    // Verify Bob can't join again
    let err = operations::join_org(&pool_bob, &invite, "pubkey_bob", "Bob");
    assert!(err.is_err());
}

#[test]
fn test_list_orgs() {
    let pool = temp_pool();

    // Create two orgs
    operations::create_org(&pool, "Org Alpha", "pubkey_alice", "Alice").unwrap();
    operations::create_org(&pool, "Org Beta", "pubkey_alice", "Alice").unwrap();

    let orgs = operations::list_orgs(&pool).unwrap();
    assert_eq!(orgs.len(), 2);

    let names: Vec<&str> = orgs.iter().map(|o| o.org_name.as_str()).collect();
    assert!(names.contains(&"Org Alpha"));
    assert!(names.contains(&"Org Beta"));
}

#[test]
fn test_add_remove_member() {
    let pool = temp_pool();
    let created =
        operations::create_org(&pool, "Edge Vector Foundation", "pubkey_alice", "Alice").unwrap();
    let org_hash = &created.org_hash;

    // Add Bob
    let bob = OrgMemberInfo {
        node_public_key: "pubkey_bob".to_string(),
        display_name: "Bob".to_string(),
        added_at: 1000,
        added_by: "pubkey_alice".to_string(),
    };
    operations::add_member(&pool, org_hash, bob).unwrap();

    let org = operations::get_org(&pool, org_hash).unwrap().unwrap();
    assert_eq!(org.members.len(), 2);

    // Adding duplicate should fail
    let bob_dup = OrgMemberInfo {
        node_public_key: "pubkey_bob".to_string(),
        display_name: "Bob Again".to_string(),
        added_at: 2000,
        added_by: "pubkey_alice".to_string(),
    };
    assert!(operations::add_member(&pool, org_hash, bob_dup).is_err());

    // Remove Bob
    operations::remove_member(&pool, org_hash, "pubkey_bob").unwrap();

    let org = operations::get_org(&pool, org_hash).unwrap().unwrap();
    assert_eq!(org.members.len(), 1);
    assert_eq!(org.members[0].display_name, "Alice");

    // Removing non-existent member should fail
    assert!(operations::remove_member(&pool, org_hash, "pubkey_nobody").is_err());
}

#[test]
fn test_generate_invite() {
    let pool = temp_pool();
    let created =
        operations::create_org(&pool, "Edge Vector Foundation", "pubkey_alice", "Alice").unwrap();

    let invite = operations::generate_invite(&pool, &created.org_hash).unwrap();
    assert_eq!(invite.org_name, "Edge Vector Foundation");
    assert_eq!(invite.org_public_key, created.org_public_key);
    assert_eq!(invite.org_e2e_secret, created.org_e2e_secret);
    assert_eq!(invite.members.len(), 1);

    // Non-existent org should fail
    assert!(operations::generate_invite(&pool, "nonexistent_hash").is_err());

    // Member (non-admin) should not be able to generate invite
    let pool_bob = temp_pool();
    operations::join_org(&pool_bob, &invite, "pubkey_bob", "Bob").unwrap();
    assert!(operations::generate_invite(&pool_bob, &created.org_hash).is_err());
}

#[test]
fn test_delete_org() {
    let pool = temp_pool();
    let created =
        operations::create_org(&pool, "Edge Vector Foundation", "pubkey_alice", "Alice").unwrap();

    // Delete
    operations::delete_org(&pool, &created.org_hash).unwrap();

    // Should be gone
    let result = operations::get_org(&pool, &created.org_hash).unwrap();
    assert!(result.is_none());

    // Deleting again should fail
    assert!(operations::delete_org(&pool, &created.org_hash).is_err());
}
