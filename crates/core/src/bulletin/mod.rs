//! Encrypted bulletin board client.
//!
//! Two operations against the storage_service Lambda's bulletin endpoints:
//!
//! - **Send** (`presign_bulletin_send` → encrypt → PUT): seal a payload
//!   under the recipient's Ed25519 public key (sealed-box / X25519-DH +
//!   AES-GCM via `crypto::inbox`) and PUT to a presigned URL. The path on
//!   R2 becomes `bulletin/<recipient_pseudonym>/<message_id>.enc` and is
//!   subject to a 7-day lifecycle expiration. Senders must be on a paid
//!   plan; the server enforces the gate via BillingTable.
//!
//! - **Read** (`presign_bulletin_read` → GET → decrypt): fetch the blob
//!   and open with the local Ed25519 secret key. There is no server-side
//!   ownership check — knowing the pseudonym is the right to read it
//!   (same trust model as `messaging_service.GET /api/messaging/messages`).
//!
//! The recipient is expected to learn `(pseudonym, message_id)` out of
//! band, typically via the existing `messaging_service` connection-relay.
//! That piggyback wiring lives in a follow-up; this module exposes only
//! the R2 client surface so consumers (CLI, web UI) can call it directly.
//!
//! ## Why a separate module from `sync::`
//!
//! Sync uses *symmetric* AES-GCM with the user's E2E key — fine for
//! same-user-multi-device because both sides hold the same key. Bulletin
//! board needs *asymmetric* recipient-public-key encryption (only the
//! recipient can open). The two crypto stacks coexist; this module
//! reuses the existing `crypto::inbox::seal_box_base64` primitives so
//! we don't reinvent the curve math.

use crate::crypto::error::CryptoError;
use crate::crypto::inbox::{open_box_base64, seal_box_base64};
use crate::sync::auth::AuthClient;
use crate::sync::error::SyncError;
use crate::sync::s3::S3Client;
use std::sync::Arc;

/// Errors specific to bulletin operations.
#[derive(Debug, thiserror::Error)]
pub enum BulletinError {
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("network/auth error: {0}")]
    Sync(#[from] SyncError),
    #[error("bulletin message not found: pseudonym={pseudonym} id={message_id}")]
    NotFound {
        pseudonym: String,
        message_id: String,
    },
}

impl From<CryptoError> for BulletinError {
    fn from(e: CryptoError) -> Self {
        BulletinError::Crypto(e.to_string())
    }
}

pub type BulletinResult<T> = Result<T, BulletinError>;

/// Client for the encrypted bulletin board.
///
/// Stateless — clones cheaply (just bumps Arcs).
#[derive(Clone)]
pub struct BulletinClient {
    auth: Arc<AuthClient>,
    s3: S3Client,
}

impl BulletinClient {
    pub fn new(auth: Arc<AuthClient>, s3: S3Client) -> Self {
        Self { auth, s3 }
    }

    /// Encrypt `plaintext` under `recipient_ed25519_pubkey_base64` and PUT
    /// it to `bulletin/<recipient_pseudonym>/<message_id>.enc`.
    ///
    /// The caller's account must be on a paid plan — `presign_bulletin_send`
    /// returns 403 otherwise, surfaced as [`BulletinError::Sync`] with the
    /// server's error message.
    ///
    /// `message_id` must be safe as an S3 key path component (no slashes,
    /// no `..`); the server validates with `validate_s3_key_component`.
    /// Callers typically use a UUID.
    pub async fn send(
        &self,
        recipient_pseudonym: &str,
        message_id: &str,
        recipient_ed25519_pubkey_base64: &str,
        plaintext: &[u8],
    ) -> BulletinResult<()> {
        let sealed = seal_box_base64(recipient_ed25519_pubkey_base64, plaintext)?;
        let url = self
            .auth
            .presign_bulletin_send(recipient_pseudonym, message_id)
            .await?;
        self.s3.upload(&url, sealed).await?;
        Ok(())
    }

    /// Fetch the bulletin object at `bulletin/<recipient_pseudonym>/<message_id>.enc`
    /// and open it with `my_ed25519_secret_base64`.
    ///
    /// Returns [`BulletinError::NotFound`] when R2 returns 404 (object
    /// expired by the 7-day lifecycle, or never written).
    pub async fn read(
        &self,
        recipient_pseudonym: &str,
        message_id: &str,
        my_ed25519_secret_base64: &str,
    ) -> BulletinResult<Vec<u8>> {
        let url = self
            .auth
            .presign_bulletin_read(recipient_pseudonym, message_id)
            .await?;
        let bytes = self
            .s3
            .download(&url)
            .await?
            .ok_or_else(|| BulletinError::NotFound {
                pseudonym: recipient_pseudonym.to_string(),
                message_id: message_id.to_string(),
            })?;
        let plaintext = open_box_base64(my_ed25519_secret_base64, &bytes)?;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::inbox::seal_box_base64;
    use crate::security::keys::Ed25519KeyPair;

    #[test]
    fn seal_then_open_round_trip_via_inbox_primitives() {
        // The bulletin module is a thin wrapper over the inbox primitives —
        // verifying the round-trip here documents the expected shape and
        // catches any regression in the underlying curve/HKDF code that
        // would silently break bulletin send/read.
        let recipient = Ed25519KeyPair::generate().unwrap();
        let sealed = seal_box_base64(&recipient.public_key_base64(), b"hello bulletin").unwrap();
        let opened =
            crate::crypto::inbox::open_box_base64(&recipient.secret_key_base64(), &sealed).unwrap();
        assert_eq!(&opened[..], b"hello bulletin");
    }

    #[test]
    fn bulletin_error_from_crypto_preserves_message() {
        let cerr = CryptoError::InvalidFormat("bad pubkey".to_string());
        let berr: BulletinError = cerr.into();
        let msg = berr.to_string();
        assert!(msg.contains("bad pubkey"), "{msg}");
    }
}
