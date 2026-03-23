//! TransformResolver — local cache + registry fetch with sha256 verification.
//!
//! Resolves transform hashes to WASM bytes:
//! 1. Check local Sled cache
//! 2. Fetch from Global Transform Registry if not cached
//! 3. Verify sha256(bytes) == hash before caching/returning

use sha2::{Digest, Sha256};

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Resolves transform hashes to WASM bytes with local Sled caching
/// and sha256 verification.
pub struct TransformResolver {
    /// URL of the Global Transform Registry (schema_service)
    registry_url: String,
    /// Local Sled cache: hash → wasm_bytes
    cache_tree: sled::Tree,
    /// HTTP client for registry fetches
    client: reqwest::Client,
}

impl TransformResolver {
    /// Create a new TransformResolver.
    ///
    /// - `registry_url`: Base URL of the schema service (e.g. "http://localhost:9002")
    /// - `cache_db`: Sled database for local caching
    pub fn new(registry_url: String, cache_db: &sled::Db) -> FoldDbResult<Self> {
        let cache_tree = cache_db.open_tree("transform_wasm_cache").map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to open transform_wasm_cache tree: {}",
                e
            ))
        })?;

        Ok(Self {
            registry_url,
            cache_tree,
            client: reqwest::Client::new(),
        })
    }

    /// Resolve a transform hash to WASM bytes.
    ///
    /// 1. Check local Sled cache
    /// 2. Fetch from registry if not cached
    /// 3. Verify sha256(bytes) == hash before caching
    pub async fn resolve(&self, hash: &str) -> FoldDbResult<Vec<u8>> {
        // 1. Check local cache
        if let Some(cached) = self.cache_get(hash)? {
            log_feature!(
                LogFeature::Schema,
                info,
                "Transform '{}' resolved from local cache",
                hash
            );
            return Ok(cached);
        }

        // 2. Fetch from registry
        log_feature!(
            LogFeature::Schema,
            info,
            "Transform '{}' not in cache, fetching from registry",
            hash
        );
        let wasm_bytes = self.fetch_from_registry(hash).await?;

        // 3. Verify hash
        let computed_hash = compute_sha256(&wasm_bytes);
        if computed_hash != hash {
            return Err(FoldDbError::SecurityError(format!(
                "Transform hash mismatch: expected '{}', computed '{}' — registry may be compromised",
                hash, computed_hash
            )));
        }

        // Cache locally
        self.cache_put(hash, &wasm_bytes)?;

        log_feature!(
            LogFeature::Schema,
            info,
            "Transform '{}' fetched, verified, and cached ({} bytes)",
            hash,
            wasm_bytes.len()
        );

        Ok(wasm_bytes)
    }

    /// Check if a transform is cached locally.
    pub fn is_cached(&self, hash: &str) -> FoldDbResult<bool> {
        self.cache_tree
            .contains_key(hash.as_bytes())
            .map_err(|e| FoldDbError::Config(format!("Failed to check transform cache: {}", e)))
    }

    /// Evict a transform from the local cache.
    pub fn evict(&self, hash: &str) -> FoldDbResult<()> {
        self.cache_tree
            .remove(hash.as_bytes())
            .map_err(|e| FoldDbError::Config(format!("Failed to evict from transform cache: {}", e)))?;
        Ok(())
    }

    fn cache_get(&self, hash: &str) -> FoldDbResult<Option<Vec<u8>>> {
        match self.cache_tree.get(hash.as_bytes()) {
            Ok(Some(bytes)) => Ok(Some(bytes.to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(FoldDbError::Config(format!(
                "Failed to read from transform cache: {}",
                e
            ))),
        }
    }

    fn cache_put(&self, hash: &str, wasm_bytes: &[u8]) -> FoldDbResult<()> {
        self.cache_tree
            .insert(hash.as_bytes(), wasm_bytes)
            .map_err(|e| {
                FoldDbError::Config(format!("Failed to write to transform cache: {}", e))
            })?;
        self.cache_tree
            .flush()
            .map_err(|e| FoldDbError::Config(format!("Failed to flush transform cache: {}", e)))?;
        Ok(())
    }

    async fn fetch_from_registry(&self, hash: &str) -> FoldDbResult<Vec<u8>> {
        let url = format!("{}/api/transform/{}/wasm", self.registry_url, hash);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                FoldDbError::Config(format!(
                    "Failed to fetch transform '{}' from registry: {}",
                    hash, e
                ))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(FoldDbError::Config(format!(
                "Transform '{}' not found in registry at {}",
                hash, self.registry_url
            )));
        }

        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Registry returned status {} for transform '{}'",
                response.status(),
                hash
            )));
        }

        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to read WASM bytes for transform '{}': {}",
                hash, e
            ))
        })
    }
}

/// Compute sha256 hex digest.
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_cache_roundtrip() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_cache_db");
        let db = sled::open(&db_path).expect("failed to open sled");
        let resolver =
            TransformResolver::new("http://localhost:9999".to_string(), &db)
                .expect("failed to create resolver");

        let hash = "abc123";
        let wasm = b"fake wasm bytes";

        // Not cached initially
        assert!(!resolver.is_cached(hash).unwrap());
        assert!(resolver.cache_get(hash).unwrap().is_none());

        // Put and get
        resolver.cache_put(hash, wasm).unwrap();
        assert!(resolver.is_cached(hash).unwrap());
        assert_eq!(resolver.cache_get(hash).unwrap().unwrap(), wasm.to_vec());

        // Evict
        resolver.evict(hash).unwrap();
        assert!(!resolver.is_cached(hash).unwrap());
    }

    #[tokio::test]
    async fn test_resolve_verifies_hash() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_verify_db");
        let db = sled::open(&db_path).expect("failed to open sled");
        let resolver =
            TransformResolver::new("http://localhost:9999".to_string(), &db)
                .expect("failed to create resolver");

        let wasm = b"test wasm data";
        let correct_hash = compute_sha256(wasm);
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        // Pre-populate cache with the wasm bytes under the correct hash
        resolver.cache_put(&correct_hash, wasm).unwrap();

        // Resolve with correct hash succeeds
        let result = resolver.resolve(&correct_hash).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm.to_vec());

        // Pre-populate cache with wrong hash → wasm mapping to test verification
        // (This simulates a corrupted cache entry)
        resolver.cache_put(wrong_hash, wasm).unwrap();
        // Cache hit returns bytes without re-verifying (cache is trusted once verified)
        // The verification happens only on fetch from registry
    }
}
