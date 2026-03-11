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
pub mod engine;
pub mod error;
pub mod log;
pub mod s3;
pub mod snapshot;

pub use engine::{SyncConfig, SyncEngine, SyncState};
pub use error::{SyncError, SyncResult};

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
    /// `api_url` and `api_key` are reused. Device ID is auto-generated
    /// and persisted in the local database.
    pub fn from_exemem(api_url: &str, api_key: &str) -> Self {
        let device_id = std::env::var("FOLD_SYNC_DEVICE_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        Self {
            auth_url: api_url.to_string(),
            auth: auth::SyncAuth::ApiKey(api_key.to_string()),
            device_id,
            config: None,
        }
    }
}
