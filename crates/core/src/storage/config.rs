use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Error type for configuration parsing
#[derive(Debug)]
pub enum ConfigError {
    MissingVariable(String),
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVariable(var) => write!(f, "Missing environment variable: {}", var),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid configuration value: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration for cloud sync (Exemem encrypted S3 backup).
///
/// **Field persistence model:** only `api_url` and `p2p_sync` are serialized
/// to disk. `api_key`, `session_token`, and `user_hash` are per-device
/// secrets that live in the host application's credential store (e.g.
/// fold_db_node's `credentials.json` / `credentials.enc`) and are hydrated
/// into this struct at runtime — typically in the node-creation path
/// before the sync engine is built. They are marked
/// `#[serde(skip_serializing)]` so `node_config.json` is safe to back up
/// or share.
///
/// Existing config files that contain these fields (from before this
/// change) still deserialize cleanly — serde reads them, and the next
/// save strips them out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    /// Exemem API URL (sync routes at /api/sync/*). Persisted to disk.
    pub api_url: String,
    /// API key for authentication. Runtime-hydrated from the credential
    /// store; never persisted to `node_config.json`.
    #[serde(default, skip_serializing)]
    pub api_key: String,
    /// Session token for authenticated API access. Runtime-hydrated.
    #[serde(default, skip_serializing)]
    pub session_token: Option<String>,
    /// User hash derived from public key. Runtime-hydrated.
    #[serde(default, skip_serializing)]
    pub user_hash: Option<String>,
    /// Optional ephemeral peer-to-peer device sync (free; uses R2 with
    /// 24h lifecycle). When `None`, p2p sync is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p2p_sync: Option<P2pSyncConfig>,
}

/// Configuration for ephemeral peer-to-peer sync between a single user's
/// devices.
///
/// Sled writes from this device are appended to a small encrypted log and
/// pushed to each peer's `<user_hash>/p2p/<me>__<peer>/<seq>.enc` mailbox.
/// A background loop also polls each peer's outgoing mailbox
/// (`<user_hash>/p2p/<peer>__<me>/`) every `poll_interval_ms` and applies
/// new entries to the local store.
///
/// R2 lifecycle (server-side) expires p2p objects after 24h, so this is a
/// best-effort low-latency channel layered on top of the durable log /
/// snapshot sync — explicit ACK/delete is intentionally not implemented;
/// the 24h lifecycle reclaims storage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2pSyncConfig {
    /// Other device IDs belonging to the same user. The local device pushes
    /// every Sled write to each of these peers' inbound mailboxes and pulls
    /// from each peer's outbound mailbox.
    #[serde(default)]
    pub peer_device_ids: Vec<String>,
    /// How often to poll each peer's outbound mailbox, in milliseconds.
    /// Default: 30_000 (30s).
    #[serde(default = "default_p2p_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

fn default_p2p_poll_interval_ms() -> u64 {
    30_000
}

impl Default for P2pSyncConfig {
    fn default() -> Self {
        Self {
            peer_device_ids: Vec::new(),
            poll_interval_ms: default_p2p_poll_interval_ms(),
        }
    }
}

/// Storage configuration — always local Sled, optionally with cloud sync.
///
/// Uses a custom deserializer to support both the new format and the legacy
/// `{"type": "local", ...}` / `{"type": "exemem", ...}` JSON formats.
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseConfig {
    /// Path to the local Sled database directory
    pub path: PathBuf,
    /// Optional cloud sync configuration (Exemem encrypted S3 backup)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_sync: Option<CloudSyncConfig>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            path: PathBuf::from("data"),
            cloud_sync: None,
        }
    }
}

impl DatabaseConfig {
    /// Create a local-only config
    pub fn local(path: PathBuf) -> Self {
        DatabaseConfig {
            path,
            cloud_sync: None,
        }
    }

    /// Create a config with cloud sync enabled
    pub fn with_cloud_sync(path: PathBuf, cloud_sync: CloudSyncConfig) -> Self {
        DatabaseConfig {
            path,
            cloud_sync: Some(cloud_sync),
        }
    }

    /// Check if cloud sync is enabled
    pub fn has_cloud_sync(&self) -> bool {
        self.cloud_sync.is_some()
    }

    /// Creates DatabaseConfig from environment variables:
    /// - FOLD_STORAGE_PATH: path for local storage (default: "data")
    /// - FOLD_STORAGE_MODE: "local" (default) or "exemem"
    /// - For exemem mode: EXEMEM_API_URL, EXEMEM_API_KEY
    pub fn from_env() -> Result<Self, ConfigError> {
        let path = env::var("FOLD_STORAGE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));

        let mode = env::var("FOLD_STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

        let cloud_sync = match mode.to_lowercase().as_str() {
            "local" => None,
            "exemem" => {
                let api_url = env::var("EXEMEM_API_URL")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_URL".to_string()))?;
                let api_key = env::var("EXEMEM_API_KEY")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_KEY".to_string()))?;
                Some(CloudSyncConfig {
                    api_url,
                    api_key,
                    session_token: env::var("EXEMEM_SESSION_TOKEN").ok(),
                    user_hash: env::var("EXEMEM_USER_HASH").ok(),
                    p2p_sync: None,
                })
            }
            _ => {
                return Err(ConfigError::InvalidValue(format!(
                    "Invalid FOLD_STORAGE_MODE: '{}'. Must be 'local' or 'exemem'",
                    mode
                )))
            }
        };

        Ok(DatabaseConfig { path, cloud_sync })
    }
}

/// Custom deserializer that handles both new format and legacy tagged enum format.
///
/// New format:
/// ```json
/// { "path": "/data", "cloud_sync": { "api_url": "...", "api_key": "..." } }
/// ```
///
/// Legacy formats (auto-migrated):
/// ```json
/// { "type": "local", "path": "/data" }
/// { "type": "exemem", "api_url": "...", "api_key": "..." }
/// ```
impl<'de> Deserialize<'de> for DatabaseConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // Check if this is the legacy tagged format
        if let Some(type_tag) = value.get("type").and_then(|v| v.as_str()) {
            match type_tag {
                "local" => {
                    let path = value
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("data"));
                    Ok(DatabaseConfig {
                        path,
                        cloud_sync: None,
                    })
                }
                "exemem" => {
                    // `api_url` is the only field required on disk — the
                    // per-device secrets (api_key, session_token, user_hash)
                    // are runtime-hydrated from the credential store, so
                    // their absence is expected, not an error.
                    let api_url = value
                        .get("api_url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            serde::de::Error::custom(
                                "exemem database config missing required string field 'api_url'",
                            )
                        })?
                        .to_string();
                    let api_key = value
                        .get("api_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let session_token = value
                        .get("session_token")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let user_hash = value
                        .get("user_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // TODO(storage-path): the legacy `{"type":"exemem"}` JSON has no
                    // `path` field, so the path is recovered from FOLD_STORAGE_PATH.
                    // Downstream consumers (fold_db_node's run.sh, org-test.sh,
                    // smoke-test-dmg.sh, src/bin/folddb/main.rs, and
                    // src-tauri/src/lib.rs) still emit this shape; once they are
                    // migrated to the new `{path, cloud_sync}` format, drop this
                    // legacy branch entirely.
                    let path = std::env::var("FOLD_STORAGE_PATH")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("data"));

                    Ok(DatabaseConfig {
                        path,
                        cloud_sync: Some(CloudSyncConfig {
                            api_url,
                            api_key,
                            session_token,
                            user_hash,
                            p2p_sync: None,
                        }),
                    })
                }
                other => Err(serde::de::Error::custom(format!(
                    "Unknown database type: '{}'. Supported: 'local', 'exemem'",
                    other
                ))),
            }
        } else {
            // New format: direct struct fields
            let path = value
                .get("path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data"));

            // If `cloud_sync` is present, it must parse — silently dropping a
            // malformed block (e.g. missing `api_url`) would turn a broken
            // cloud-mode config into an apparently-healthy local-mode one.
            let cloud_sync = value
                .get("cloud_sync")
                .map(|v| serde_json::from_value::<CloudSyncConfig>(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;

            Ok(DatabaseConfig { path, cloud_sync })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_new_format() {
        let json = r#"{"path": "/data/db", "cloud_sync": {"api_url": "https://api.example.com", "api_key": "key123"}}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_some());
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "key123");
    }

    #[test]
    fn test_deserialize_new_format_no_sync() {
        let json = r#"{"path": "/data/db"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_none());
    }

    #[test]
    fn test_deserialize_legacy_local() {
        let json = r#"{"type": "local", "path": "/data/db"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_none());
    }

    #[test]
    fn test_deserialize_legacy_exemem() {
        let json =
            r#"{"type": "exemem", "api_url": "https://api.example.com", "api_key": "key123"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert!(config.cloud_sync.is_some());
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "key123");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = DatabaseConfig::with_cloud_sync(
            PathBuf::from("/data"),
            CloudSyncConfig {
                api_url: "https://api.example.com".to_string(),
                api_key: "key123".to_string(),
                session_token: None,
                user_hash: None,
                p2p_sync: None,
            },
        );
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DatabaseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, config.path);
        assert!(deserialized.cloud_sync.is_some());
    }

    #[test]
    fn test_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, PathBuf::from("data"));
        assert!(config.cloud_sync.is_none());
    }

    #[test]
    fn serialize_omits_per_device_secrets() {
        let config = DatabaseConfig::with_cloud_sync(
            PathBuf::from("/data"),
            CloudSyncConfig {
                api_url: "https://api.example.com".to_string(),
                api_key: "secret_api_key".to_string(),
                session_token: Some("secret_token".to_string()),
                user_hash: Some("user_abc".to_string()),
                p2p_sync: None,
            },
        );
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            json.contains("api_url"),
            "api_url must be persisted: {}",
            json
        );
        assert!(
            !json.contains("secret_api_key"),
            "api_key must NOT be persisted to disk: {}",
            json
        );
        assert!(
            !json.contains("secret_token"),
            "session_token must NOT be persisted to disk: {}",
            json
        );
        assert!(
            !json.contains("user_abc"),
            "user_hash must NOT be persisted to disk: {}",
            json
        );
    }

    #[test]
    fn deserialize_tolerates_legacy_files_with_secrets() {
        // Files written before the skip_serializing change include api_key,
        // session_token, and user_hash. They must still deserialize cleanly —
        // the next save will strip them.
        let json = r#"{
            "path": "/data/db",
            "cloud_sync": {
                "api_url": "https://api.example.com",
                "api_key": "legacy_key",
                "session_token": "legacy_token",
                "user_hash": "legacy_hash"
            }
        }"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "legacy_key");
        assert_eq!(sync.session_token.as_deref(), Some("legacy_token"));
        assert_eq!(sync.user_hash.as_deref(), Some("legacy_hash"));
    }

    #[test]
    fn deserialize_new_file_without_secrets_defaults_fields() {
        // Files written after this change contain only api_url. The runtime
        // must see api_key = "" and session_token/user_hash = None until the
        // credential-store hydration runs.
        let json = r#"{
            "path": "/data/db",
            "cloud_sync": { "api_url": "https://api.example.com" }
        }"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "");
        assert!(sync.session_token.is_none());
        assert!(sync.user_hash.is_none());
    }

    #[test]
    fn legacy_exemem_missing_api_url_errors() {
        // The only required field in the legacy exemem tag is api_url. A file
        // missing it is broken — silently defaulting to empty string masks
        // the bug and fails later with a confusing runtime error.
        let json = r#"{"type": "exemem", "api_key": "key123"}"#;
        let err = serde_json::from_str::<DatabaseConfig>(json).unwrap_err();
        assert!(
            err.to_string().contains("api_url"),
            "error should mention api_url: {}",
            err
        );
    }

    #[test]
    fn new_format_malformed_cloud_sync_errors() {
        // If cloud_sync is present, it must parse. Silently dropping a
        // malformed block turns a broken cloud-mode config into an
        // apparently-healthy local-mode one.
        let json = r#"{
            "path": "/data/db",
            "cloud_sync": { "api_key": "key_without_url" }
        }"#;
        let err = serde_json::from_str::<DatabaseConfig>(json).unwrap_err();
        assert!(
            err.to_string().contains("api_url"),
            "error should mention missing api_url: {}",
            err
        );
    }

    #[test]
    fn p2p_sync_config_default_poll_interval() {
        // Files written before the p2p feature don't include `p2p_sync`,
        // so it must default to `None` rather than failing to parse.
        let json = r#"{
            "path": "/data/db",
            "cloud_sync": { "api_url": "https://api.example.com" }
        }"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        let sync = config.cloud_sync.unwrap();
        assert!(sync.p2p_sync.is_none());
    }

    #[test]
    fn p2p_sync_config_round_trips() {
        let p2p = P2pSyncConfig {
            peer_device_ids: vec!["device-b".to_string(), "device-c".to_string()],
            poll_interval_ms: 15_000,
        };
        let config = DatabaseConfig::with_cloud_sync(
            PathBuf::from("/data"),
            CloudSyncConfig {
                api_url: "https://api.example.com".to_string(),
                api_key: "k".to_string(),
                session_token: None,
                user_hash: None,
                p2p_sync: Some(p2p.clone()),
            },
        );
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            json.contains("p2p_sync"),
            "p2p_sync block must be persisted: {}",
            json
        );
        let parsed: DatabaseConfig = serde_json::from_str(&json).unwrap();
        let parsed_p2p = parsed.cloud_sync.unwrap().p2p_sync.unwrap();
        assert_eq!(parsed_p2p, p2p);
    }

    #[test]
    fn p2p_sync_config_uses_default_poll_interval_when_omitted() {
        let json = r#"{
            "peer_device_ids": ["d1"]
        }"#;
        let p2p: P2pSyncConfig = serde_json::from_str(json).unwrap();
        assert_eq!(p2p.peer_device_ids, vec!["d1".to_string()]);
        assert_eq!(p2p.poll_interval_ms, 30_000);
    }
}
