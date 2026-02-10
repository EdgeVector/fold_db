//! Cryptographic primitives for encryption at rest.
//!
//! This module provides:
//! - **`CryptoProvider`** trait — abstract encrypt/decrypt interface
//! - **`LocalCryptoProvider`** — file-based AES-256-GCM for standalone dev nodes
//! - **`NoOpCryptoProvider`** — passthrough for tests and migration
//! - **`envelope`** — self-describing binary ciphertext format
//!
//! Cloud providers (e.g., `KmsCryptoProvider` using AWS KMS) are implemented
//! in `exemem-infra` and depend on the AWS SDK feature flag.

pub mod envelope;
pub mod error;
pub mod provider;

pub use error::{CryptoError, CryptoResult};
pub use provider::{CryptoProvider, LocalCryptoProvider, NoOpCryptoProvider};
