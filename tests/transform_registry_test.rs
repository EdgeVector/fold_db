//! Integration tests for the Global Transform Registry.

use std::collections::HashMap;

use fold_db::schema::types::data_classification::DataClassification;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::operations::Query;
use fold_db::schema::types::Schema;
use fold_db::schema_service::state::SchemaServiceState;
use fold_db::schema_service::types::{
    AddViewRequest, RegisterTransformRequest, TransformAddOutcome,
};
use tempfile::tempdir;

/// Create a test state with a temp directory.
fn make_test_state() -> SchemaServiceState {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir
        .path()
        .join("test_transform_db")
        .to_string_lossy()
        .to_string();
    // Leak the tempdir so it isn't deleted while state is in use
    std::mem::forget(temp_dir);
    SchemaServiceState::new(db_path).expect("failed to create state")
}

/// Create a test schema and add it to the state.
async fn add_test_schema(
    state: &SchemaServiceState,
    name: &str,
    fields: &[(&str, FieldValueType)],
    classifications: &[(&str, &str)],
) -> String {
    let field_names: Vec<String> = fields.iter().map(|(f, _)| f.to_string()).collect();
    let mut schema = Schema::new(
        name.to_string(),
        fold_db::schema::types::schema::DeclarativeSchemaType::Single,
        None,
        Some(field_names.clone()),
        None,
        None,
    );
    schema.descriptive_name = Some(name.to_string());
    for (f, _) in fields {
        schema
            .field_descriptions
            .insert(f.to_string(), format!("{} field", f));
    }
    for (f, t) in fields {
        schema.field_types.insert(f.to_string(), t.clone());
    }
    for &(f, c) in classifications {
        schema
            .field_classifications
            .insert(f.to_string(), vec![c.to_string()]);
        // Provide struct-based DataClassification so LLM is not required
        let dc = match c.to_lowercase().as_str() {
            "high" | "restricted" | "pii" | "medical" | "financial" | "hipaa" => {
                DataClassification::high()
            }
            "medium" | "internal" | "confidential" => DataClassification::medium(),
            _ => DataClassification::low(),
        };
        schema.field_data_classifications.insert(f.to_string(), dc);
    }
    // Fill in default classifications for fields without explicit ones
    for (f, _) in fields {
        if !schema.field_classifications.contains_key(*f) {
            schema
                .field_classifications
                .insert(f.to_string(), vec!["word".to_string()]);
        }
        if !schema.field_data_classifications.contains_key(*f) {
            schema
                .field_data_classifications
                .insert(f.to_string(), DataClassification::low());
        }
    }
    let outcome = state
        .add_schema(schema, HashMap::new())
        .await
        .expect("failed to add test schema");
    match outcome {
        fold_db::schema_service::types::SchemaAddOutcome::Added(s, _)
        | fold_db::schema_service::types::SchemaAddOutcome::Expanded(_, s, _) => s.name,
        fold_db::schema_service::types::SchemaAddOutcome::AlreadyExists(s, _) => s.name,
    }
}

fn make_register_request(
    name: &str,
    schema_name: &str,
    input_fields: &[&str],
    output_fields: &[(&str, FieldValueType)],
    wasm_bytes: &[u8],
) -> RegisterTransformRequest {
    RegisterTransformRequest {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: Some(format!("{} transform", name)),
        input_queries: vec![Query::new(
            schema_name.to_string(),
            input_fields.iter().map(|f| f.to_string()).collect(),
        )],
        output_fields: output_fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        source_url: None,
        rust_source: None,
        wasm_bytes: wasm_bytes.to_vec(),
    }
}

// ============== Registration & Deduplication ==============

#[tokio::test]
async fn test_register_transform_creates_record() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Medical Records",
        &[
            ("name", FieldValueType::String),
            ("diagnosis", FieldValueType::String),
        ],
        &[("name", "word"), ("diagnosis", "medical")],
    )
    .await;

    let wasm = b"fake_wasm_module_bytes_for_testing";
    let request = make_register_request(
        "downgrade_medical",
        &schema_name,
        &["name", "diagnosis"],
        &[("summary", FieldValueType::String)],
        wasm,
    );

    let (record, outcome) = state
        .register_transform(request)
        .await
        .expect("failed to register transform");

    assert!(matches!(outcome, TransformAddOutcome::Added));
    assert_eq!(record.name, "downgrade_medical");
    assert_eq!(record.version, "1.0.0");
    assert!(!record.hash.is_empty());
    assert!(record.registered_at > 0);
    assert!(record.output_schema.contains_key("summary"));
}

#[tokio::test]
async fn test_register_transform_deduplicates_by_hash() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Users",
        &[("user_id", FieldValueType::String)],
        &[("user_id", "word")],
    )
    .await;

    let wasm = b"same_wasm_bytes";
    let request1 = make_register_request(
        "transform_a",
        &schema_name,
        &["user_id"],
        &[("out", FieldValueType::String)],
        wasm,
    );
    let request2 = make_register_request(
        "transform_b", // different name, same WASM
        &schema_name,
        &["user_id"],
        &[("out", FieldValueType::String)],
        wasm,
    );

    let (record1, outcome1) = state
        .register_transform(request1)
        .await
        .expect("failed to register first");
    let (record2, outcome2) = state
        .register_transform(request2)
        .await
        .expect("failed to register second");

    assert!(matches!(outcome1, TransformAddOutcome::Added));
    assert!(matches!(outcome2, TransformAddOutcome::AlreadyExists));
    assert_eq!(record1.hash, record2.hash);
}

#[tokio::test]
async fn test_different_wasm_produces_different_hash() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Items",
        &[("item_id", FieldValueType::String)],
        &[("item_id", "word")],
    )
    .await;

    let (record1, _) = state
        .register_transform(make_register_request(
            "transform_v1",
            &schema_name,
            &["item_id"],
            &[("out", FieldValueType::String)],
            b"wasm_v1",
        ))
        .await
        .expect("failed to register v1");

    let (record2, _) = state
        .register_transform(make_register_request(
            "transform_v2",
            &schema_name,
            &["item_id"],
            &[("out", FieldValueType::String)],
            b"wasm_v2",
        ))
        .await
        .expect("failed to register v2");

    assert_ne!(record1.hash, record2.hash);
}

// ============== Hash Verification ==============

#[test]
fn test_verify_matching_hash() {
    let wasm = b"test wasm payload";
    let hash = SchemaServiceState::compute_wasm_hash(wasm);
    let (matches, computed) = SchemaServiceState::verify_transform(&hash, wasm);
    assert!(matches);
    assert_eq!(computed, hash);
}

#[test]
fn test_verify_mismatched_hash() {
    let wasm = b"test wasm payload";
    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let (matches, computed) = SchemaServiceState::verify_transform(wrong_hash, wasm);
    assert!(!matches);
    assert_ne!(computed, wrong_hash);
}

#[test]
fn test_hash_is_deterministic() {
    let wasm = b"deterministic payload";
    let hash1 = SchemaServiceState::compute_wasm_hash(wasm);
    let hash2 = SchemaServiceState::compute_wasm_hash(wasm);
    assert_eq!(hash1, hash2);
}

// ============== WASM Storage ==============

#[tokio::test]
async fn test_get_transform_wasm_after_registration() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Products",
        &[("sku", FieldValueType::String)],
        &[("sku", "word")],
    )
    .await;

    let wasm = b"real_wasm_module_data_here";
    let request = make_register_request(
        "sku_transform",
        &schema_name,
        &["sku"],
        &[("product_code", FieldValueType::String)],
        wasm,
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    let retrieved = state
        .get_transform_wasm(&record.hash)
        .await
        .expect("failed to get WASM");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), wasm.to_vec());
}

#[tokio::test]
async fn test_get_transform_wasm_nonexistent() {
    let state = make_test_state();
    let result = state
        .get_transform_wasm("nonexistent_hash")
        .await
        .expect("should not error");
    assert!(result.is_none());
}

// ============== Classification Phase 1 ==============

#[tokio::test]
async fn test_classification_high_input_produces_high_ceiling() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Patient Data",
        &[
            ("name", FieldValueType::String),
            ("diagnosis", FieldValueType::String),
        ],
        &[("name", "word"), ("diagnosis", "medical")],
    )
    .await;

    let wasm = b"medical_transform_wasm";
    let request = make_register_request(
        "medical_summary",
        &schema_name,
        &["name", "diagnosis"],
        &[("summary", FieldValueType::String)],
        wasm,
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    // "medical" maps to HIGH in classify_field
    assert_eq!(record.input_ceiling, DataClassification::high());
    // Without Phase 2, assigned = max(ceiling, output) = HIGH
    assert_eq!(record.assigned_classification, DataClassification::high());
}

#[tokio::test]
async fn test_classification_low_input_produces_low_ceiling() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Public Catalog",
        &[("title", FieldValueType::String)],
        &[("title", "word")],
    )
    .await;

    let request = make_register_request(
        "title_transform",
        &schema_name,
        &["title"],
        &[("short_title", FieldValueType::String)],
        b"title_wasm",
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    assert_eq!(record.input_ceiling, DataClassification::low());
    assert_eq!(record.assigned_classification, DataClassification::low());
}

#[tokio::test]
async fn test_classification_medium_input() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Internal Reports",
        &[
            ("report_id", FieldValueType::String),
            ("content", FieldValueType::String),
        ],
        &[("report_id", "word"), ("content", "internal")],
    )
    .await;

    let request = make_register_request(
        "report_summary",
        &schema_name,
        &["report_id", "content"],
        &[("excerpt", FieldValueType::String)],
        b"report_wasm",
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    assert_eq!(record.input_ceiling, DataClassification::medium());
}

// ============== Phase 2 Skipped (no transform-wasm feature) ==============

#[tokio::test]
async fn test_phase2_not_verified_without_feature() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Simple Data",
        &[("value", FieldValueType::String)],
        &[("value", "word")],
    )
    .await;

    let request = make_register_request(
        "simple_transform",
        &schema_name,
        &["value"],
        &[("output", FieldValueType::String)],
        b"simple_wasm",
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    // Without transform-wasm feature, Phase 2 doesn't run
    assert!(!record.classification_verified);
    assert_eq!(record.sample_count, 0);
    assert!(record.nmi_matrix.is_empty());
}

#[tokio::test]
async fn test_phase1_ceiling_used_when_phase2_inconclusive() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Restricted Data",
        &[("ssn", FieldValueType::String)],
        &[("ssn", "pii")],
    )
    .await;

    let request = make_register_request(
        "anonymizer",
        &schema_name,
        &["ssn"],
        &[("anon_id", FieldValueType::String)],
        b"anonymizer_wasm",
    );

    let (record, _) = state
        .register_transform(request)
        .await
        .expect("failed to register");

    // Phase 2 didn't run → ceiling is used
    assert!(!record.classification_verified);
    assert_eq!(record.assigned_classification, DataClassification::high());
}

// ============== Unknown Schema Fails Registration ==============

#[tokio::test]
async fn test_unknown_schema_in_query_fails_registration() {
    let state = make_test_state();

    let request = RegisterTransformRequest {
        name: "bad_transform".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![Query::new(
            "NonExistentSchema".to_string(),
            vec!["field_a".to_string()],
        )],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: None,
        wasm_bytes: b"some_wasm".to_vec(),
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("unknown schema"),
        "Expected error about unknown schema, got: {}",
        err
    );
}

// ============== Validation ==============

#[tokio::test]
async fn test_register_rejects_empty_name() {
    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "  ".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: None,
        wasm_bytes: b"wasm".to_vec(),
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("non-empty"));
}

#[tokio::test]
async fn test_register_rejects_neither_source_nor_bytes() {
    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: None,
        wasm_bytes: vec![],
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("rust_source or wasm_bytes"),
        "Expected either/or error, got: {}",
        err,
    );
}

#[tokio::test]
async fn test_register_rejects_both_source_and_bytes() {
    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: Some("fn transform_impl(input: Value) -> Value { input }".to_string()),
        wasm_bytes: b"wasm".to_vec(),
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("not both"),
        "Expected 'not both' error, got: {}",
        err,
    );
}

#[tokio::test]
async fn test_register_rejects_empty_version() {
    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "test".to_string(),
        version: "  ".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: None,
        wasm_bytes: b"wasm".to_vec(),
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_register_rejects_empty_output_fields() {
    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::new(),
        source_url: None,
        rust_source: None,
        wasm_bytes: b"wasm".to_vec(),
    };

    let result = state.register_transform(request).await;
    assert!(result.is_err());
}

// ============== Listing & Retrieval ==============

#[tokio::test]
async fn test_list_transforms_after_registration() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Data",
        &[("field", FieldValueType::String)],
        &[("field", "word")],
    )
    .await;

    // Register two transforms
    state
        .register_transform(make_register_request(
            "transform_alpha",
            &schema_name,
            &["field"],
            &[("out", FieldValueType::String)],
            b"wasm_alpha",
        ))
        .await
        .expect("failed to register alpha");

    state
        .register_transform(make_register_request(
            "transform_beta",
            &schema_name,
            &["field"],
            &[("out", FieldValueType::String)],
            b"wasm_beta",
        ))
        .await
        .expect("failed to register beta");

    let list = state.get_transform_list().expect("failed to list");
    assert_eq!(list.len(), 2);

    let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"transform_alpha"));
    assert!(names.contains(&"transform_beta"));
}

#[tokio::test]
async fn test_get_transform_by_hash() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Test Records",
        &[("x", FieldValueType::Integer)],
        &[("x", "word")],
    )
    .await;

    let (record, _) = state
        .register_transform(make_register_request(
            "int_transform",
            &schema_name,
            &["x"],
            &[("y", FieldValueType::Integer)],
            b"int_wasm",
        ))
        .await
        .expect("failed to register");

    let retrieved = state
        .get_transform_by_hash(&record.hash)
        .expect("failed to get");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "int_transform");
}

#[test]
fn test_get_transform_by_hash_nonexistent() {
    let state = make_test_state();
    let result = state
        .get_transform_by_hash("nonexistent")
        .expect("should not error");
    assert!(result.is_none());
}

// ============== Persistence Across Restarts ==============

#[tokio::test]
async fn test_transforms_persist_across_restart() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir
        .path()
        .join("persist_test_db")
        .to_string_lossy()
        .to_string();

    let wasm = b"persistent_wasm_module";
    let hash;

    // First session: register a transform
    {
        let state = SchemaServiceState::new(db_path.clone()).expect("failed to create state");
        let schema_name = add_test_schema(
            &state,
            "Persistence Records",
            &[("data", FieldValueType::String)],
            &[("data", "word")],
        )
        .await;

        let (record, _) = state
            .register_transform(make_register_request(
                "persistent_transform",
                &schema_name,
                &["data"],
                &[("result", FieldValueType::String)],
                wasm,
            ))
            .await
            .expect("failed to register");

        hash = record.hash.clone();
    }

    // Second session: verify it's still there
    {
        let state = SchemaServiceState::new(db_path).expect("failed to reopen state");

        let record = state
            .get_transform_by_hash(&hash)
            .expect("failed to get")
            .expect("transform should persist");
        assert_eq!(record.name, "persistent_transform");

        let wasm_bytes = state
            .get_transform_wasm(&hash)
            .await
            .expect("failed to get WASM")
            .expect("WASM should persist");
        assert_eq!(wasm_bytes, wasm.to_vec());
    }
}

// ============== Similar Transforms ==============

#[tokio::test]
async fn test_find_similar_transforms() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Similarity Records",
        &[("f", FieldValueType::String)],
        &[("f", "word")],
    )
    .await;

    state
        .register_transform(make_register_request(
            "downgrade_medical_summary",
            &schema_name,
            &["f"],
            &[("out", FieldValueType::String)],
            b"wasm_medical",
        ))
        .await
        .expect("register failed");

    state
        .register_transform(make_register_request(
            "downgrade_financial_report",
            &schema_name,
            &["f"],
            &[("out", FieldValueType::String)],
            b"wasm_financial",
        ))
        .await
        .expect("register failed");

    // Search for "downgrade" should find both
    let result = state
        .find_similar_transforms("downgrade_something", 0.1)
        .expect("search failed");
    assert_eq!(result.similar_transforms.len(), 2);

    // Search with high threshold
    let result = state
        .find_similar_transforms("completely_unrelated", 0.9)
        .expect("search failed");
    assert!(result.similar_transforms.is_empty());
}

// ============== StoredView transform_hash ==============

#[tokio::test]
async fn test_stored_view_transform_hash_field() {
    use fold_db::schema_service::types::StoredView;

    // Verify StoredView serializes/deserializes with transform_hash
    let view = StoredView {
        name: "test_view".to_string(),
        input_queries: vec![],
        transform_hash: Some("abc123def456".to_string()),
        wasm_bytes: None,
        output_schema_name: "output".to_string(),
        schema_type: fold_db::schema::types::schema::DeclarativeSchemaType::Single,
    };

    let json = serde_json::to_string(&view).expect("serialize failed");
    assert!(json.contains("transform_hash"));
    assert!(json.contains("abc123def456"));

    let back: StoredView = serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(back.transform_hash, Some("abc123def456".to_string()));
}

#[tokio::test]
async fn test_stored_view_without_transform_hash() {
    use fold_db::schema_service::types::StoredView;

    // Backward compatibility: old views without transform_hash still deserialize
    let json = r#"{
        "name": "old_view",
        "input_queries": [],
        "wasm_bytes": null,
        "output_schema_name": "out",
        "schema_type": "Single"
    }"#;

    let view: StoredView = serde_json::from_str(json).expect("deserialize failed");
    assert!(view.transform_hash.is_none());
}

#[tokio::test]
async fn add_view_rejects_empty_input_queries() {
    let state = make_test_state();

    let request = AddViewRequest {
        name: "NoSourceView".to_string(),
        descriptive_name: "No Source View".to_string(),
        input_queries: vec![],
        output_fields: vec!["summary".to_string()],
        field_descriptions: HashMap::from([("summary".to_string(), "summary field".to_string())]),
        field_classifications: HashMap::new(),
        field_data_classifications: HashMap::new(),
        wasm_bytes: None,
        schema_type: fold_db::schema::types::schema::DeclarativeSchemaType::Single,
    };

    let err = state
        .add_view(request)
        .await
        .expect_err("expected add_view to reject empty input_queries");
    assert!(
        err.to_string().contains("at least one input query"),
        "unexpected error message: {err}"
    );
}

// ============== Rust source compilation ==============

/// The tests below exercise the `cargo build --target wasm32-unknown-unknown`
/// path. They're skipped on hosts without the wasm target installed so the
/// main test job stays green on machines that only run library tests.
fn wasm_toolchain_available() -> bool {
    fold_db::schema_service::wasm_compiler::check_wasm_toolchain().is_ok()
}

fn minimal_transform_source() -> &'static str {
    // Full `fn transform_impl(...)` definition that the scaffold wraps.
    // Kept tiny to keep compile time bounded.
    r#"
fn transform_impl(input: Value) -> Value {
    serde_json::json!({ "fields": { "summary": input } })
}
"#
}

#[tokio::test]
async fn test_register_transform_accepts_rust_source_compiles_and_stores() {
    if !wasm_toolchain_available() {
        eprintln!("Skipping: wasm32-unknown-unknown not installed");
        return;
    }

    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Notes",
        &[("body", FieldValueType::String)],
        &[("body", "word")],
    )
    .await;

    let request = RegisterTransformRequest {
        name: "summarize_notes".to_string(),
        version: "1.0.0".to_string(),
        description: Some("compile-from-source test".to_string()),
        input_queries: vec![Query::new(schema_name.clone(), vec!["body".to_string()])],
        output_fields: HashMap::from([("summary".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: Some(minimal_transform_source().to_string()),
        wasm_bytes: vec![],
    };

    let (record, outcome) = state
        .register_transform(request)
        .await
        .expect("register_transform should succeed with rust_source");

    assert!(matches!(outcome, TransformAddOutcome::Added));
    assert!(!record.hash.is_empty(), "wasm hash must be populated");
    let source_hash = record
        .source_hash
        .as_ref()
        .expect("source_hash must be populated when rust_source is submitted");
    assert_eq!(source_hash.len(), 64, "sha256 hex is 64 chars");
    assert_ne!(
        &record.hash, source_hash,
        "wasm hash and source hash are distinct"
    );

    let wasm = state
        .get_transform_wasm(&record.hash)
        .await
        .expect("wasm lookup should not error")
        .expect("wasm bytes must be retrievable by hash");
    assert_eq!(
        &wasm[..4],
        b"\0asm",
        "stored artifact must be a valid WASM module"
    );

    let source = state
        .get_transform_source(&record.hash)
        .await
        .expect("source lookup should not error")
        .expect("source must be retrievable by hash");
    assert_eq!(source, minimal_transform_source());
}

#[tokio::test]
async fn test_register_transform_deterministic_compile_from_same_source() {
    if !wasm_toolchain_available() {
        eprintln!("Skipping: wasm32-unknown-unknown not installed");
        return;
    }

    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Posts",
        &[("text", FieldValueType::String)],
        &[("text", "word")],
    )
    .await;

    let build = |name: &str| RegisterTransformRequest {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![Query::new(schema_name.clone(), vec!["text".to_string()])],
        output_fields: HashMap::from([("summary".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: Some(minimal_transform_source().to_string()),
        wasm_bytes: vec![],
    };

    let (first, out1) = state
        .register_transform(build("first"))
        .await
        .expect("first register should succeed");
    let (second, out2) = state
        .register_transform(build("second"))
        .await
        .expect("second register should succeed");

    assert!(matches!(out1, TransformAddOutcome::Added));
    assert!(
        matches!(out2, TransformAddOutcome::AlreadyExists),
        "same source should produce the same wasm hash and deduplicate"
    );
    assert_eq!(first.hash, second.hash);
}

#[tokio::test]
async fn test_register_transform_rejects_invalid_rust_source() {
    if !wasm_toolchain_available() {
        eprintln!("Skipping: wasm32-unknown-unknown not installed");
        return;
    }

    let state = make_test_state();
    let request = RegisterTransformRequest {
        name: "bad".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        input_queries: vec![],
        output_fields: HashMap::from([("out".to_string(), FieldValueType::String)]),
        source_url: None,
        rust_source: Some(
            "fn transform_impl(input: Value) -> Value { this is not valid rust }".to_string(),
        ),
        wasm_bytes: vec![],
    };

    let err = state
        .register_transform(request)
        .await
        .expect_err("invalid rust must not compile");
    let msg = format!("{}", err);
    assert!(
        msg.contains("compile") || msg.contains("error"),
        "expected compile failure, got: {}",
        msg,
    );
}

#[tokio::test]
async fn test_get_transform_source_returns_none_for_byte_upload() {
    let state = make_test_state();
    let schema_name = add_test_schema(
        &state,
        "Items",
        &[("sku", FieldValueType::String)],
        &[("sku", "word")],
    )
    .await;

    let (record, _) = state
        .register_transform(make_register_request(
            "bytes_only",
            &schema_name,
            &["sku"],
            &[("out", FieldValueType::String)],
            b"pre_compiled_bytes",
        ))
        .await
        .expect("register with pre-compiled bytes should succeed");

    assert!(
        record.source_hash.is_none(),
        "no source_hash when only bytes uploaded"
    );
    let source = state
        .get_transform_source(&record.hash)
        .await
        .expect("source lookup should not error");
    assert!(source.is_none(), "no source stored for byte-only upload");
}
