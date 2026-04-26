//! Initialization helpers for each runtime target (node / Lambda / Tauri / CLI).
//!
//! Stub for Phase 1 / T6. The plan's shape: each helper installs the FMT +
//! RELOAD + RING layers, returns a guard that holds non-blocking writer
//! handles, and is `OnceCell`-guarded so a second call returns
//! [`crate::ObsError::AlreadyInitialized`].
