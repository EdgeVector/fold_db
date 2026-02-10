//! Cryptographic primitives for encryption at rest.
//!
//! This module provides:
//! - **`CryptoProvider`** trait — abstract encrypt/decrypt interface
//! - **`LocalCryptoProvider`** — file-based AES-256-GCM for standalone dev nodes
//! - **`NoOpCryptoProvider`** — passthrough for tests and migration
//! - **`KmsCryptoProvider`** — AWS KMS envelope encryption for cloud deployments
//! - **`envelope`** — self-describing binary ciphertext format

pub mod envelope;
pub mod error;
#[cfg(feature = "aws-backend")]
pub mod kms_provider;
pub mod provider;

pub use error::{CryptoError, CryptoResult};
#[cfg(feature = "aws-backend")]
pub use kms_provider::KmsCryptoProvider;
pub use provider::{CryptoProvider, LocalCryptoProvider, NoOpCryptoProvider};
