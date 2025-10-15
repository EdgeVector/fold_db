use datafold::schema_service::server::{SchemaAddOutcome, SchemaServiceState};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

/// Helper function to verify that every outcome returns a valid schema
fn verify_outcome_has_schema(outcome: &SchemaAddOutcome) {
    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert!(!response.name.is_empty(), "added schema must have a name");
            assert!(
                response.definition.fields.is_some(),
                "added schema must have fields defined"
            );
        }
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(
                !conflict.closest_schema.name.is_empty(),
                "closest schema must have a name"
            );
            assert!(
                conflict.closest_schema.definition.fields.is_some(),
                "closest schema must have fields defined"
            );
            assert!(
                conflict.similarity > 0.0 && conflict.similarity <= 1.0,
                "similarity must be between 0 and 1"
            );
        }
    }
}

#[test]
fn closeness_rejects_identical_schema_with_different_name() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "UserProfile",
        "fields": [
            {"name": "user_id", "type": "string"},
            {"name": "email", "type": "string"},
            {"name": "created_at", "type": "number"}
        ]
    });

    let existing_path = temp_dir.path().join("UserProfile.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let duplicate_schema = json!({
        "name": "UserAccount",
        "fields": [
            {"name": "user_id", "type": "string"},
            {"name": "email", "type": "string"},
            {"name": "created_at", "type": "number"}
        ]
    });

    let outcome = state
        .add_schema(duplicate_schema)
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "UserProfile");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => panic!("identical schema should have been rejected"),
    }
}

#[test]
fn closeness_always_returns_schema_on_success() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let new_schema = json!({
        "name": "TestSchema",
        "fields": [
            {"name": "id", "type": "string"}
        ]
    });

    let outcome = state
        .add_schema(new_schema)
        .expect("failed to add schema");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "TestSchema");
            assert!(response.definition.fields.is_some());
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("new unique schema should be added, not rejected")
        }
    }
}

#[test]
fn closeness_always_returns_schema_on_rejection() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "Original",
        "fields": [
            {"name": "field1", "type": "string"},
            {"name": "field2", "type": "number"}
        ]
    });

    let existing_path = temp_dir.path().join("Original.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let duplicate_schema = json!({
        "name": "Duplicate",
        "fields": [
            {"name": "field1", "type": "string"},
            {"name": "field2", "type": "number"}
        ]
    });

    let outcome = state
        .add_schema(duplicate_schema)
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "Original");
            assert!(conflict.closest_schema.definition.fields.is_some());
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("duplicate schema should be rejected with closest schema returned")
        }
    }
}

#[test]
fn closeness_allows_dissimilar_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "UserProfile",
        "fields": [
            {"name": "user_id", "type": "string"},
            {"name": "email", "type": "string"}
        ]
    });

    let existing_path = temp_dir.path().join("UserProfile.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let different_schema = json!({
        "name": "ProductCatalog",
        "fields": [
            {"name": "product_id", "type": "string"},
            {"name": "product_name", "type": "string"},
            {"name": "price", "type": "number"},
            {"name": "inventory_count", "type": "number"}
        ]
    });

    let outcome = state
        .add_schema(different_schema)
        .expect("failed to add dissimilar schema");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "ProductCatalog");
        }
        SchemaAddOutcome::TooSimilar(_) => panic!("dissimilar schema should have been accepted"),
    }
}

#[test]
fn closeness_handles_similar_but_slightly_different_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "User",
        "fields": [
            {"name": "id", "type": "string"},
            {"name": "name", "type": "string"},
            {"name": "email", "type": "string"}
        ]
    });

    let existing_path = temp_dir.path().join("User.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let similar_schema_with_extra_field = json!({
        "name": "UserExtended",
        "fields": [
            {"name": "id", "type": "string"},
            {"name": "name", "type": "string"},
            {"name": "email", "type": "string"},
            {"name": "phone", "type": "string"}
        ]
    });

    let outcome = state
        .add_schema(similar_schema_with_extra_field)
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "UserExtended");
            assert!(response.definition.field_mappers.is_some());
            let mappers = response.definition.field_mappers.as_ref().unwrap();
            assert!(mappers.contains_key("id"));
            assert!(mappers.contains_key("name"));
            assert!(mappers.contains_key("email"));
            assert!(!mappers.contains_key("phone"));
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra field should have been accepted with field mappers")
        }
    }
}

#[test]
fn closeness_uses_normalized_comparison_for_properties() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "First",
        "type": "object",
        "description": "test schema",
        "fields": [
            {"name": "field_a", "type": "string"}
        ]
    });

    let existing_path = temp_dir.path().join("First.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let reordered_properties_schema = json!({
        "description": "test schema",
        "name": "Second",
        "fields": [
            {"name": "field_a", "type": "string"}
        ],
        "type": "object"
    });

    let outcome = state
        .add_schema(reordered_properties_schema)
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "First");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("schemas should be detected as identical despite property ordering")
        }
    }
}

#[test]
fn closeness_ignores_schema_name_in_comparison() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "VeryLongDescriptiveSchemaName",
        "fields": [
            {"name": "field1", "type": "string"}
        ]
    });

    let existing_path = temp_dir.path().join("VeryLongDescriptiveSchemaName.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let same_content_different_name = json!({
        "name": "X",
        "fields": [
            {"name": "field1", "type": "string"}
        ]
    });

    let outcome = state
        .add_schema(same_content_different_name)
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "VeryLongDescriptiveSchemaName");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("schemas should be detected as identical despite different names")
        }
    }
}

#[test]
fn closeness_with_object_style_fields() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "ExistingObject",
        "fields": {
            "field_a": {},
            "field_b": {},
            "field_c": {}
        }
    });

    let existing_path = temp_dir.path().join("ExistingObject.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let similar_object_schema = json!({
        "name": "NewObject",
        "fields": {
            "field_a": {},
            "field_b": {},
            "field_c": {}
        }
    });

    let outcome = state
        .add_schema(similar_object_schema)
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "ExistingObject");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("identical object-style schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_creates_field_mappers_for_high_field_overlap() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "BaseEntity",
        "fields": {
            "id": {},
            "created_at": {},
            "updated_at": {},
            "name": {},
            "description": {}
        }
    });

    let existing_path = temp_dir.path().join("BaseEntity.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let extended_schema = json!({
        "name": "ExtendedEntity",
        "fields": {
            "id": {},
            "created_at": {},
            "updated_at": {},
            "name": {},
            "description": {},
            "extra_field_1": {},
            "extra_field_2": {}
        }
    });

    let outcome = state
        .add_schema(extended_schema)
        .expect("failed to add schema with high field overlap");

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "ExtendedEntity");
            assert!(response.definition.field_mappers.is_some());
            let mappers = response.definition.field_mappers.as_ref().unwrap();
            assert_eq!(mappers.len(), 5);
            assert!(mappers.contains_key("id"));
            assert!(mappers.contains_key("created_at"));
            assert!(mappers.contains_key("updated_at"));
            assert!(mappers.contains_key("name"));
            assert!(mappers.contains_key("description"));
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra fields should be accepted with field mappers")
        }
    }
}

#[test]
fn closeness_with_multiple_existing_schemas_finds_closest() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let schema1 = json!({
        "name": "Schema1",
        "fields": [
            {"name": "a", "type": "string"}
        ]
    });

    let schema2 = json!({
        "name": "Schema2",
        "fields": [
            {"name": "x", "type": "string"},
            {"name": "y", "type": "string"}
        ]
    });

    let schema3 = json!({
        "name": "Schema3",
        "fields": [
            {"name": "x", "type": "string"},
            {"name": "y", "type": "string"},
            {"name": "z", "type": "string"}
        ]
    });

    fs::write(
        temp_dir.path().join("Schema1.json"),
        serde_json::to_string_pretty(&schema1).expect("failed to serialize"),
    )
    .expect("failed to write schema1");

    fs::write(
        temp_dir.path().join("Schema2.json"),
        serde_json::to_string_pretty(&schema2).expect("failed to serialize"),
    )
    .expect("failed to write schema2");

    fs::write(
        temp_dir.path().join("Schema3.json"),
        serde_json::to_string_pretty(&schema3).expect("failed to serialize"),
    )
    .expect("failed to write schema3");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let new_schema = json!({
        "name": "NewSchema",
        "fields": [
            {"name": "x", "type": "string"},
            {"name": "y", "type": "string"}
        ]
    });

    let outcome = state
        .add_schema(new_schema)
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "Schema2");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("schema should match Schema2 as closest duplicate")
        }
    }
}

#[test]
fn closeness_with_nested_objects() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "NestedSchema",
        "fields": [
            {
                "name": "user",
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "name": {"type": "string"}
                }
            }
        ]
    });

    let existing_path = temp_dir.path().join("NestedSchema.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let duplicate_nested = json!({
        "name": "NestedSchemaCopy",
        "fields": [
            {
                "name": "user",
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "name": {"type": "string"}
                }
            }
        ]
    });

    let outcome = state
        .add_schema(duplicate_nested)
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "NestedSchema");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_) => {
            panic!("identical nested schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_field_overlap_below_threshold_without_high_similarity() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "LowOverlap",
        "fields": {
            "common_a": {},
            "common_b": {},
            "unique_1": {},
            "unique_2": {},
            "unique_3": {},
            "unique_4": {},
            "unique_5": {}
        }
    });

    let existing_path = temp_dir.path().join("LowOverlap.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let new_schema = json!({
        "name": "DifferentSchema",
        "fields": {
            "common_a": {},
            "common_b": {},
            "different_1": {},
            "different_2": {},
            "different_3": {},
            "different_4": {},
            "different_5": {}
        }
    });

    let outcome = state
        .add_schema(new_schema)
        .expect("failed to add schema with low overlap");

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "DifferentSchema");
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("low field overlap should allow schema addition")
        }
    }
}

#[test]
fn closeness_with_empty_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let first_empty = json!({
        "name": "Empty1",
        "fields": []
    });

    let outcome1 = state
        .add_schema(first_empty)
        .expect("failed to add first empty schema");

    assert!(matches!(outcome1, SchemaAddOutcome::Added(_)));

    let second_empty = json!({
        "name": "Empty2",
        "fields": []
    });

    let outcome2 = state
        .add_schema(second_empty)
        .expect("failed to evaluate empty schema similarity");

    match outcome2 {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert_eq!(conflict.closest_schema.name, "Empty1");
        }
        SchemaAddOutcome::Added(_) => {
            panic!("two empty schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_respects_field_mapper_preservation() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let schemas_directory = temp_dir.path().to_string_lossy().to_string();

    let existing_schema = json!({
        "name": "Original",
        "fields": {
            "id": {},
            "name": {}
        }
    });

    let existing_path = temp_dir.path().join("Original.json");
    fs::write(
        &existing_path,
        serde_json::to_string_pretty(&existing_schema).expect("failed to serialize"),
    )
    .expect("failed to write existing schema");

    let state = SchemaServiceState::new(schemas_directory.clone())
        .expect("failed to initialize schema service state");

    let new_schema_with_existing_mappers = json!({
        "name": "Extended",
        "fields": {
            "id": {},
            "name": {},
            "email": {}
        },
        "field_mappers": {
            "email": "SomeOtherSchema.email"
        }
    });

    let outcome = state
        .add_schema(new_schema_with_existing_mappers)
        .expect("failed to add schema with existing mappers");

    match outcome {
        SchemaAddOutcome::Added(response) => {
            assert_eq!(response.name, "Extended");
            let mappers = response
                .definition
                .field_mappers
                .as_ref()
                .expect("field mappers should exist");
            assert!(mappers.contains_key("id"));
            assert!(mappers.contains_key("name"));
            assert!(mappers.contains_key("email"));
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra field should be accepted")
        }
    }
}

