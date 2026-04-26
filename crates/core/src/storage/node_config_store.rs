use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};

use super::sled_pool::SledPool;
use crate::crypto::{decrypt_envelope, encrypt_envelope};
use std::sync::Arc;

const TREE_NAME: &str = "node_config";

/// Prefix marker for encrypted string values stored in this tree.
/// Matches the convention used by `EncryptingKvStore` so that the stored
/// value is valid UTF-8 and round-trips through the existing string-based
/// get/set API without changing the wire format of untouched fields.
const ENCRYPTED_PREFIX: &str = "ENC:";

/// Thin wrapper around a SledPool for storing node configuration.
///
/// Sensitive fields (currently: the node's Ed25519 private key) are
/// transparently encrypted at rest with AES-256-GCM when a 32-byte
/// encryption key is supplied via [`NodeConfigStore::with_crypto_key`].
/// Reads transparently handle both legacy plaintext values (pre-migration)
/// and encrypted values, and writes always produce encrypted output when
/// a key is configured. Migration is performed implicitly on the next
/// write — no explicit re-encryption pass is required.
///
/// All runtime config (identity, cloud credentials, AI settings) lives here.
#[derive(Clone)]
pub struct NodeConfigStore {
    pool: Arc<SledPool>,
    /// Optional 32-byte key used to encrypt sensitive fields at rest.
    /// When `None`, sensitive fields are stored in plaintext (legacy mode)
    /// and reads of previously-encrypted values will fail loudly rather
    /// than silently returning ciphertext.
    identity_key: Option<[u8; 32]>,
}

impl NodeConfigStore {
    pub fn new(pool: Arc<SledPool>) -> Result<Self, sled::Error> {
        Self::with_crypto_key(pool, None)
    }

    /// Create a store with an optional at-rest encryption key for sensitive
    /// fields. Callers that can read the node's identity (e.g. the factory,
    /// migration path, discovery config loader) should pass `Some(key)` so
    /// the private key never hits disk in plaintext.
    pub fn with_crypto_key(
        pool: Arc<SledPool>,
        identity_key: Option<[u8; 32]>,
    ) -> Result<Self, sled::Error> {
        // Validate that we can open the tree by doing a test acquire
        let guard = pool.acquire_arc().map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "Failed to acquire pool: {}",
                e
            )))
        })?;
        guard.db().open_tree(TREE_NAME)?;
        drop(guard);
        Ok(Self { pool, identity_key })
    }

    /// Encrypt a plaintext string into the `ENC:<base64>` wire format.
    fn encrypt_sensitive(&self, plaintext: &str) -> Result<String, sled::Error> {
        let key = self.identity_key.ok_or_else(|| {
            sled::Error::Io(std::io::Error::other(
                "NodeConfigStore has no identity encryption key configured; \
                 refusing to write sensitive field in plaintext",
            ))
        })?;
        let ciphertext = encrypt_envelope(&key, plaintext.as_bytes()).map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "identity encryption failed: {}",
                e
            )))
        })?;
        Ok(format!("{}{}", ENCRYPTED_PREFIX, B64.encode(&ciphertext)))
    }

    /// Decrypt a stored value. Transparently handles pre-migration plaintext
    /// (any value that does NOT start with `ENC:` is returned verbatim) and
    /// the `ENC:<base64>` wire format. Returns an error only if an encrypted
    /// value is encountered without a configured key, or decryption fails.
    fn decrypt_sensitive(&self, stored: String) -> Result<String, sled::Error> {
        if !stored.starts_with(ENCRYPTED_PREFIX) {
            // Legacy plaintext written by a pre-encryption build.
            return Ok(stored);
        }
        let b64_part = &stored[ENCRYPTED_PREFIX.len()..];
        let ciphertext = B64.decode(b64_part).map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "identity ciphertext base64 decode failed: {}",
                e
            )))
        })?;
        let key = self.identity_key.ok_or_else(|| {
            sled::Error::Io(std::io::Error::other(
                "encrypted identity field found in Sled but no decryption key \
                 configured on this NodeConfigStore handle",
            ))
        })?;
        let plaintext_bytes = decrypt_envelope(&key, &ciphertext).map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "identity decryption failed: {}",
                e
            )))
        })?;
        String::from_utf8(plaintext_bytes).map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "identity plaintext is not valid UTF-8: {}",
                e
            )))
        })
    }

    fn tree(&self) -> sled::Tree {
        let guard = self.pool.acquire_arc().expect("SledPool acquire failed");
        let tree = guard
            .db()
            .open_tree(TREE_NAME)
            .expect("Failed to open node_config tree");
        tree
    }

    // --- Generic key-value ---

    pub fn get(&self, key: &str) -> Option<String> {
        self.tree()
            .get(key)
            .ok()?
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), sled::Error> {
        let tree = self.tree();
        tree.insert(key, value.as_bytes())?;
        tree.flush()?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), sled::Error> {
        let tree = self.tree();
        tree.remove(key)?;
        tree.flush()?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.tree().is_empty()
    }

    // --- Cloud config (shared across devices) ---
    //
    // Only `api_url` and `user_hash` are stored in Sled because they are
    // safe to sync across devices. Per-device secrets (`api_key`,
    // `session_token`) live exclusively in credentials.json, managed by
    // the fold_db_node layer.

    pub fn get_cloud_config(&self) -> Option<CloudCredentials> {
        if self.get("cloud:enabled")?.as_str() != "true" {
            return None;
        }
        Some(CloudCredentials {
            api_url: self.get("cloud:api_url")?,
            user_hash: self.get("cloud:user_hash"),
        })
    }

    pub fn set_cloud_config(&self, creds: &CloudCredentials) -> Result<(), sled::Error> {
        self.set("cloud:api_url", &creds.api_url)?;
        if let Some(ref uh) = creds.user_hash {
            self.set("cloud:user_hash", uh)?;
        }
        // Don't set cloud:enabled here — only the factory should set it when
        // sync is actually configured with a valid API key. Setting it during
        // registration (without an API key) causes the factory to bootstrap
        // sync on restart, which deadlocks.
        Ok(())
    }

    pub fn is_cloud_enabled(&self) -> bool {
        self.get("cloud:enabled").as_deref() == Some("true")
    }

    // --- Node identity ---
    //
    // `identity:private_key` is encrypted at rest with AES-256-GCM when a
    // crypto key is configured on this store handle. The public key is
    // stored in plaintext — there is no confidentiality requirement and
    // callers (e.g. discovery_config) sometimes need to read it without
    // holding the encryption key.

    pub fn get_identity(&self) -> Option<NodeIdentity> {
        let public_key = self.get("identity:public_key")?;
        let stored_private = self.get("identity:private_key")?;
        let private_key = match self.decrypt_sensitive(stored_private) {
            Ok(p) => p,
            Err(e) => {
                log::error!("failed to load encrypted node identity: {}", e);
                return None;
            }
        };
        Some(NodeIdentity {
            private_key,
            public_key,
        })
    }

    pub fn set_identity(&self, id: &NodeIdentity) -> Result<(), sled::Error> {
        let encrypted_private = self.encrypt_sensitive(&id.private_key)?;
        self.set("identity:private_key", &encrypted_private)?;
        self.set("identity:public_key", &id.public_key)?;
        Ok(())
    }

    /// Read the raw stored value for `identity:private_key` without
    /// decryption. Exposed for tests that need to verify the on-disk
    /// representation is ciphertext.
    #[doc(hidden)]
    pub fn raw_identity_private_key(&self) -> Option<String> {
        self.get("identity:private_key")
    }

    // --- Identity card (display name, contact) ---

    pub fn get_display_name(&self) -> Option<String> {
        self.get("identity:display_name")
    }

    pub fn set_display_name(&self, name: &str) -> Result<(), sled::Error> {
        self.set("identity:display_name", name)
    }

    pub fn get_contact_hint(&self) -> Option<String> {
        self.get("identity:contact_hint")
    }

    pub fn set_contact_hint(&self, hint: &str) -> Result<(), sled::Error> {
        self.set("identity:contact_hint", hint)
    }

    pub fn get_birthday(&self) -> Option<String> {
        self.get("identity:birthday")
    }

    pub fn set_birthday(&self, birthday: &str) -> Result<(), sled::Error> {
        self.set("identity:birthday", birthday)
    }

    // --- AI config ---

    pub fn get_ai_config(&self) -> Option<AiConfig> {
        // Return None if no provider configured
        let provider = self.get("ai:provider")?;
        Some(AiConfig {
            provider,
            anthropic_key: self.get("ai:anthropic_key"),
            anthropic_model: self.get("ai:anthropic_model"),
            anthropic_base_url: self.get("ai:anthropic_base_url"),
            ollama_model: self.get("ai:ollama_model"),
            ollama_url: self.get("ai:ollama_url"),
            ollama_vision_model: self.get("ai:ollama_vision_model"),
        })
    }

    pub fn set_ai_config(&self, config: &AiConfig) -> Result<(), sled::Error> {
        self.set("ai:provider", &config.provider)?;
        // Set optional fields, delete if None
        for (key, val) in [
            ("ai:anthropic_key", &config.anthropic_key),
            ("ai:anthropic_model", &config.anthropic_model),
            ("ai:anthropic_base_url", &config.anthropic_base_url),
            ("ai:ollama_model", &config.ollama_model),
            ("ai:ollama_url", &config.ollama_url),
            ("ai:ollama_vision_model", &config.ollama_vision_model),
        ] {
            match val {
                Some(v) => self.set(key, v)?,
                None => self.delete(key)?,
            }
        }
        Ok(())
    }

    // --- Schema service URL ---

    pub fn get_schema_service_url(&self) -> Option<String> {
        self.get("schema_service_url")
    }

    pub fn set_schema_service_url(&self, url: &str) -> Result<(), sled::Error> {
        self.set("schema_service_url", url)
    }
}

/// Cloud configuration stored in Sled (safe to sync across devices).
///
/// Per-device secrets (api_key, session_token) are NOT stored here.
/// They live in credentials.json, managed by the fold_db_node layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudCredentials {
    pub api_url: String,
    pub user_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub anthropic_key: Option<String>,
    pub anthropic_model: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_vision_model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (NodeConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(SledPool::new(dir.path().to_path_buf()));
        // Tests exercise encrypted identity by default so the common
        // path (set_identity → get_identity) covers the encryption flow.
        let store = NodeConfigStore::with_crypto_key(pool, Some([0x42u8; 32])).unwrap();
        // Return dir to keep tempdir alive
        (store, dir)
    }

    fn temp_store_no_crypto() -> (NodeConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(SledPool::new(dir.path().to_path_buf()));
        let store = NodeConfigStore::new(pool).unwrap();
        (store, dir)
    }

    #[test]
    fn test_get_set_string() {
        let (store, _dir) = temp_store();
        assert!(store.get("foo").is_none());
        store.set("foo", "bar").unwrap();
        assert_eq!(store.get("foo").unwrap(), "bar");
    }

    #[test]
    fn test_delete() {
        let (store, _dir) = temp_store();
        store.set("foo", "bar").unwrap();
        store.delete("foo").unwrap();
        assert!(store.get("foo").is_none());
    }

    #[test]
    fn test_cloud_config_round_trip() {
        let (store, _dir) = temp_store();
        assert!(store.get_cloud_config().is_none());
        assert!(!store.is_cloud_enabled());

        let creds = CloudCredentials {
            api_url: "https://example.com".into(),
            user_hash: Some("deadbeef".into()),
        };
        store.set_cloud_config(&creds).unwrap();
        // set_cloud_config deliberately doesn't set cloud:enabled —
        // only the factory sets it when sync is fully configured.
        assert!(!store.is_cloud_enabled());
        store.set("cloud:enabled", "true").unwrap();
        assert!(store.is_cloud_enabled());

        let loaded = store.get_cloud_config().unwrap();
        assert_eq!(loaded.api_url, "https://example.com");
        assert_eq!(loaded.user_hash.unwrap(), "deadbeef");
    }

    #[test]
    fn test_identity_round_trip() {
        let (store, _dir) = temp_store();
        assert!(store.get_identity().is_none());

        let id = NodeIdentity {
            private_key: "priv_base64".into(),
            public_key: "pub_base64".into(),
        };
        store.set_identity(&id).unwrap();

        let loaded = store.get_identity().unwrap();
        assert_eq!(loaded.private_key, "priv_base64");
        assert_eq!(loaded.public_key, "pub_base64");
    }

    #[test]
    fn test_identity_private_key_encrypted_on_disk() {
        let (store, _dir) = temp_store();
        let id = NodeIdentity {
            private_key: "super-secret-ed25519-seed".into(),
            public_key: "pub_base64".into(),
        };
        store.set_identity(&id).unwrap();

        // Raw bytes on disk must NOT contain the plaintext secret and
        // must use the ENC: envelope prefix.
        let raw = store.raw_identity_private_key().unwrap();
        assert!(
            raw.starts_with("ENC:"),
            "stored private_key should be encrypted, got: {}",
            raw
        );
        assert!(!raw.contains("super-secret-ed25519-seed"));

        // Public key remains plaintext.
        assert_eq!(store.get("identity:public_key").unwrap(), "pub_base64");
    }

    #[test]
    fn test_identity_plaintext_migration_on_read() {
        // Simulate a legacy node that wrote the private key before encryption
        // was introduced. Reading through a crypto-enabled store should
        // transparently return the plaintext value.
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(SledPool::new(dir.path().to_path_buf()));

        // Phase 1: legacy write via a crypto-less store — but bypass
        // set_identity (which now requires a key) by using the raw kv API.
        let legacy = NodeConfigStore::new(Arc::clone(&pool)).unwrap();
        legacy
            .set("identity:private_key", "legacy-plaintext")
            .unwrap();
        legacy.set("identity:public_key", "legacy-pub").unwrap();

        // Phase 2: upgraded store with an encryption key reads the legacy
        // value successfully via the migration fallback.
        let upgraded =
            NodeConfigStore::with_crypto_key(Arc::clone(&pool), Some([0x42u8; 32])).unwrap();
        let loaded = upgraded.get_identity().unwrap();
        assert_eq!(loaded.private_key, "legacy-plaintext");
        assert_eq!(loaded.public_key, "legacy-pub");

        // Phase 3: the next write re-persists the value in encrypted form.
        upgraded
            .set_identity(&NodeIdentity {
                private_key: "new-secret".into(),
                public_key: "legacy-pub".into(),
            })
            .unwrap();
        let raw = upgraded.raw_identity_private_key().unwrap();
        assert!(raw.starts_with("ENC:"));
        assert!(!raw.contains("new-secret"));
    }

    #[test]
    fn test_set_identity_without_key_fails_loudly() {
        let (store, _dir) = temp_store_no_crypto();
        let id = NodeIdentity {
            private_key: "priv".into(),
            public_key: "pub".into(),
        };
        // Writing a sensitive field without a configured key must error —
        // no silent plaintext fallback.
        assert!(store.set_identity(&id).is_err());
    }

    #[test]
    fn test_get_identity_without_key_fails_on_encrypted_value() {
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(SledPool::new(dir.path().to_path_buf()));

        // Write an encrypted value with a keyed store.
        let keyed =
            NodeConfigStore::with_crypto_key(Arc::clone(&pool), Some([0x42u8; 32])).unwrap();
        keyed
            .set_identity(&NodeIdentity {
                private_key: "priv".into(),
                public_key: "pub".into(),
            })
            .unwrap();

        // A crypto-less handle must NOT silently hand out ciphertext.
        let unkeyed = NodeConfigStore::new(Arc::clone(&pool)).unwrap();
        assert!(unkeyed.get_identity().is_none());
    }

    #[test]
    fn test_identity_persists_across_handles_with_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(SledPool::new(dir.path().to_path_buf()));

        {
            let a =
                NodeConfigStore::with_crypto_key(Arc::clone(&pool), Some([0x11u8; 32])).unwrap();
            a.set_identity(&NodeIdentity {
                private_key: "persist-me".into(),
                public_key: "pub".into(),
            })
            .unwrap();
        }

        let b = NodeConfigStore::with_crypto_key(Arc::clone(&pool), Some([0x11u8; 32])).unwrap();
        let loaded = b.get_identity().unwrap();
        assert_eq!(loaded.private_key, "persist-me");
    }

    #[test]
    fn test_ai_config_round_trip() {
        let (store, _dir) = temp_store();
        assert!(store.get_ai_config().is_none());

        let config = AiConfig {
            provider: "Anthropic".into(),
            anthropic_key: Some("sk-ant-test".into()),
            anthropic_model: Some("claude-sonnet-4-20250514".into()),
            anthropic_base_url: None,
            ollama_model: None,
            ollama_url: None,
            ollama_vision_model: None,
        };
        store.set_ai_config(&config).unwrap();

        let loaded = store.get_ai_config().unwrap();
        assert_eq!(loaded.provider, "Anthropic");
        assert_eq!(loaded.anthropic_key.unwrap(), "sk-ant-test");
        assert!(loaded.ollama_model.is_none());
    }

    #[test]
    fn test_is_empty() {
        let (store, _dir) = temp_store();
        assert!(store.is_empty());
        store.set("foo", "bar").unwrap();
        assert!(!store.is_empty());
    }

    #[test]
    fn test_cloud_disabled_returns_none() {
        let (store, _dir) = temp_store();
        store.set("cloud:api_url", "https://example.com").unwrap();
        // cloud:enabled not set -> get_cloud_config returns None
        assert!(store.get_cloud_config().is_none());
    }
}
