use crate::atom::molecule_types::{MoleculeStatus, MoleculeUpdate};
use chrono::{DateTime, Utc};

/// A trait defining the common behavior for molecule references.
///
/// This trait provides the interface for both single molecule references
/// and collections of molecule references.
pub trait MoleculeBehavior {
    /// Returns the unique identifier of this reference
    fn uuid(&self) -> &str;

    /// Returns the timestamp of the last update
    fn updated_at(&self) -> DateTime<Utc>;

    /// Returns the status of this reference
    fn status(&self) -> &MoleculeStatus;

    /// Sets the status of this reference
    fn set_status(&mut self, status: &MoleculeStatus, source_pub_key: String);

    /// Returns the update history
    fn update_history(&self) -> &Vec<MoleculeUpdate>;
}