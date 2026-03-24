//! NMI (Normalized Mutual Information) estimation for transform classification.
//!
//! Phase 2 of the classification pipeline: generates synthetic data, runs it
//! through the WASM transform, and estimates information leakage via NMI.

#![cfg(feature = "transform-wasm")]

use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::field_value_type::FieldValueType;
use crate::view::wasm_engine::WasmTransformEngine;

/// Generates synthetic data for NMI estimation.
pub struct SyntheticDataGenerator;

impl SyntheticDataGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a baseline input with default values for each field.
    pub fn generate_baseline(
        &self,
        input_schema: &HashMap<String, FieldValueType>,
    ) -> serde_json::Value {
        let mut baseline = serde_json::Map::new();
        for (field_name, field_type) in input_schema {
            let default_val = match field_type {
                FieldValueType::String => serde_json::json!(""),
                FieldValueType::Integer => serde_json::json!(0),
                FieldValueType::Float => serde_json::json!(0.0),
                FieldValueType::Boolean => serde_json::json!(false),
                _ => serde_json::json!(null),
            };
            baseline.insert(field_name.clone(), default_val);
        }
        serde_json::Value::Object(baseline)
    }

    /// Generate N varied samples for a given field type.
    pub fn generate_field_samples(
        &self,
        field_type: &FieldValueType,
        count: u32,
    ) -> Vec<serde_json::Value> {
        (0..count)
            .map(|i| match field_type {
                FieldValueType::String => serde_json::json!(format!("sample_{}", i)),
                FieldValueType::Integer => serde_json::json!(i as i64),
                FieldValueType::Float => serde_json::json!(i as f64 * 0.1),
                FieldValueType::Boolean => serde_json::json!(i % 2 == 0),
                _ => serde_json::json!(format!("sample_{}", i)),
            })
            .collect()
    }
}

/// Estimate NMI matrix between input and output fields by running the WASM
/// transform on synthetic data.
///
/// Returns a map: input_field -> { output_field -> nmi_score }.
pub fn estimate_nmi_matrix(
    wasm_bytes: &[u8],
    _input_schema: &HashMap<String, FieldValueType>,
    output_fields: &HashMap<String, FieldValueType>,
    baseline: &serde_json::Value,
    all_input_samples: &HashMap<String, Vec<serde_json::Value>>,
    sample_count: u32,
) -> FoldDbResult<HashMap<String, HashMap<String, f32>>> {
    let engine = WasmTransformEngine::new().map_err(|e| {
        FoldDbError::Config(format!("Failed to create WASM engine for NMI estimation: {}", e))
    })?;

    let mut nmi_matrix: HashMap<String, HashMap<String, f32>> = HashMap::new();

    // For each input field, vary it while holding others at baseline,
    // then measure NMI between the varied input and each output field.
    for (input_field, samples) in all_input_samples {
        let mut output_scores: HashMap<String, f32> = HashMap::new();

        // Collect output values for each sample
        let mut output_columns: HashMap<String, Vec<String>> = HashMap::new();
        let mut input_column: Vec<String> = Vec::new();

        for sample_val in samples.iter().take(sample_count as usize) {
            // Build input with this field varied, others at baseline
            let mut input = baseline
                .as_object()
                .cloned()
                .unwrap_or_default();
            input.insert(input_field.clone(), sample_val.clone());

            let input_json = serde_json::json!({ "inputs": { "data": input } });

            match engine.execute(wasm_bytes, &input_json) {
                Ok(output) => {
                    input_column.push(sample_val.to_string());
                    if let Some(fields) = output.get("fields").and_then(|f| f.as_object()) {
                        for output_field in output_fields.keys() {
                            let val = fields
                                .get(output_field)
                                .map(|v| v.to_string())
                                .unwrap_or_default();
                            output_columns
                                .entry(output_field.clone())
                                .or_default()
                                .push(val);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // Compute NMI for each output field
        for (output_field, output_vals) in &output_columns {
            let nmi = compute_nmi(&input_column, output_vals);
            output_scores.insert(output_field.clone(), nmi);
        }

        nmi_matrix.insert(input_field.clone(), output_scores);
    }

    Ok(nmi_matrix)
}

/// Compute Normalized Mutual Information between two discrete random variables.
/// Both are represented as vectors of string-encoded values.
fn compute_nmi(x: &[String], y: &[String]) -> f32 {
    if x.is_empty() || x.len() != y.len() {
        return 0.0;
    }

    let n = x.len() as f32;

    // Count joint and marginal frequencies
    let mut joint: HashMap<(&str, &str), f32> = HashMap::new();
    let mut x_counts: HashMap<&str, f32> = HashMap::new();
    let mut y_counts: HashMap<&str, f32> = HashMap::new();

    for (xi, yi) in x.iter().zip(y.iter()) {
        *joint.entry((xi.as_str(), yi.as_str())).or_default() += 1.0;
        *x_counts.entry(xi.as_str()).or_default() += 1.0;
        *y_counts.entry(yi.as_str()).or_default() += 1.0;
    }

    // H(X)
    let h_x: f32 = x_counts
        .values()
        .map(|&c| {
            let p = c / n;
            -p * p.ln()
        })
        .sum();

    // H(Y)
    let h_y: f32 = y_counts
        .values()
        .map(|&c| {
            let p = c / n;
            -p * p.ln()
        })
        .sum();

    if h_x == 0.0 || h_y == 0.0 {
        return 0.0;
    }

    // MI(X;Y)
    let mi: f32 = joint
        .iter()
        .map(|((xi, yi), &count)| {
            let p_xy = count / n;
            let p_x = x_counts[xi] / n;
            let p_y = y_counts[yi] / n;
            p_xy * (p_xy / (p_x * p_y)).ln()
        })
        .sum();

    // NMI = 2 * MI / (H(X) + H(Y))
    let nmi = 2.0 * mi / (h_x + h_y);
    nmi.clamp(0.0, 1.0)
}
