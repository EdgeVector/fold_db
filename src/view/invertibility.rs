use crate::schema::types::errors::SchemaError;
use crate::view::wasm_engine::WasmTransformEngine;
use serde_json::Value;

/// Verify that forward and inverse WASM transforms form a round-trip.
///
/// Generates test inputs of various types and checks that `inverse(forward(x)) == x`
/// for all of them. Returns true if the pair is reversible, false otherwise.
/// Returns Err only on execution failures, not on failed round-trips.
pub fn verify_roundtrip(
    engine: &WasmTransformEngine,
    forward: &[u8],
    inverse: &[u8],
) -> Result<bool, SchemaError> {
    let test_inputs = generate_test_inputs();

    for input in &test_inputs {
        let forward_result = engine.execute(forward, input)?;
        let inverse_result = engine.execute(inverse, &forward_result)?;

        if !values_equal(input, &inverse_result) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Generate a set of representative test inputs for round-trip verification.
fn generate_test_inputs() -> Vec<Value> {
    vec![
        // Strings
        Value::String("hello".into()),
        Value::String("".into()),
        Value::String("unicode: \u{1F600} \u{00E9}".into()),
        // Integers
        serde_json::json!(0),
        serde_json::json!(1),
        serde_json::json!(-42),
        serde_json::json!(1_000_000),
        // Floats
        serde_json::json!(7.77),
        serde_json::json!(-0.001),
        serde_json::json!(1e10),
        // Booleans
        Value::Bool(true),
        Value::Bool(false),
        // Null
        Value::Null,
        // Small array
        serde_json::json!([1, "two", 3.0, true, null]),
        // Small object
        serde_json::json!({"key": "value", "num": 42, "nested": {"a": 1}}),
    ]
}

/// Compare two JSON values with float tolerance.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            match (na.as_f64(), nb.as_f64()) {
                (Some(fa), Some(fb)) => (fa - fb).abs() < 1e-10,
                _ => na == nb,
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            aa.len() == ab.len() && aa.iter().zip(ab.iter()).all(|(x, y)| values_equal(x, y))
        }
        (Value::Object(oa), Value::Object(ob)) => {
            oa.len() == ob.len()
                && oa
                    .iter()
                    .all(|(k, v)| ob.get(k).is_some_and(|bv| values_equal(v, bv)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_values_equal_basic() {
        assert!(values_equal(&serde_json::json!(1), &serde_json::json!(1)));
        assert!(values_equal(
            &serde_json::json!("hello"),
            &serde_json::json!("hello")
        ));
        assert!(!values_equal(
            &serde_json::json!(1),
            &serde_json::json!(2)
        ));
    }

    #[test]
    fn test_values_equal_float_tolerance() {
        // Difference of 1e-12, well within 1e-10 tolerance
        let a = serde_json::json!(1.0000000000001);
        let b = serde_json::json!(1.0000000000002);
        assert!(values_equal(&a, &b));

        let c = serde_json::json!(1.0);
        let d = serde_json::json!(2.0);
        assert!(!values_equal(&c, &d));
    }

    #[test]
    fn test_values_equal_nested() {
        let a = serde_json::json!({"arr": [1, 2.0000000000001], "str": "x"});
        let b = serde_json::json!({"arr": [1, 2.0000000000002], "str": "x"});
        assert!(values_equal(&a, &b));
    }

    #[test]
    fn test_generate_test_inputs_coverage() {
        let inputs = generate_test_inputs();
        assert!(inputs.len() >= 10, "Should have diverse test inputs");

        let has_string = inputs.iter().any(|v| v.is_string());
        let has_number = inputs.iter().any(|v| v.is_number());
        let has_bool = inputs.iter().any(|v| v.is_boolean());
        let has_null = inputs.iter().any(|v| v.is_null());
        let has_array = inputs.iter().any(|v| v.is_array());
        let has_object = inputs.iter().any(|v| v.is_object());

        assert!(has_string, "Missing string inputs");
        assert!(has_number, "Missing number inputs");
        assert!(has_bool, "Missing boolean inputs");
        assert!(has_null, "Missing null inputs");
        assert!(has_array, "Missing array inputs");
        assert!(has_object, "Missing object inputs");
    }
}
