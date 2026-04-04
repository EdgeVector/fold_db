//! S3-based sync for encrypted Sled databases.
//!
//! The sync module replicates a local Sled database to S3 as encrypted blobs.
//! The server never sees plaintext data — only opaque ciphertext.
//!
//! ## Architecture
//!
//! ```text
//! fold_db (local Sled)
//!       │
//!       ▼
//! SyncEngine
//!   ├── Records KvStore ops as encrypted log entries
//!   ├── Uploads log entries to S3 via presigned URLs
//!   ├── Compacts logs into snapshots every 100 entries
//!   └── Manages single-device write lock
//!       │
//!       ▼
//! Auth Lambda (thin)
//!   ├── Validates API key / bearer token
//!   ├── Returns presigned S3 URLs scoped to /{user_hash}/*
//!   └── Manages device lock (S3 object)
//!       │
//!       ▼
//! S3 Bucket
//!   /{user_hash}/
//!     snapshots/
//!       latest.enc          — most recent snapshot
//!       {seq}.enc           — historical snapshots
//!     log/
//!       {seq}.enc           — individual encrypted log entries
//!     lock.json             — device lock file
//! ```
//!
//! ## Data Flow
//!
//! **Write path:** mutation → Sled write → SyncEngine records op →
//!   timer fires → seal entries → get presigned URLs → upload to S3
//!
//! **Read path (bootstrap):** download latest.enc → decrypt → restore to Sled →
//!   list log entries after snapshot seq → download + replay each
//!
//! ## Security
//!
//! - All data encrypted client-side with AES-256-GCM before upload
//! - E2E key never leaves the client device
//! - Server sees only opaque encrypted blobs
//! - Presigned URLs expire after 15 minutes
//! - No AWS credentials on the client

pub mod auth;
pub mod conflict;
pub mod engine;
pub mod error;
pub mod log;
pub mod org_sync;
pub mod s3;
pub mod snapshot;

pub use conflict::{ConflictRecord, ConflictResolution, ConflictSide};
pub use engine::{SyncConfig, SyncEngine, SyncState, SyncStatus};
pub use error::{SyncError, SyncResult};
pub use org_sync::{SyncDestination, SyncPartitioner};

/// Configuration needed to enable S3 sync.
///
/// Derived automatically from the Exemem credentials — no extra config needed.
/// The sync auth Lambda shares the same API URL and API key as the Exemem platform.
#[derive(Clone, Debug)]
pub struct SyncSetup {
    /// Exemem API base URL (sync routes live at /api/sync/*).
    pub auth_url: String,
    /// Authentication credential (same as Exemem auth).
    pub auth: auth::SyncAuth,
    /// Unique identifier for this device (auto-generated if not set).
    pub device_id: String,
    /// Sync tuning parameters. Uses defaults if None.
    pub config: Option<SyncConfig>,
}

impl SyncSetup {
    /// Create SyncSetup from Exemem credentials.
    ///
    /// The sync auth Lambda is part of the Exemem platform, so the same
    /// `api_url` and `api_key` are reused. Device ID is read from the
    /// `FOLD_SYNC_DEVICE_ID` env var, or persisted to a `.device_id` file
    /// in `data_dir` so it survives restarts.
    pub fn from_exemem(api_url: &str, api_key: &str, data_dir: &str) -> Self {
        let device_id = std::env::var("FOLD_SYNC_DEVICE_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| get_or_create_device_id(data_dir));

        Self {
            auth_url: api_url.to_string(),
            auth: auth::SyncAuth::ApiKey(api_key.to_string()),
            device_id,
            config: None,
        }
    }
}

/// Read a persisted device ID from `<data_dir>/.device_id`, or generate a new
/// UUID and write it there so the same ID is used across restarts.
fn get_or_create_device_id(data_dir: &str) -> String {
    let device_id_path = std::path::Path::new(data_dir).join(".device_id");

    // Try to read existing device ID
    if let Ok(id) = std::fs::read_to_string(&device_id_path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }

    // Generate and persist new device ID
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = device_id_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&device_id_path, &id) {
        eprintln!(
            "WARNING: Failed to persist device ID to {}: {}. \
             A new device ID will be generated on next restart.",
            device_id_path.display(),
            e
        );
    }
    id
}
