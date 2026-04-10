use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::FoldDbError;
use crate::security::Ed25519KeyPair;
use crate::storage::SledPool;

use super::types::{OrgInviteBundle, OrgMemberInfo, OrgMembership, OrgRole};

const ORG_TREE_NAME: &str = "org_memberships";

fn org_key(org_hash: &str) -> String {
    format!("org:{}", org_hash)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

fn org_tree(pool: &Arc<SledPool>) -> Result<sled::Tree, FoldDbError> {
    let guard = pool.acquire_arc().map_err(|e| {
        FoldDbError::Database(format!("Failed to acquire SledPool: {}", e))
    })?;
    guard.db().open_tree(ORG_TREE_NAME)
        .map_err(|e| FoldDbError::Database(format!("Failed to open org_memberships tree: {}", e)))
}

/// Create a new organization. Generates an Ed25519 keypair and random E2E secret.
/// The calling node becomes the admin.
pub fn create_org(
    pool: &Arc<SledPool>,
    org_name: &str,
    creator_public_key: &str,
    creator_display_name: &str,
) -> Result<OrgMembership, FoldDbError> {
    let keypair = Ed25519KeyPair::generate().map_err(|e| {
        FoldDbError::SecurityError(format!("Failed to generate org keypair: {}", e))
    })?;

    // Generate random 32-byte E2E secret
    let mut e2e_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut e2e_bytes);
    let org_e2e_secret = STANDARD.encode(e2e_bytes);

    // Derive org_hash = hex(SHA256(public_key_bytes))
    let pub_bytes = keypair.public_key_bytes();
    let mut hasher = Sha256::new();
    hasher.update(pub_bytes);
    let org_hash = format!("{:x}", hasher.finalize());

    let ts = now_secs();

    let founder = OrgMemberInfo {
        node_public_key: creator_public_key.to_string(),
        display_name: creator_display_name.to_string(),
        added_at: ts,
        added_by: creator_public_key.to_string(),
    };

    let membership = OrgMembership {
        org_name: org_name.to_string(),
        org_hash: org_hash.clone(),
        org_public_key: keypair.public_key_base64(),
        org_secret_key: Some(keypair.secret_key_base64()),
        org_e2e_secret,
        role: OrgRole::Admin,
        members: vec![founder],
        created_at: ts,
        joined_at: ts,
    };

    let tree = org_tree(pool)?;
    let key = org_key(&org_hash);
    let value = serde_json::to_vec(&membership)?;
    tree.insert(key.as_bytes(), value)
        .map_err(|e| FoldDbError::Database(format!("Failed to store org membership: {}", e)))?;

    Ok(membership)
}

/// Join an existing organization using an invite bundle.
pub fn join_org(
    pool: &Arc<SledPool>,
    invite: &OrgInviteBundle,
    my_public_key: &str,
    my_display_name: &str,
) -> Result<OrgMembership, FoldDbError> {
    // Derive org_hash from the invite's public key
    let pub_bytes = STANDARD
        .decode(&invite.org_public_key)
        .map_err(|e| FoldDbError::SecurityError(format!("Invalid org public key base64: {}", e)))?;
    let mut hasher = Sha256::new();
    hasher.update(&pub_bytes);
    let org_hash = format!("{:x}", hasher.finalize());

    let tree = org_tree(pool)?;
    let key = org_key(&org_hash);

    // Check if already a member
    if tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read org tree: {}", e)))?
        .is_some()
    {
        return Err(FoldDbError::Database(format!(
            "Already a member of org '{}'",
            invite.org_name
        )));
    }

    let ts = now_secs();

    // Start with the invite's member list and add ourselves
    let mut members = invite.members.clone();
    let me = OrgMemberInfo {
        node_public_key: my_public_key.to_string(),
        display_name: my_display_name.to_string(),
        added_at: ts,
        added_by: my_public_key.to_string(),
    };
    members.push(me);

    let membership = OrgMembership {
        org_name: invite.org_name.clone(),
        org_hash: org_hash.clone(),
        org_public_key: invite.org_public_key.clone(),
        org_secret_key: None, // Joiners don't get the secret key
        org_e2e_secret: invite.org_e2e_secret.clone(),
        role: OrgRole::Member,
        members,
        created_at: ts,
        joined_at: ts,
    };

    let value = serde_json::to_vec(&membership)?;
    tree.insert(key.as_bytes(), value)
        .map_err(|e| FoldDbError::Database(format!("Failed to store org membership: {}", e)))?;

    Ok(membership)
}

/// List all organizations this node belongs to.
pub fn list_orgs(pool: &Arc<SledPool>) -> Result<Vec<OrgMembership>, FoldDbError> {
    let tree = org_tree(pool)?;
    let mut orgs = Vec::new();

    for entry in tree.iter() {
        let (_, value) = entry
            .map_err(|e| FoldDbError::Database(format!("Failed to iterate org tree: {}", e)))?;
        let membership: OrgMembership = serde_json::from_slice(&value)?;
        orgs.push(membership);
    }

    Ok(orgs)
}

/// Get a single organization by its hash.
pub fn get_org(pool: &Arc<SledPool>, org_hash: &str) -> Result<Option<OrgMembership>, FoldDbError> {
    let tree = org_tree(pool)?;
    let key = org_key(org_hash);

    match tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read org tree: {}", e)))?
    {
        Some(value) => {
            let membership: OrgMembership = serde_json::from_slice(&value)?;
            Ok(Some(membership))
        }
        None => Ok(None),
    }
}

/// Add a member to an existing organization. Only admins should call this.
pub fn add_member(pool: &Arc<SledPool>, org_hash: &str, member: OrgMemberInfo) -> Result<(), FoldDbError> {
    let tree = org_tree(pool)?;
    let key = org_key(org_hash);

    let value = tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read org tree: {}", e)))?
        .ok_or_else(|| {
            FoldDbError::Database(format!("Organization with hash '{}' not found", org_hash))
        })?;

    let mut membership: OrgMembership = serde_json::from_slice(&value)?;

    // Check for duplicate
    if membership
        .members
        .iter()
        .any(|m| m.node_public_key == member.node_public_key)
    {
        return Err(FoldDbError::Database(format!(
            "Member with public key '{}' already exists in org",
            member.node_public_key
        )));
    }

    membership.members.push(member);

    let updated = serde_json::to_vec(&membership)?;
    tree.insert(key.as_bytes(), updated)
        .map_err(|e| FoldDbError::Database(format!("Failed to update org membership: {}", e)))?;

    Ok(())
}

/// Remove a member from an organization by their node public key.
pub fn remove_member(
    pool: &Arc<SledPool>,
    org_hash: &str,
    node_public_key: &str,
) -> Result<(), FoldDbError> {
    let tree = org_tree(pool)?;
    let key = org_key(org_hash);

    let value = tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read org tree: {}", e)))?
        .ok_or_else(|| {
            FoldDbError::Database(format!("Organization with hash '{}' not found", org_hash))
        })?;

    let mut membership: OrgMembership = serde_json::from_slice(&value)?;

    let original_len = membership.members.len();
    membership
        .members
        .retain(|m| m.node_public_key != node_public_key);

    if membership.members.len() == original_len {
        return Err(FoldDbError::Database(format!(
            "Member with public key '{}' not found in org",
            node_public_key
        )));
    }

    let updated = serde_json::to_vec(&membership)?;
    tree.insert(key.as_bytes(), updated)
        .map_err(|e| FoldDbError::Database(format!("Failed to update org membership: {}", e)))?;

    Ok(())
}

/// Generate an invite bundle for an organization. Requires admin access (has org_secret_key).
pub fn generate_invite(pool: &Arc<SledPool>, org_hash: &str) -> Result<OrgInviteBundle, FoldDbError> {
    let membership = get_org(pool, org_hash)?.ok_or_else(|| {
        FoldDbError::Database(format!("Organization with hash '{}' not found", org_hash))
    })?;

    if membership.role != OrgRole::Admin {
        return Err(FoldDbError::Permission(
            "Only admins can generate invites".to_string(),
        ));
    }

    Ok(OrgInviteBundle {
        org_name: membership.org_name,
        org_hash: org_hash.to_string(),
        org_public_key: membership.org_public_key,
        org_e2e_secret: membership.org_e2e_secret,
        members: membership.members,
    })
}

/// Delete an organization from local storage.
pub fn delete_org(pool: &Arc<SledPool>, org_hash: &str) -> Result<(), FoldDbError> {
    let tree = org_tree(pool)?;
    let key = org_key(org_hash);

    let existed = tree
        .remove(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to delete org: {}", e)))?;

    if existed.is_none() {
        return Err(FoldDbError::Database(format!(
            "Organization with hash '{}' not found",
            org_hash
        )));
    }

    Ok(())
}
