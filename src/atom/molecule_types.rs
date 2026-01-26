use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub enum MoleculeStatus {
    Active,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeUpdate {
    #[schema(value_type = String, format = "date-time")]
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) status: MoleculeStatus,
    pub(crate) source_pub_key: String,
}

/// Helper that updates status related fields.
pub fn apply_status_update(
    status_field: &mut MoleculeStatus,
    updated_at_field: &mut DateTime<Utc>,
    history: &mut Vec<MoleculeUpdate>,
    status: &MoleculeStatus,
    source_pub_key: String,
) {
    let now = Utc::now();
    *status_field = status.clone();
    *updated_at_field = now;
    history.push(MoleculeUpdate {
        timestamp: now,
        status: status.clone(),
        source_pub_key,
    });
}
