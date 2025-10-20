use std::collections::HashMap;
use datafold::schema_service::server::{SchemaAddOutcome, SchemaServiceState};
use serde_json::json;
use tempfile::tempdir;

/// Helper function to add field_topologies to a schema JSON value
fn add_string_topologies(mut schema_json: serde_json::Value, fields: &[&str]) -> serde_json::Value {
    let mut topologies = serde_json::Map::new();
    for field in fields {
        topologies.insert(
            field.to_string(),
            json!({"root": {"type": "Primitive", "value": "String"}})
        );
    }
    schema_json["field_topologies"] = json!(topologies);
    schema_json
}

/// Helper function to convert object-style fields to array with topologies
fn convert_object_fields_to_array(mut schema_json: serde_json::Value) -> serde_json::Value {
    if let Some(fields_obj) = schema_json.get("fields").and_then(|f| f.as_object()).cloned() {
        let field_names: Vec<String> = fields_obj.keys().cloned().collect();
        let mut topologies = serde_json::Map::new();
        
        for field_name in &field_names {
            topologies.insert(
                field_name.clone(),
                json!({"root": {"type": "Primitive", "value": "String"}})
            );
        }
        
        schema_json["fields"] = json!(field_names);
        schema_json["field_topologies"] = json!(topologies);
    }
    schema_json
}

/// Helper function to convert JSON to Schema
fn json_to_schema(value: serde_json::Value) -> datafold::schema::types::Schema {
    serde_json::from_value(value)
        .expect("failed to deserialize schema from JSON")
}

/// Helper function to verify that every outcome returns a valid schema
fn verify_outcome_has_schema(outcome: &SchemaAddOutcome) {
    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "added schema must have a name");
            assert!(
                response.fields.is_some(),
                "added schema must have fields defined"
            );
        }
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(
                !conflict.closest_schema.name.is_empty(),
                "closest schema must have a name"
            );
            assert!(
                conflict.closest_schema.fields.is_some(),
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
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "UserProfile",
        "fields": ["user_id", "email", "created_at"]
    }),
        &["user_id", "email", "created_at"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let duplicate_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "UserAccount",
        "fields": ["user_id", "email", "created_at"]
    }),
        &["user_id", "email", "created_at"]
    ));

    let outcome = state
        .add_schema(duplicate_schema, HashMap::new())
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => panic!("identical schema should have been rejected"),
    }
}

#[test]
fn closeness_always_returns_schema_on_success() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let new_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "TestSchema",
        "fields": ["id"]
    }),
        &["id"]
    ));

    let outcome = state
        .add_schema(new_schema, HashMap::new())
        .expect("failed to add schema");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            // Schema name is now the topology hash, not the original name
            assert!(!response.name.is_empty(), "schema must have a name");
            assert!(response.fields.is_some());
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("new unique schema should be added, not rejected")
        }
    }
}

#[test]
fn closeness_always_returns_schema_on_rejection() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "Original",
        "fields": [
            "field1",
            "field2"
        ]
    }),
        &["field1", "field2"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let duplicate_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "Duplicate",
        "fields": [
            "field1",
            "field2"
        ]
    }),
        &["field1", "field2"]
    ));

    let outcome = state
        .add_schema(duplicate_schema, HashMap::new())
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.closest_schema.fields.is_some());
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("duplicate schema should be rejected with closest schema returned")
        }
    }
}

#[test]
fn closeness_allows_dissimilar_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "UserProfile",
        "fields": [
            "user_id",
            "email"
        ]
    }),
        &["user_id", "email"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let different_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "ProductCatalog",
        "fields": [
            "product_id",
            "product_name",
            "price",
            "inventory_count"
        ]
    }),
        &["product_id", "product_name", "price", "inventory_count"]
    ));

    let outcome = state
        .add_schema(different_schema, HashMap::new())
        .expect("failed to add dissimilar schema");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "schema must have a name");
        }
        SchemaAddOutcome::TooSimilar(_) => panic!("dissimilar schema should have been accepted"),
    }
}

#[test]
fn closeness_handles_similar_but_slightly_different_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "User",
        "fields": [
            "id",
            "name",
            "email"
        ]
    }),
        &["id", "name", "email"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let similar_schema_with_extra_field = json_to_schema(add_string_topologies(
        json!({
        "name": "UserExtended",
        "fields": [
            "id",
            "name",
            "email",
            "phone"
        ]
    }),
        &["id", "name", "email", "phone"]
    ));

    let outcome = state
        .add_schema(similar_schema_with_extra_field, HashMap::new())
        .expect("failed to evaluate schema similarity");

    verify_outcome_has_schema(&outcome);

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "schema must have a name");
            // Note: field_mappers are no longer automatically created by the schema service
            // They must be provided explicitly when adding the schema
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra field should have been accepted")
        }
    }
}

#[test]
fn closeness_uses_normalized_comparison_for_properties() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "First",
        "type": "object",
        "description": "test schema",
        "fields": [
            "field_a"
        ]
    }),
        &["field_a"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let reordered_properties_schema = json_to_schema(add_string_topologies(
        json!({
        "description": "test schema",
        "name": "Second",
        "fields": [
            "field_a"
        ],
        "type": "object"
    }),
        &["field_a"]
    ));

    let outcome = state
        .add_schema(reordered_properties_schema, HashMap::new())
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("schemas should be detected as identical despite property ordering")
        }
    }
}

#[test]
fn closeness_ignores_schema_name_in_comparison() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "VeryLongDescriptiveSchemaName",
        "fields": [
            "field1"
        ]
    }),
        &["field1"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let same_content_different_name = json_to_schema(add_string_topologies(
        json!({
        "name": "X",
        "fields": [
            "field1"
        ]
    }),
        &["field1"]
    ));

    let outcome = state
        .add_schema(same_content_different_name, HashMap::new())
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("schemas should be detected as identical despite different names")
        }
    }
}

#[test]
fn closeness_with_object_style_fields() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(convert_object_fields_to_array(json!({
        "name": "ExistingObject",
        "fields": {
            "field_a": {},
            "field_b": {},
            "field_c": {}
        }
    })));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let similar_object_schema = json_to_schema(convert_object_fields_to_array(json!({
        "name": "NewObject",
        "fields": {
            "field_a": {},
            "field_b": {},
            "field_c": {}
        }
    })));

    let outcome = state
        .add_schema(similar_object_schema, HashMap::new())
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("identical object-style schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_creates_field_mappers_for_high_field_overlap() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(convert_object_fields_to_array(json!({
        "name": "BaseEntity",
        "fields": {
            "id": {},
            "created_at": {},
            "updated_at": {},
            "name": {},
            "description": {}
        }
    })));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let extended_schema = json_to_schema(convert_object_fields_to_array(json!({
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
    })));

    let outcome = state
        .add_schema(extended_schema, HashMap::new())
        .expect("failed to add schema with high field overlap");

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "schema must have a name");
            // Note: field_mappers are no longer automatically created by the schema service
            // Schemas with different fields/topologies are treated as distinct schemas
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra fields should be accepted")
        }
    }
}

#[test]
fn closeness_with_multiple_existing_schemas_finds_closest() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let schema1 = json_to_schema(add_string_topologies(
        json!({
        "name": "Schema1",
        "fields": [
            "a"
        ]
    }),
        &["a"]
    ));

    let schema2 = json_to_schema(add_string_topologies(
        json!({
        "name": "Schema2",
        "fields": [
            "x",
            "y"
        ]
    }),
        &["x", "y"]
    ));

    let schema3 = json_to_schema(add_string_topologies(
        json!({
        "name": "Schema3",
        "fields": [
            "x",
            "y",
            "z"
        ]
    }),
        &["x", "y", "z"]
    ));

    state.add_schema(schema1, HashMap::new()).expect("failed to add schema1");
    state.add_schema(schema2, HashMap::new()).expect("failed to add schema2");
    state.add_schema(schema3, HashMap::new()).expect("failed to add schema3");

    let new_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "NewSchema",
        "fields": [
            "x",
            "y"
        ]
    }),
        &["x", "y"]
    ));

    let outcome = state
        .add_schema(new_schema, HashMap::new())
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("schema should match Schema2 as closest duplicate")
        }
    }
}

#[test]
fn closeness_with_nested_objects() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "NestedSchema",
        "fields": ["user_id", "user_name", "metadata"]
    }),
        &["user_id", "user_name", "metadata"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let duplicate_nested = json_to_schema(add_string_topologies(
        json!({
        "name": "NestedSchemaCopy",
        "fields": ["user_id", "user_name", "metadata"]
    }),
        &["user_id", "user_name", "metadata"]
    ));

    let outcome = state
        .add_schema(duplicate_nested, HashMap::new())
        .expect("failed to evaluate schema similarity");

    match outcome {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
            assert!(conflict.similarity >= 0.9);
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("identical nested schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_field_overlap_below_threshold_without_high_similarity() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(convert_object_fields_to_array(json!({
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
    })));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let new_schema = json_to_schema(convert_object_fields_to_array(json!({
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
    })));

    let outcome = state
        .add_schema(new_schema, HashMap::new())
        .expect("failed to add schema with low overlap");

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "schema must have a name");
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("low field overlap should allow schema addition")
        }
    }
}

#[test]
#[ignore = "empty schemas cannot have topology hashes computed - not supported with topology-based system"]
fn closeness_with_empty_schemas() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let first_empty = json_to_schema(json!({
        "name": "Empty1",
        "fields": []
    }));

    let outcome1 = state
        .add_schema(first_empty, HashMap::new())
        .expect("failed to add first empty schema");

    assert!(matches!(outcome1, SchemaAddOutcome::Added(_, _)));

    let second_empty = json_to_schema(json!({
        "name": "Empty2",
        "fields": []
    }));

    let outcome2 = state
        .add_schema(second_empty, HashMap::new())
        .expect("failed to evaluate empty schema similarity");

    match outcome2 {
        SchemaAddOutcome::TooSimilar(conflict) => {
            assert!(!conflict.closest_schema.name.is_empty(), "closest schema must have a name");
        }
        SchemaAddOutcome::Added(_, _) => {
            panic!("two empty schemas should be detected as similar")
        }
    }
}

#[test]
fn closeness_respects_field_mapper_preservation() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    let existing_schema = json_to_schema(convert_object_fields_to_array(json!({
        "name": "Original",
        "fields": {
            "id": {},
            "name": {}
        }
    })));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    let new_schema_with_existing_mappers = json_to_schema(convert_object_fields_to_array(json!({
        "name": "Extended",
        "fields": {
            "id": {},
            "name": {},
            "email": {}
        },
        "field_mappers": {
            "email": "SomeOtherSchema.email"
        }
    })));

    let outcome = state
        .add_schema(new_schema_with_existing_mappers, HashMap::new())
        .expect("failed to add schema with existing mappers");

    match outcome {
        SchemaAddOutcome::Added(response, _) => {
            assert!(!response.name.is_empty(), "schema must have a name");
            // Verify that explicitly provided field_mappers are preserved
            let mappers = response
                .field_mappers
                .as_ref()
                .expect("field mappers should be preserved");
            assert!(mappers.contains_key("email"), "explicitly provided email mapper should be preserved");
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra field should be accepted")
        }
    }
}

#[test]
#[ignore = "field_mappers are no longer automatically created - this feature needs to be reimplemented"]
fn mutation_mappers_updated_when_field_mappers_added() {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

    let state = SchemaServiceState::new(db_path.clone())
        .expect("failed to initialize schema service state");

    // Add an existing schema
    let existing_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "UserProfile",
        "schema_type": "Single",
        "fields": ["user_id", "username", "email"]
    }),
        &["user_id", "username", "email"]
    ));

    state
        .add_schema(existing_schema, HashMap::new())
        .expect("failed to add existing schema");

    // Propose a new schema with mutation_mappers that map JSON fields to schema fields
    let mut mutation_mappers = HashMap::new();
    mutation_mappers.insert("id".to_string(), "UserProfilePublic.user_id".to_string());
    mutation_mappers.insert("name".to_string(), "UserProfilePublic.username".to_string());
    mutation_mappers.insert("email".to_string(), "UserProfilePublic.email".to_string());
    mutation_mappers.insert("display_name".to_string(), "UserProfilePublic.display_name".to_string());

    let new_schema = json_to_schema(add_string_topologies(
        json!({
        "name": "UserProfilePublic",
        "schema_type": "Single",
        "fields": ["user_id", "username", "email", "display_name"]
    }),
        &["user_id", "username", "email", "display_name"]
    ));

    let outcome = state
        .add_schema(new_schema, mutation_mappers)
        .expect("failed to add schema with mutation mappers");

    match outcome {
        SchemaAddOutcome::Added(response, updated_mutation_mappers) => {
            assert!(!response.name.is_empty(), "schema must have a name");
            
            // Verify field_mappers were created for shared fields
            let field_mappers = response
                .field_mappers
                .as_ref()
                .expect("field mappers should exist");
            assert!(field_mappers.contains_key("user_id"));
            assert!(field_mappers.contains_key("username"));
            assert!(field_mappers.contains_key("email"));
            assert!(!field_mappers.contains_key("display_name"), "display_name is new, should not have field mapper");

            // Verify mutation_mappers were updated to point to existing schema
            assert_eq!(updated_mutation_mappers.get("id").unwrap(), "UserProfile.user_id");
            assert_eq!(updated_mutation_mappers.get("name").unwrap(), "UserProfile.username");
            assert_eq!(updated_mutation_mappers.get("email").unwrap(), "UserProfile.email");
            // display_name is a new field, so it should remain unchanged
            assert_eq!(updated_mutation_mappers.get("display_name").unwrap(), "UserProfilePublic.display_name");
            
            println!("Updated mutation_mappers: {:?}", updated_mutation_mappers);
        }
        SchemaAddOutcome::TooSimilar(_) => {
            panic!("schema with extra field should be accepted with field mappers")
        }
    }
}
