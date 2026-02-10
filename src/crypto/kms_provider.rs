use super::envelope::{decrypt_envelope, encrypt_envelope};
use super::error::{CryptoError, CryptoResult};
use super::provider::CryptoProvider;
use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client as KmsClient;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AWS KMS-backed crypto provider using envelope encryption.
///
/// Each tenant (`user_hash`) gets a unique AES-256 Data Encryption Key (DEK).
/// The DEK is encrypted by the KMS Customer Managed Key (KEK) and stored
/// in the `ExememDekStore` DynamoDB table. The plaintext DEK is cached
/// in memory for the lifetime of the Lambda invocation / request.
///
/// Key hierarchy:
/// ```text
/// KMS CMK (KEK) → encrypts → per-tenant DEK → encrypts → atom content
/// ```
pub struct KmsCryptoProvider {
    kms_client: KmsClient,
    dynamo_client: DynamoClient,
    kms_key_id: String,
    dek_table_name: String,
    /// Cached plaintext DEK for the current tenant
    cached_dek: RwLock<Option<CachedDek>>,
    /// The user_hash (tenant) this provider serves
    user_hash: String,
}

struct CachedDek {
    plaintext_key: [u8; 32],
}

impl KmsCryptoProvider {
    /// Create a new KMS crypto provider for a specific tenant.
    ///
    /// The DEK is loaded lazily on first encrypt/decrypt call.
    pub fn new(
        kms_client: KmsClient,
        dynamo_client: DynamoClient,
        kms_key_id: String,
        dek_table_name: String,
        user_hash: String,
    ) -> Self {
        Self {
            kms_client,
            dynamo_client,
            kms_key_id,
            dek_table_name,
            cached_dek: RwLock::new(None),
            user_hash,
        }
    }

    /// Create from environment variables and shared AWS clients.
    pub fn from_env(
        kms_client: KmsClient,
        dynamo_client: DynamoClient,
        user_hash: String,
    ) -> CryptoResult<Self> {
        let kms_key_id = std::env::var("KMS_KEY_ID")
            .map_err(|_| CryptoError::KeyError("KMS_KEY_ID env var not set".into()))?;
        let dek_table_name = std::env::var("DEK_TABLE_NAME")
            .map_err(|_| CryptoError::KeyError("DEK_TABLE_NAME env var not set".into()))?;

        Ok(Self::new(
            kms_client,
            dynamo_client,
            kms_key_id,
            dek_table_name,
            user_hash,
        ))
    }

    /// Get or create the plaintext DEK for this tenant.
    ///
    /// 1. Check in-memory cache
    /// 2. Load encrypted DEK from DynamoDB, decrypt via KMS
    /// 3. If no DEK exists, generate one via KMS `GenerateDataKey`
    async fn get_dek(&self) -> CryptoResult<[u8; 32]> {
        // Fast path: check cache
        {
            let cache = self.cached_dek.read().await;
            if let Some(ref cached) = *cache {
                return Ok(cached.plaintext_key);
            }
        }

        // Slow path: load or generate DEK
        let dek = self.load_or_generate_dek().await?;

        // Cache it
        {
            let mut cache = self.cached_dek.write().await;
            *cache = Some(CachedDek { plaintext_key: dek });
        }

        Ok(dek)
    }

    /// Load existing DEK from DynamoDB or generate a new one.
    async fn load_or_generate_dek(&self) -> CryptoResult<[u8; 32]> {
        // Try loading from DynamoDB
        let result = self
            .dynamo_client
            .get_item()
            .table_name(&self.dek_table_name)
            .key("user_hash", AttributeValue::S(self.user_hash.clone()))
            .send()
            .await
            .map_err(|e| CryptoError::KeyError(format!("Failed to load DEK: {}", e)))?;

        if let Some(item) = result.item() {
            // DEK exists — decrypt it
            if let Some(encrypted_dek_attr) = item.get("encrypted_dek") {
                let encrypted_dek = encrypted_dek_attr
                    .as_b()
                    .map_err(|_| CryptoError::KeyError("encrypted_dek is not binary".into()))?
                    .as_ref()
                    .to_vec();

                return self.decrypt_dek(&encrypted_dek).await;
            }
        }

        // No DEK exists — generate one
        self.generate_and_store_dek().await
    }

    /// Generate a new DEK via KMS `GenerateDataKey` and store the encrypted copy.
    async fn generate_and_store_dek(&self) -> CryptoResult<[u8; 32]> {
        let response = self
            .kms_client
            .generate_data_key()
            .key_id(&self.kms_key_id)
            .key_spec(aws_sdk_kms::types::DataKeySpec::Aes256)
            .send()
            .await
            .map_err(|e| CryptoError::KeyError(format!("KMS GenerateDataKey failed: {}", e)))?;

        let plaintext_blob = response
            .plaintext()
            .ok_or_else(|| CryptoError::KeyError("KMS returned no plaintext key".into()))?;

        let ciphertext_blob = response
            .ciphertext_blob()
            .ok_or_else(|| CryptoError::KeyError("KMS returned no ciphertext blob".into()))?;

        let plaintext_bytes = plaintext_blob.as_ref();
        if plaintext_bytes.len() != 32 {
            return Err(CryptoError::KeyError(format!(
                "KMS key length {} != 32",
                plaintext_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(plaintext_bytes);

        // Store encrypted DEK in DynamoDB
        let now = chrono::Utc::now().to_rfc3339();
        self.dynamo_client
            .put_item()
            .table_name(&self.dek_table_name)
            .item("user_hash", AttributeValue::S(self.user_hash.clone()))
            .item(
                "encrypted_dek",
                AttributeValue::B(Blob::new(ciphertext_blob.as_ref().to_vec())),
            )
            .item("key_version", AttributeValue::N("1".into()))
            .item("created_at", AttributeValue::S(now))
            .item("kms_key_id", AttributeValue::S(self.kms_key_id.clone()))
            .send()
            .await
            .map_err(|e| CryptoError::KeyError(format!("Failed to store DEK: {}", e)))?;

        log::info!(
            "Generated and stored new DEK for tenant: {}",
            self.user_hash
        );

        Ok(key)
    }

    /// Decrypt an encrypted DEK using KMS.
    async fn decrypt_dek(&self, encrypted_dek: &[u8]) -> CryptoResult<[u8; 32]> {
        let response = self
            .kms_client
            .decrypt()
            .key_id(&self.kms_key_id)
            .ciphertext_blob(Blob::new(encrypted_dek.to_vec()))
            .send()
            .await
            .map_err(|e| CryptoError::KeyError(format!("KMS Decrypt failed: {}", e)))?;

        let plaintext_blob = response
            .plaintext()
            .ok_or_else(|| CryptoError::KeyError("KMS returned no plaintext".into()))?;

        let plaintext_bytes = plaintext_blob.as_ref();
        if plaintext_bytes.len() != 32 {
            return Err(CryptoError::KeyError(format!(
                "Decrypted key length {} != 32",
                plaintext_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(plaintext_bytes);
        Ok(key)
    }
}

#[async_trait]
impl CryptoProvider for KmsCryptoProvider {
    async fn encrypt(&self, plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
        let key = self.get_dek().await?;
        encrypt_envelope(&key, plaintext)
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        let key = self.get_dek().await?;
        decrypt_envelope(&key, ciphertext)
    }
}
