use serde::{Deserialize, Serialize};

/// Represents a node's membership in an organization.
/// Stored locally in Sled under the "org_memberships" tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMembership {
    /// Human-readable organization name
    pub org_name: String,
    /// Hex-encoded SHA256 hash of the org public key bytes — used as the S3 prefix
    pub org_hash: String,
    /// Base64-encoded Ed25519 public key for the organization
    pub org_public_key: String,
    /// Base64-encoded Ed25519 secret key (only present for the admin who created the org)
    pub org_secret_key: Option<String>,
    /// Base64-encoded AES-256-GCM shared secret for E2E encryption of org data
    pub org_e2e_secret: String,
    /// Role of this node in the organization
    pub role: OrgRole,
    /// Known members of the organization
    pub members: Vec<OrgMemberInfo>,
    /// Unix timestamp (seconds) when the org was created
    pub created_at: u64,
    /// Unix timestamp (seconds) when this node joined the org
    pub joined_at: u64,
}

/// Information about a single member of an organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberInfo {
    /// Base64-encoded Ed25519 public key of the member's node
    pub node_public_key: String,
    /// Human-readable display name for this member
    pub display_name: String,
    /// Unix timestamp (seconds) when this member was added
    pub added_at: u64,
    /// Base64-encoded public key of the admin who added this member
    pub added_by: String,
}

/// Role within an organization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrgRole {
    /// Can add/remove members, generate invites, delete the org
    Admin,
    /// Can read/write org-tagged schemas but cannot manage membership
    Member,
}

/// Bundle sent to a new member to join an organization.
/// Contains everything needed to participate in org sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInviteBundle {
    /// Human-readable organization name
    pub org_name: String,
    /// Hex-encoded SHA256 hash of the org public key — used for decline and display
    pub org_hash: String,
    /// Base64-encoded Ed25519 public key for the organization
    pub org_public_key: String,
    /// Base64-encoded AES-256-GCM shared secret for E2E encryption
    pub org_e2e_secret: String,
    /// Current member list at time of invite generation
    pub members: Vec<OrgMemberInfo>,
}
