//! WASM transform execution tests.
//! These tests verify the end-to-end WASM contract: alloc + transform + memory exports.
//! Only run when the `transform-wasm` feature is enabled.
#![cfg(feature = "transform-wasm")]

use fold_db::view::WasmTransformEngine;
use serde_json::json;

/// Build a WASM module from WAT source text.
fn wat_to_wasm(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("valid WAT")
}

/// A minimal WASM module that returns a hardcoded JSON response.
/// This verifies the alloc/transform/memory contract works end-to-end.
///
/// The module stores `{"fields":{"out":{"k1":"hello"}}}` in memory at offset 1024
/// and returns a pointer to it from transform().
fn hardcoded_output_module() -> Vec<u8> {
    // The JSON output bytes: {"fields":{"out":{"k1":"hello"}}}
    let output = r#"{"fields":{"out":{"k1":"hello"}}}"#;
    let output_bytes = output.as_bytes();
    let len = output_bytes.len();

    // Build data section hex string for the output
    let escaped = output_bytes
        .iter()
        .map(|b| format!("\\{:02x}", b))
        .collect::<String>();

    let wat = format!(
        r#"(module
            (memory (export "memory") 1)
            ;; Store the output JSON at offset 1024
            (data (i32.const 1024) "{escaped}")

            ;; alloc: simple bump allocator starting at offset 2048
            (global $bump (mut i32) (i32.const 2048))
            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )

            ;; transform: ignore input, return hardcoded output at offset 1024
            (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
                ;; Pack pointer (1024) and length ({len}) into i64: (ptr << 32) | len
                (i64.or
                    (i64.shl (i64.extend_i32_u (i32.const 1024)) (i64.const 32))
                    (i64.extend_i32_u (i32.const {len}))
                )
            )
        )"#,
    );

    wat_to_wasm(&wat)
}

/// A WASM module that echoes its input back as output.
/// It reads the input JSON bytes and returns them unchanged.
/// This verifies that input data is correctly passed through the alloc/memory protocol.
fn echo_module() -> Vec<u8> {
    let wat = r#"(module
        (memory (export "memory") 1)

        ;; alloc: bump allocator starting at offset 4096
        (global $bump (mut i32) (i32.const 4096))
        (func (export "alloc") (param $size i32) (result i32)
            (local $ptr i32)
            (local.set $ptr (global.get $bump))
            (global.set $bump (i32.add (global.get $bump) (local.get $size)))
            (local.get $ptr)
        )

        ;; transform: return the input pointer and length unchanged
        (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
            (i64.or
                (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
                (i64.extend_i32_u (local.get $len))
            )
        )
    )"#;

    wat_to_wasm(wat)
}

/// A WASM module that copies input to a new location and returns it.
/// Verifies memory operations work correctly.
fn copy_module() -> Vec<u8> {
    let wat = r#"(module
        (memory (export "memory") 2)

        ;; alloc: bump allocator
        (global $bump (mut i32) (i32.const 4096))
        (func (export "alloc") (param $size i32) (result i32)
            (local $ptr i32)
            (local.set $ptr (global.get $bump))
            (global.set $bump (i32.add (global.get $bump) (local.get $size)))
            (local.get $ptr)
        )

        ;; transform: copy input to offset 32768 and return from there
        (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
            ;; memory.copy dest=32768, src=$ptr, len=$len
            (memory.copy
                (i32.const 32768)
                (local.get $ptr)
                (local.get $len)
            )
            ;; Return packed (32768 << 32) | len
            (i64.or
                (i64.shl (i64.extend_i32_u (i32.const 32768)) (i64.const 32))
                (i64.extend_i32_u (local.get $len))
            )
        )
    )"#;

    wat_to_wasm(wat)
}

#[test]
fn wasm_engine_executes_hardcoded_output() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = hardcoded_output_module();
    let input = json!({"anything": "ignored"});

    let result = engine.execute(&wasm, &input, 1_000_000_000).unwrap();

    assert_eq!(result, json!({"fields": {"out": {"k1": "hello"}}}));
}

#[test]
fn wasm_engine_echo_returns_input() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = echo_module();

    let input = json!({"inputs": {"BlogPost": {"title": {"r1": "Hello"}}}});
    let result = engine.execute(&wasm, &input, 1_000_000_000).unwrap();

    // Echo module returns input unchanged
    assert_eq!(result, input);
}

#[test]
fn wasm_engine_copy_returns_input() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = copy_module();

    let input = json!({"fields": {"word_count": {"r1": 42}}});
    let result = engine.execute(&wasm, &input, 1_000_000_000).unwrap();

    assert_eq!(result, input);
}

#[test]
fn wasm_engine_caches_compiled_modules() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = hardcoded_output_module();

    // Execute twice with same bytes — second should use cached module
    let r1 = engine.execute(&wasm, &json!({}), 1_000_000_000).unwrap();
    let r2 = engine.execute(&wasm, &json!({}), 1_000_000_000).unwrap();

    assert_eq!(r1, r2);
}

#[test]
fn wasm_engine_rejects_invalid_wasm() {
    let engine = WasmTransformEngine::new().unwrap();
    let invalid = vec![0, 1, 2, 3]; // Not valid WASM

    let result = engine.execute(&invalid, &json!({}), 1_000_000_000);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to compile"));
}

#[test]
fn wasm_engine_rejects_module_missing_exports() {
    // Valid WASM module but missing required exports
    let wat = r#"(module (memory (export "memory") 1))"#;
    let wasm = wat_to_wasm(wat);

    let engine = WasmTransformEngine::new().unwrap();
    let result = engine.execute(&wasm, &json!({}), 1_000_000_000);
    assert!(result.is_err());
    // Should mention missing alloc or transform
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("alloc") || err.contains("transform"),
        "Error should mention missing export: {}",
        err
    );
}

#[test]
fn wasm_engine_handles_large_input() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = echo_module();

    // Build a large input (~100KB of JSON)
    let mut fields = serde_json::Map::new();
    for i in 0..1000 {
        fields.insert(
            format!("field_{}", i),
            json!({"key": format!("value_{}", i)}),
        );
    }
    let input = json!({"inputs": {"LargeSchema": fields}});

    let result = engine.execute(&wasm, &input, 1_000_000_000).unwrap();
    assert_eq!(result, input);
}

// ==================== MDT-E: max_gas enforcement ==================== //

/// A WASM module whose `transform` enters an unconditional infinite loop.
/// Designed to exhaust any finite fuel budget deterministically — the
/// inner `br 0` consumes a fixed number of fuel units per iteration, so
/// two devices seeded with the same `max_gas` trap at the same iteration
/// count and the same remaining fuel (which for fuel exhaustion is zero).
fn infinite_loop_module() -> Vec<u8> {
    let wat = r#"(module
        (memory (export "memory") 1)

        (global $bump (mut i32) (i32.const 4096))
        (func (export "alloc") (param $size i32) (result i32)
            (local $ptr i32)
            (local.set $ptr (global.get $bump))
            (global.set $bump (i32.add (global.get $bump) (local.get $size)))
            (local.get $ptr)
        )

        ;; Burn fuel forever. The guest should be trapped with
        ;; Trap::OutOfFuel, which the engine classifies as
        ;; SchemaError::TransformGasExceeded.
        (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
            (loop $spin
                (br $spin)
            )
            (i64.const 0)
        )
    )"#;
    wat_to_wasm(wat)
}

/// An MDT-E baseline: the hardcoded-output transform runs to completion
/// under a generous budget. Paired with the infinite-loop test below,
/// this establishes that the fuel path distinguishes "ran" vs. "trapped"
/// rather than uniformly rejecting every invocation.
#[test]
fn max_gas_sufficient_budget_succeeds() {
    let engine = WasmTransformEngine::new().unwrap();
    let wasm = hardcoded_output_module();

    let result = engine
        .execute(&wasm, &json!({}), 1_000_000_000)
        .expect("hardcoded output must finish under a generous budget");
    assert_eq!(result, json!({"fields": {"out": {"k1": "hello"}}}));
}

/// The MDT-E contract: when a transform exceeds its fuel budget, the
/// engine surfaces `SchemaError::TransformGasExceeded` with the
/// deterministic `input_size` (serialized JSON byte length). The
/// resolver maps this to `UnavailableReason::GasExceeded`; that
/// end-to-end path is covered separately in `view_unavailable_test.rs`.
#[test]
fn max_gas_exhausted_returns_transform_gas_exceeded() {
    use fold_db::schema::types::errors::SchemaError;

    let engine = WasmTransformEngine::new().unwrap();
    let wasm = infinite_loop_module();
    let input = json!({"anything": "ignored"});
    // Input JSON: {"anything":"ignored"} — 22 bytes — which is what
    // the engine records as `input_size` on the error.
    let expected_input_size = serde_json::to_vec(&input).expect("serialize input").len() as u64;

    let err = engine
        .execute(&wasm, &input, 10_000)
        .expect_err("infinite loop must exhaust a 10_000-unit budget");
    match err {
        SchemaError::TransformGasExceeded { input_size } => {
            assert_eq!(
                input_size, expected_input_size,
                "input_size must reflect serialized input bytes"
            );
        }
        other => panic!("expected TransformGasExceeded, got {other:?}"),
    }
}

/// Deterministic termination is the whole point of MDT-E: two
/// independent `WasmTransformEngine` instances seeded with the same
/// `max_gas` and the same bytes + input must trap identically. There's
/// no device state in this test — just two engines in the same
/// process, which is the weakest form of the invariant we can assert
/// without a two-node harness. (The full cross-node assertion lives in
/// the multi-device round-1 findings doc; the unit-test contract is
/// that the fuel counter alone, without any per-device knobs, pins
/// outcome.)
#[test]
fn max_gas_deterministic_across_engine_instances() {
    use fold_db::schema::types::errors::SchemaError;

    let wasm = infinite_loop_module();
    let input = json!({"anything": "ignored"});
    let budget = 42_000u64;

    let engine_a = WasmTransformEngine::new().unwrap();
    let engine_b = WasmTransformEngine::new().unwrap();

    let err_a = engine_a
        .execute(&wasm, &input, budget)
        .expect_err("engine A must trap on fuel exhaustion");
    let err_b = engine_b
        .execute(&wasm, &input, budget)
        .expect_err("engine B must trap on fuel exhaustion");

    match (err_a, err_b) {
        (
            SchemaError::TransformGasExceeded {
                input_size: input_size_a,
            },
            SchemaError::TransformGasExceeded {
                input_size: input_size_b,
            },
        ) => {
            assert_eq!(
                input_size_a, input_size_b,
                "input_size must match across engine instances — it is a pure \
                 function of the serialized input, not of device state"
            );
        }
        other => panic!("both engines must trap with TransformGasExceeded, got {other:?}"),
    }
}
