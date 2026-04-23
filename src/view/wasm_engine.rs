use crate::schema::types::errors::SchemaError;
use serde_json::Value;

#[cfg(feature = "transform-wasm")]
use std::collections::HashMap;
#[cfg(feature = "transform-wasm")]
use std::sync::Mutex;
#[cfg(feature = "transform-wasm")]
use wasmtime::{Config, Engine, Linker, Module, Store, Trap};

/// Sandboxed WASM execution engine with compiled module caching.
pub struct WasmTransformEngine {
    #[cfg(feature = "transform-wasm")]
    engine: Engine,
    #[cfg(feature = "transform-wasm")]
    module_cache: Mutex<HashMap<u64, Module>>,
    /// When the transform-wasm feature is disabled, this is a no-op engine.
    #[cfg(not(feature = "transform-wasm"))]
    _phantom: (),
}

impl WasmTransformEngine {
    pub fn new() -> Result<Self, SchemaError> {
        #[cfg(feature = "transform-wasm")]
        {
            // Cross-device determinism stacks two engine-level configs:
            //
            //   * `cranelift_nan_canonicalization(true)` (MDT-C) — so
            //     every architecture produces the same IEEE 754 bit
            //     pattern for a quiet NaN. Without this, aarch64 and
            //     x86_64 Cranelift back-ends pick different sign bits
            //     for `0.0 / 0.0` and `sqrt(-1)` (x86 → FFF8…, ARM →
            //     7FF8…), the cross-device divergence called out as
            //     Invariant #1 in `docs/design/multi_device_transforms.md`
            //     (exemem-workspace PR #137). The `wasm-determinism` CI
            //     job enforces it.
            //
            //   * `consume_fuel(true)` (MDT-E) — the per-invocation
            //     `max_gas` budget is set on the Store before each
            //     `transform` call so identical inputs trap at
            //     identical fuel counts on every device. Without this
            //     the `Store::set_fuel` call inside `execute_wasm`
            //     would itself error.
            let mut config = Config::new();
            config.cranelift_nan_canonicalization(true);
            config.consume_fuel(true);
            let engine = Engine::new(&config)
                .map_err(|e| SchemaError::InvalidTransform(format!("wasmtime engine init: {e}")))?;
            Ok(Self {
                engine,
                module_cache: Mutex::new(HashMap::new()),
            })
        }
        #[cfg(not(feature = "transform-wasm"))]
        {
            Ok(Self { _phantom: () })
        }
    }

    /// Execute a WASM transform on the given input value with an explicit
    /// per-invocation fuel ceiling.
    ///
    /// `max_gas` is the wasmtime fuel budget consumed before the guest is
    /// trapped. It is set on the `Store` to exactly this value — callers
    /// do not get to relax or tighten it per-device — so a given
    /// `(transform, input)` pair either succeeds on every device or fails
    /// with [`SchemaError::TransformGasExceeded`] on every device (MDT-E).
    ///
    /// The WASM module must export:
    /// - `alloc(size: i32) -> i32` — allocate memory for input
    /// - `transform(ptr: i32, len: i32) -> i64` — execute transform, returns (ptr << 32 | len)
    /// - `memory` — linear memory export
    pub fn execute(
        &self,
        wasm_bytes: &[u8],
        input: &Value,
        max_gas: u64,
    ) -> Result<Value, SchemaError> {
        #[cfg(feature = "transform-wasm")]
        {
            self.execute_wasm(wasm_bytes, input, max_gas)
        }
        #[cfg(not(feature = "transform-wasm"))]
        {
            let _ = wasm_bytes;
            let _ = input;
            let _ = max_gas;
            Err(SchemaError::InvalidTransform(
                "WASM transforms require the 'transform-wasm' feature flag".to_string(),
            ))
        }
    }

    #[cfg(feature = "transform-wasm")]
    fn execute_wasm(
        &self,
        wasm_bytes: &[u8],
        input: &Value,
        max_gas: u64,
    ) -> Result<Value, SchemaError> {
        let module = self.get_or_compile(wasm_bytes)?;
        let linker = Linker::new(&self.engine);
        let mut store = Store::new(&self.engine, ());

        // Seed the per-invocation fuel budget. With `consume_fuel(true)`
        // on the engine, wasmtime traps with `Trap::OutOfFuel` when this
        // counter reaches zero — exactly the enforcement contract MDT-E
        // requires. `set_fuel` itself fails only if fuel metering was
        // not enabled on the engine, which is a programmer error.
        store.set_fuel(max_gas).map_err(|e| {
            SchemaError::InvalidTransform(format!(
                "WASM fuel metering misconfigured (set_fuel failed): {e}"
            ))
        })?;

        let instance = linker.instantiate(&mut store, &module).map_err(|e| {
            SchemaError::InvalidTransform(format!("WASM instantiation failed: {e}"))
        })?;

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            SchemaError::InvalidTransform("WASM module must export 'memory'".to_string())
        })?;

        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| {
                SchemaError::InvalidTransform(format!("WASM module must export 'alloc': {e}"))
            })?;

        let transform_fn = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "transform")
            .map_err(|e| {
                SchemaError::InvalidTransform(format!("WASM module must export 'transform': {e}"))
            })?;

        // Write input JSON to WASM memory. `input_bytes.len()` is the
        // deterministic "primary input dimension" fed to
        // `SchemaError::TransformGasExceeded` so the classifier at the
        // resolver boundary can surface a size-aware reason.
        let input_bytes = serde_json::to_vec(input).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize transform input: {e}"))
        })?;
        let input_size = input_bytes.len() as u64;

        let input_ptr = alloc_fn
            .call(&mut store, input_bytes.len() as i32)
            .map_err(|e| classify_call_error(e, input_size, "WASM alloc"))?;

        memory.data_mut(&mut store)[input_ptr as usize..input_ptr as usize + input_bytes.len()]
            .copy_from_slice(&input_bytes);

        // Call transform
        let result_packed = transform_fn
            .call(&mut store, (input_ptr, input_bytes.len() as i32))
            .map_err(|e| classify_call_error(e, input_size, "WASM transform execution"))?;

        // Unpack result pointer and length from i64
        let result_ptr = (result_packed >> 32) as usize;
        let result_len = (result_packed & 0xFFFF_FFFF) as usize;

        let result_bytes = &memory.data(&store)[result_ptr..result_ptr + result_len];
        let result: Value = serde_json::from_slice(result_bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize transform output: {e}"))
        })?;

        Ok(result)
    }

    #[cfg(feature = "transform-wasm")]
    fn get_or_compile(&self, wasm_bytes: &[u8]) -> Result<Module, SchemaError> {
        let hash = Self::hash_bytes(wasm_bytes);
        let mut cache = self.module_cache.lock().unwrap_or_else(|p| p.into_inner());

        if let Some(module) = cache.get(&hash) {
            return Ok(module.clone());
        }

        let module = Module::new(&self.engine, wasm_bytes).map_err(|e| {
            SchemaError::InvalidTransform(format!("Failed to compile WASM module: {e}"))
        })?;

        cache.insert(hash, module.clone());
        Ok(module)
    }

    #[cfg(feature = "transform-wasm")]
    fn hash_bytes(bytes: &[u8]) -> u64 {
        seahash::hash(bytes)
    }
}

/// Classify a `wasmtime::Error` returned from a guest `call`. Fuel
/// exhaustion surfaces as [`SchemaError::TransformGasExceeded`] so the
/// resolver can produce a `GasExceeded` reason without stringly-typed
/// sniffing; everything else stays as `InvalidTransform` for
/// classification as `ExecutionError` downstream.
#[cfg(feature = "transform-wasm")]
fn classify_call_error(err: wasmtime::Error, input_size: u64, context: &str) -> SchemaError {
    if let Some(trap) = err.downcast_ref::<Trap>() {
        if *trap == Trap::OutOfFuel {
            return SchemaError::TransformGasExceeded { input_size };
        }
    }
    SchemaError::InvalidTransform(format!("{context} failed: {err}"))
}

impl std::fmt::Debug for WasmTransformEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTransformEngine").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = WasmTransformEngine::new().unwrap();
        // Just verify it can be created
        let _ = format!("{:?}", engine);
    }

    #[test]
    #[cfg(not(feature = "transform-wasm"))]
    fn test_execute_without_feature_flag_errors() {
        let engine = WasmTransformEngine::new().unwrap();
        let result = engine.execute(&[0, 1, 2], &serde_json::json!(42), 1_000_000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("transform-wasm"),
            "Error should mention feature flag: {}",
            err
        );
    }
}
