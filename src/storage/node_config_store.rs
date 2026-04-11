use serde::{Deserialize, Serialize};

use super::sled_pool::SledPool;
use std::sync::Arc;

const TREE_NAME: &str = "node_config";

/// Thin wrapper around a SledPool for storing node configuration.
/// All runtime config (identity, cloud credentials, AI settings) lives here.
pub struct NodeConfigStore {
    pool: Arc<SledPool>,
}

impl NodeConfigStore {
    pub fn new(pool: Arc<SledPool>) -> Result<Self, sled::Error> {
        // Validate that we can open the tree by doing a test acquire
        let guard = pool.acquire_arc().map_err(|e| {
            sled::Error::Io(std::io::Error::other(format!(
                "Failed to acquire pool: {}",
                e
            )))
        })?;
        guard.db().open_tree(TREE_NAME)?;
        drop(guard);
        Ok(Self { pool })
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
        self.set("cloud:enabled", "true")?;
        Ok(())
    }

    pub fn is_cloud_enabled(&self) -> bool {
        self.get("cloud:enabled").as_deref() == Some("true")
    }

    // --- Node identity ---

    pub fn get_identity(&self) -> Option<NodeIdentity> {
        Some(NodeIdentity {
            private_key: self.get("identity:private_key")?,
            public_key: self.get("identity:public_key")?,
        })
    }

    pub fn set_identity(&self, id: &NodeIdentity) -> Result<(), sled::Error> {
        self.set("identity:private_key", &id.private_key)?;
        self.set("identity:public_key", &id.public_key)?;
        Ok(())
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
        let store = NodeConfigStore::new(pool).unwrap();
        // Return dir to keep tempdir alive
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
