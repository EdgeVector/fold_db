//! Cryptographic primitives for E2E encryption.
//!
//! This module provides:
//! - **`CryptoProvider`** trait — abstract encrypt/decrypt interface
//! - **`LocalCryptoProvider`** — file-based AES-256-GCM for standalone dev nodes
//! - **`NoOpCryptoProvider`** — passthrough for tests and migration
//! - **`envelope`** — self-describing binary ciphertext format

pub mod e2e;
pub mod envelope;
pub mod error;
pub mod provider;

pub use e2e::E2eKeys;
pub use error::{CryptoError, CryptoResult};
pub use provider::{CryptoProvider, LocalCryptoProvider, NoOpCryptoProvider};
