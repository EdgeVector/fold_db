use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoleculeStatus {
    Active,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeUpdate {
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) status: MoleculeStatus,
    pub(crate) source_pub_key: String,
}