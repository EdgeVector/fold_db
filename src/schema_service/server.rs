use actix_cors::Cors;
use actix_web::{web, App, HttpServer as ActixHttpServer};

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;

// Re-export types for backward compatibility (e.g., schema_client.rs imports from here)
pub use super::state::{SchemaServiceState, SchemaStorage};
pub use super::types::*;

#[cfg(feature = "aws-backend")]
pub use super::state::CloudConfig;

// Route handlers (pub(super) visibility — accessible from sibling modules)
use super::routes::{
    add_schema, find_similar, get_available_schemas, get_schema, health_check, list_schemas,
    reload_schemas, reset_database,
};

/// Schema Service HTTP Server
pub struct SchemaServiceServer {
    state: web::Data<SchemaServiceState>,
    bind_address: String,
}

impl SchemaServiceServer {
    /// Create a new schema service server with local sled storage
    pub fn new(db_path: String, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new(db_path)?;

        Ok(Self {
            state: web::Data::new(state),
            bind_address: bind_address.to_string(),
        })
    }

    /// Create a new schema service server with Cloud backend
    #[cfg(feature = "aws-backend")]
    pub async fn new_with_cloud(config: CloudConfig, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new_with_cloud(config).await?;

        Ok(Self {
            state: web::Data::new(state),
            bind_address: bind_address.to_string(),
        })
    }

    /// Run the schema service server
    pub async fn run(&self) -> FoldDbResult<()> {
        log_feature!(
            LogFeature::HttpServer,
            info,
            "Schema service starting on {}",
            self.bind_address
        );

        let state = self.state.clone();

        let server = ActixHttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            App::new().wrap(cors).app_data(state.clone()).service(
                web::scope("/api")
                    .route("/health", web::get().to(health_check))
                    .service(
                        web::resource("/schemas")
                            .route(web::get().to(list_schemas))
                            .route(web::post().to(add_schema)),
                    )
                    .route("/schemas/reload", web::post().to(reload_schemas))
                    .route("/schemas/available", web::get().to(get_available_schemas))
                    .route("/schemas/similar/{name}", web::get().to(find_similar))
                    .route("/schema/{name}", web::get().to(get_schema))
                    .route("/system/reset", web::post().to(reset_database)),
            )
        })
        .bind(&self.bind_address)
        .map_err(|e| FoldDbError::Config(format!("Failed to bind schema service: {}", e)))?
        .run();

        server
            .await
            .map_err(|e| FoldDbError::Config(format!("Schema service error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::Schema;
    use crate::schema_service::state::jaccard_index;
    use std::collections::{HashMap, HashSet};
    use tempfile::tempdir;

    #[tokio::test]
    async fn add_schema_adds_new_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut new_schema = Schema::new(
            "NewSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add classifications
        new_schema.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
        new_schema.field_classifications.insert("value".to_string(), vec!["word".to_string()]);

        let outcome = state
            .add_schema(new_schema.clone(), HashMap::new())
            .await
            .expect("failed to add schema");

        let added_schema = match outcome {
            SchemaAddOutcome::Added(schema, _mutation_mappers) => schema,
            SchemaAddOutcome::AlreadyExists(_) | SchemaAddOutcome::TooSimilar(_) => {
                panic!("schema should have been added")
            }
        };

        // Schema name should be the identity_hash (64 char hex string)
        assert_eq!(added_schema.name.len(), 64); // 64 char hash
        new_schema.compute_identity_hash();
        assert_eq!(&added_schema.name, new_schema.get_identity_hash().unwrap());

        // Classifications should match
        assert_eq!(added_schema.field_classifications, new_schema.field_classifications);

        let stored_schemas = state
            .schemas
            .read()
            .expect("failed to acquire read lock on schema map after addition");

        // Check stored by combined name
        assert!(stored_schemas.contains_key(&added_schema.name));

        // Check the underlying storage
        match &state.storage {
            SchemaStorage::Sled { schemas_tree, .. } => {
                let db_value = schemas_tree
                    .get(added_schema.name.as_bytes())
                    .expect("failed to query database")
                    .expect("schema should exist in database");

                let stored_schema: Schema =
                    serde_json::from_slice(&db_value).expect("failed to deserialize stored schema");

                assert_eq!(stored_schema.name, added_schema.name);
            }
            #[cfg(feature = "aws-backend")]
            _ => panic!("Expected Sled storage"),
        }
    }

    #[tokio::test]
    async fn add_schema_detects_similar_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut existing_schema = Schema::new(
            "Existing".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add classifications
        existing_schema.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
        existing_schema.field_classifications.insert("value".to_string(), vec!["word".to_string()]);

        let mut similar_schema = Schema::new(
            "PotentialDuplicate".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add classifications
        similar_schema.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
        similar_schema.field_classifications.insert("value".to_string(), vec!["word".to_string()]);

        // First schema gets added
        let outcome1 = state
            .add_schema(existing_schema.clone(), HashMap::new())
            .await
            .expect("failed to add existing schema");

        let existing_name = match outcome1 {
            SchemaAddOutcome::Added(schema, _) => {
                // Should be identity_hash (64 char hex string)
                assert_eq!(schema.name.len(), 64);
                schema.name
            }
            SchemaAddOutcome::AlreadyExists(_) | SchemaAddOutcome::TooSimilar(_) => {
                panic!("first schema should be added")
            }
        };

        // Second schema with SAME fields should be detected as already existing
        let outcome2 = state
            .add_schema(similar_schema.clone(), HashMap::new())
            .await
            .expect("failed to evaluate schema similarity");

        match outcome2 {
            SchemaAddOutcome::AlreadyExists(schema) => {
                assert_eq!(schema.name, existing_name);
                assert_eq!(
                    schema.field_classifications,
                    existing_schema.field_classifications
                );
            }
            SchemaAddOutcome::Added(_, _) | SchemaAddOutcome::TooSimilar(_) => {
                panic!("schema with same fields should return AlreadyExists")
            }
        }
    }

    #[tokio::test]
    async fn add_schema_with_different_fields_creates_separate_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        // First schema: 2 fields
        let mut schema1 = Schema::new(
            "UserBasic".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "name".to_string()]),
            None,
            None,
        );
        schema1.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
        schema1.field_classifications.insert("name".to_string(), vec!["name:person".to_string(), "word".to_string()]);

        let outcome1 = state
            .add_schema(schema1.clone(), HashMap::new())
            .await
            .expect("failed to add first schema");

        let schema1_name = match outcome1 {
            SchemaAddOutcome::Added(schema, _) => schema.name,
            other => panic!("expected schema addition, got {:?}", other),
        };

        // Second schema: 3 fields (different field set)
        let mut schema2 = Schema::new(
            "UserExtended".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "id".to_string(),
                "name".to_string(),
                "email".to_string(),
            ]),
            None,
            None,
        );
        schema2.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
        schema2.field_classifications.insert("name".to_string(), vec!["name:person".to_string(), "word".to_string()]);
        schema2.field_classifications.insert("email".to_string(), vec!["email".to_string()]);

        let outcome2 = state
            .add_schema(schema2.clone(), HashMap::new())
            .await
            .expect("failed to add second schema");

        let schema2_name = match outcome2 {
            SchemaAddOutcome::Added(schema, _) => schema.name,
            other => panic!("expected schema addition, got {:?}", other),
        };

        // Should be identity hashes (64 char hex strings)
        assert_eq!(schema1_name.len(), 64);
        assert_eq!(schema2_name.len(), 64);

        // Different field sets should produce different names
        assert_ne!(schema1_name, schema2_name);
    }

    #[tokio::test]
    async fn get_available_schemas_returns_all_schemas() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut schema1 = Schema::new(
            "UserSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "user_id".to_string(),
                "username".to_string(),
                "email".to_string(),
            ]),
            None,
            None,
        );
        schema1.field_classifications.insert("user_id".to_string(), vec!["word".to_string()]);
        schema1.field_classifications.insert("username".to_string(), vec!["word".to_string()]);
        schema1.field_classifications.insert("email".to_string(), vec!["word".to_string()]);

        let mut schema2 = Schema::new(
            "ProductSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "product_id".to_string(),
                "title".to_string(),
                "price".to_string(),
                "description".to_string(),
            ]),
            None,
            None,
        );
        schema2.field_classifications.insert("product_id".to_string(), vec!["word".to_string()]);
        schema2.field_classifications.insert("title".to_string(), vec!["word".to_string()]);
        schema2.field_classifications.insert("price".to_string(), vec!["number".to_string()]);
        schema2.field_classifications.insert("description".to_string(), vec!["word".to_string()]);

        let outcome1 = state
            .add_schema(schema1.clone(), HashMap::new())
            .await
            .expect("failed to add schema1");
        let schema1_name = match outcome1 {
            SchemaAddOutcome::Added(s, _) => s.name,
            _ => panic!("schema1 should be added"),
        };

        let outcome2 = state
            .add_schema(schema2.clone(), HashMap::new())
            .await
            .expect("failed to add schema2");
        let schema2_name = match outcome2 {
            SchemaAddOutcome::Added(s, _) => s.name,
            _ => panic!("schema2 should be added"),
        };

        let schemas = state
            .schemas
            .read()
            .expect("failed to acquire read lock on schemas");
        assert_eq!(schemas.len(), 2);

        // Schemas are now stored by identity_hash
        assert!(schemas.contains_key(&schema1_name));
        assert!(schemas.contains_key(&schema2_name));

        // Different topologies should produce different names
        assert_ne!(schema1_name, schema2_name);
    }

    // ========== find_similar_schemas tests ==========

    /// Helper: create a schema with the given fields, add classifications, and insert it via state.add_schema
    async fn add_test_schema(
        state: &SchemaServiceState,
        name: &str,
        fields: Vec<&str>,
    ) -> String {
        let field_strings: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
        let mut schema = Schema::new(
            name.to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(field_strings.clone()),
            None,
            None,
        );
        for f in &field_strings {
            schema.field_classifications.insert(f.clone(), vec!["word".to_string()]);
        }
        let outcome = state
            .add_schema(schema, HashMap::new())
            .await
            .expect("failed to add test schema");
        match outcome {
            SchemaAddOutcome::Added(s, _) => s.name,
            SchemaAddOutcome::AlreadyExists(s) => s.name,
            SchemaAddOutcome::TooSimilar(c) => c.closest_schema.name,
        }
    }

    fn make_test_state() -> SchemaServiceState {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();
        // Leak the tempdir so it isn't deleted while state is in use
        std::mem::forget(temp_dir);
        SchemaServiceState::new(db_path).expect("failed to create state")
    }

    #[tokio::test]
    async fn find_similar_identical_fields_returns_similarity_1() {
        let state = make_test_state();
        // Two schemas with identical fields produce the same identity_hash,
        // so only one gets stored. We need schemas with different topologies
        // but same field names. Instead, test with overlapping but not identical schemas.
        // Schema A: {a, b}  Schema B: {a, b, c}  → Jaccard = 2/3
        let name_a = add_test_schema(&state, "SchemaA", vec!["a", "b"]).await;
        let _name_b = add_test_schema(&state, "SchemaB", vec!["a", "b", "c"]).await;

        let result = state.find_similar_schemas(&name_a, 0.0).unwrap();
        assert_eq!(result.similar_schemas.len(), 1);
        let entry = &result.similar_schemas[0];
        // Jaccard({a,b}, {a,b,c}) = 2/3
        let expected = 2.0 / 3.0;
        assert!((entry.similarity - expected).abs() < 1e-10);
    }

    #[tokio::test]
    async fn find_similar_partial_overlap() {
        let state = make_test_state();
        // A: {x, y, z}, B: {y, z, w} → Jaccard = 2/4 = 0.5
        let name_a = add_test_schema(&state, "A", vec!["x", "y", "z"]).await;
        let _name_b = add_test_schema(&state, "B", vec!["y", "z", "w"]).await;

        let result = state.find_similar_schemas(&name_a, 0.0).unwrap();
        assert_eq!(result.similar_schemas.len(), 1);
        assert!((result.similar_schemas[0].similarity - 0.5).abs() < 1e-10);
    }

    #[tokio::test]
    async fn find_similar_no_overlap_returns_zero() {
        let state = make_test_state();
        // A: {a, b}, B: {c, d} → Jaccard = 0/4 = 0.0
        let name_a = add_test_schema(&state, "A", vec!["a", "b"]).await;
        let _name_b = add_test_schema(&state, "B", vec!["c", "d"]).await;

        let result = state.find_similar_schemas(&name_a, 0.0).unwrap();
        assert_eq!(result.similar_schemas.len(), 1);
        assert!((result.similar_schemas[0].similarity - 0.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn find_similar_threshold_filters() {
        let state = make_test_state();
        // A: {a, b, c}, B: {a, b, d} → Jaccard = 2/4 = 0.5
        let name_a = add_test_schema(&state, "A", vec!["a", "b", "c"]).await;
        let _name_b = add_test_schema(&state, "B", vec!["a", "b", "d"]).await;

        // Threshold 0.6 should filter out B (similarity = 0.5)
        let result = state.find_similar_schemas(&name_a, 0.6).unwrap();
        assert!(result.similar_schemas.is_empty());

        // Threshold 0.5 should include B
        let result = state.find_similar_schemas(&name_a, 0.5).unwrap();
        assert_eq!(result.similar_schemas.len(), 1);
    }

    #[tokio::test]
    async fn find_similar_nonexistent_schema_returns_error() {
        let state = make_test_state();
        let result = state.find_similar_schemas("nonexistent", 0.5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{}", err).contains("not found"));
    }

    #[tokio::test]
    async fn find_similar_sorted_by_similarity_descending() {
        let state = make_test_state();
        // A: {a, b, c, d}
        // B: {a, b, c, e}       → Jaccard = 3/5 = 0.6
        // C: {a, e, f, g}       → Jaccard = 1/7 ≈ 0.143
        let name_a = add_test_schema(&state, "A", vec!["a", "b", "c", "d"]).await;
        let _name_b = add_test_schema(&state, "B", vec!["a", "b", "c", "e"]).await;
        let _name_c = add_test_schema(&state, "C", vec!["a", "e", "f", "g"]).await;

        let result = state.find_similar_schemas(&name_a, 0.0).unwrap();
        assert_eq!(result.similar_schemas.len(), 2);
        // First entry should have higher similarity
        assert!(result.similar_schemas[0].similarity >= result.similar_schemas[1].similarity);
    }

    #[tokio::test]
    async fn add_schema_deduplicates_across_different_classifications() {
        let state = make_test_state();

        // Schema 1: classifications = ["word"]
        let mut schema1 = Schema::new(
            "ImageA".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["artist".to_string(), "title".to_string()]),
            None,
            None,
        );
        schema1.field_classifications.insert("artist".to_string(), vec!["word".to_string()]);
        schema1.field_classifications.insert("title".to_string(), vec!["word".to_string()]);

        // Schema 2: same fields but different classifications
        let mut schema2 = Schema::new(
            "ImageB".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["artist".to_string(), "title".to_string()]),
            None,
            None,
        );
        schema2.field_classifications.insert("artist".to_string(), vec!["word".to_string(), "name:person".to_string()]);
        schema2.field_classifications.insert("title".to_string(), vec!["word".to_string(), "title".to_string()]);

        // First schema gets added
        let outcome1 = state
            .add_schema(schema1, HashMap::new())
            .await
            .expect("failed to add first schema");
        let first_name = match outcome1 {
            SchemaAddOutcome::Added(schema, _) => schema.name,
            SchemaAddOutcome::AlreadyExists(_) | SchemaAddOutcome::TooSimilar(_) => {
                panic!("first schema should be added")
            }
        };

        // Second schema should be detected as already existing (same identity hash — same field names)
        let outcome2 = state
            .add_schema(schema2, HashMap::new())
            .await
            .expect("failed to evaluate second schema");
        match outcome2 {
            SchemaAddOutcome::AlreadyExists(schema) => {
                assert_eq!(schema.name, first_name);
            }
            other => {
                panic!(
                    "schema with same fields but different classifications should return AlreadyExists, got {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn jaccard_index_both_empty_returns_one() {
        let a: HashSet<String> = HashSet::new();
        let b: HashSet<String> = HashSet::new();
        assert!((jaccard_index(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn jaccard_index_one_empty_returns_zero() {
        let a: HashSet<String> = ["x".to_string()].into_iter().collect();
        let b: HashSet<String> = HashSet::new();
        assert!((jaccard_index(&a, &b) - 0.0).abs() < 1e-10);
    }
}
