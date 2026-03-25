use crate::schema::types::errors::SchemaError;
use serde_json::Value;

#[cfg(feature = "transform-wasm")]
use std::collections::HashMap;
#[cfg(feature = "transform-wasm")]
use std::sync::Mutex;
#[cfg(feature = "transform-wasm")]
use wasmtime::{Engine, Linker, Module, Store};

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
            let engine = Engine::default();
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

    /// Execute a WASM transform on the given input value.
    ///
    /// The WASM module must export:
    /// - `alloc(size: i32) -> i32` — allocate memory for input
    /// - `transform(ptr: i32, len: i32) -> i64` — execute transform, returns (ptr << 32 | len)
    /// - `memory` — linear memory export
    pub fn execute(&self, wasm_bytes: &[u8], input: &Value) -> Result<Value, SchemaError> {
        #[cfg(feature = "transform-wasm")]
        {
            self.execute_wasm(wasm_bytes, input)
        }
        #[cfg(not(feature = "transform-wasm"))]
        {
            let _ = wasm_bytes;
            let _ = input;
            Err(SchemaError::InvalidTransform(
                "WASM transforms require the 'transform-wasm' feature flag".to_string(),
            ))
        }
    }

    #[cfg(feature = "transform-wasm")]
    fn execute_wasm(&self, wasm_bytes: &[u8], input: &Value) -> Result<Value, SchemaError> {
        let module = self.get_or_compile(wasm_bytes)?;
        let linker = Linker::new(&self.engine);
        let mut store = Store::new(&self.engine, ());

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

        // Write input JSON to WASM memory
        let input_bytes = serde_json::to_vec(input).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize transform input: {e}"))
        })?;

        let input_ptr = alloc_fn
            .call(&mut store, input_bytes.len() as i32)
            .map_err(|e| SchemaError::InvalidTransform(format!("WASM alloc failed: {e}")))?;

        memory.data_mut(&mut store)[input_ptr as usize..input_ptr as usize + input_bytes.len()]
            .copy_from_slice(&input_bytes);

        // Call transform
        let result_packed = transform_fn
            .call(&mut store, (input_ptr, input_bytes.len() as i32))
            .map_err(|e| {
                SchemaError::InvalidTransform(format!("WASM transform execution failed: {e}"))
            })?;

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
        let result = engine.execute(&[0, 1, 2], &serde_json::json!(42));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("transform-wasm"),
            "Error should mention feature flag: {}",
            err
        );
    }
}
