use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShareScope {
    Schema(String),
    SchemaField(String, String),
    AllSchemas,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRule {
    pub rule_id: String,
    pub recipient_pubkey: String,
    pub recipient_display_name: String,
    pub scope: ShareScope,
    pub share_prefix: String,
    pub share_e2e_secret: Vec<u8>,
    pub active: bool,
    pub created_at: u64,
    pub writer_pubkey: String,
    pub signature: String,
}

impl ShareRule {
    pub fn scope_matches(&self, target_schema_name: &str) -> bool {
        match &self.scope {
            ShareScope::AllSchemas => true,
            ShareScope::Schema(schema) => schema == target_schema_name,
            ShareScope::SchemaField(schema, _) => schema == target_schema_name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInvite {
    pub sender_pubkey: String,
    pub sender_display_name: String,
    pub share_prefix: String,
    pub share_e2e_secret: Vec<u8>,
    pub scope_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSubscription {
    pub sender_pubkey: String,
    pub share_prefix: String,
    pub share_e2e_secret: Vec<u8>,
    pub accepted_at: u64,
    pub active: bool,
}
