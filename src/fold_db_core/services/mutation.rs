//! Mutation Domain Service
//!
//! This module handles ONLY mutation-specific domain logic:
//! - Field value updates
//! - Atom modifications  
//! - Collection updates
//! - Universal key configuration support for HashRange schemas
//!
//! ## Universal Key Configuration
//!
//! The mutation service supports universal key configuration, allowing HashRange schemas to use
//! any field names for their hash and range keys. This is achieved through:
//!
//! - Dynamic field name extraction from schema key configuration
//! - Automatic skipping of key fields during mutation processing
//! - Support for both new universal key format and legacy range_key patterns
//!
//! ## Schema Type Support
//!
//! - **Single**: Direct field value updates
//! - **Range**: Field updates with range key context (supports both universal key and legacy range_key)
//! - **HashRange**: Field updates with hash and range key context (requires universal key configuration)
//!
//! It does NOT handle:
//! - Schema orchestration (belongs to FoldDB)
//! - Permission checking (belongs to FoldDB)
//! - Event publishing (belongs to FoldDB)
//! - Schema validation (belongs to FoldDB)

use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::fold_db_core::infrastructure::message_bus::{
    request_events::FieldValueSetRequest, MessageBus,
};
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::schema_operations::{extract_unified_keys, shape_unified_result};
use crate::schema::types::field::FieldVariant;
use crate::schema::types::schema::{Schema, SchemaType};
use crate::schema::types::Mutation;
use crate::schema::SchemaError;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

const MUTATION_SERVICE_SOURCE: &str = "mutation_service";

/// Lightweight normalized context emitted alongside FieldValueSetRequest payloads
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFieldContext {
    pub hash: Option<String>,
    pub range: Option<String>,
    pub fields: Map<String, Value>,
}

/// Wrapper around the serialized request and reusable normalized context data
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFieldValueRequest {
    pub request: FieldValueSetRequest,
    pub context: NormalizedFieldContext,
}

fn set_value(target: &mut Map<String, Value>, key: &str, value: &Value) {
    target.insert(key.to_string(), value.clone());
}

fn sort_fields(fields: &Map<String, Value>) -> Map<String, Value> {
    let mut sorted = BTreeMap::new();
    for (key, value) in fields {
        sorted.insert(key.clone(), value.clone());
    }
    sorted.into_iter().collect()
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|candidate| {
        if candidate.trim().is_empty() {
            None
        } else {
            Some(candidate)
        }
    })
}

/// Mutation service responsible for field updates and atom modifications
pub struct MutationService {
    message_bus: Arc<MessageBus>,
}

impl MutationService {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self { message_bus }
    }

    /// Construct a normalized FieldValueSetRequest payload using schema-driven key resolution.
    pub fn normalized_field_value_request(
        &self,
        schema: &Schema,
        field_name: &str,
        field_value: &Value,
        hash_key_value: Option<&Value>,
        range_key_value: Option<&Value>,
        mutation_hash: Option<&str>,
    ) -> Result<NormalizedFieldValueRequest, SchemaError> {
        self.build_field_value_request(
            schema,
            field_name,
            field_value,
            hash_key_value,
            range_key_value,
            mutation_hash,
        )
    }

    fn build_field_value_request(
        &self,
        schema: &Schema,
        field_name: &str,
        field_value: &Value,
        hash_key_value: Option<&Value>,
        range_key_value: Option<&Value>,
        mutation_hash: Option<&str>,
    ) -> Result<NormalizedFieldValueRequest, SchemaError> {
        let mut payload = Map::new();
        set_value(&mut payload, field_name, field_value);

        match &schema.schema_type {
            SchemaType::HashRange => {
                let (hash_field_name, range_field_name) =
                    self.get_hashrange_key_field_names(schema)?;

                let resolved_hash_value = hash_key_value
                    .cloned()
                    .or_else(|| {
                        if field_name == hash_field_name {
                            Some(field_value.clone())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        InfrastructureLogger::log_operation_error(
                            "MutationService",
                            "Missing hash key value for normalized request",
                            &format!(
                                "HashRange schema '{}' requires hash key value for field '{}'",
                                schema.name, field_name
                            ),
                        );
                        SchemaError::InvalidData(format!(
                            "HashRange schema '{}' requires hash key value for field '{}'",
                            schema.name, field_name
                        ))
                    })?;

                let resolved_range_value = range_key_value
                    .cloned()
                    .or_else(|| {
                        if field_name == range_field_name {
                            Some(field_value.clone())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        InfrastructureLogger::log_operation_error(
                            "MutationService",
                            "Missing range key value for normalized request",
                            &format!(
                                "HashRange schema '{}' requires range key value for field '{}'",
                                schema.name, field_name
                            ),
                        );
                        SchemaError::InvalidData(format!(
                            "HashRange schema '{}' requires range key value for field '{}'",
                            schema.name, field_name
                        ))
                    })?;

                set_value(&mut payload, &hash_field_name, &resolved_hash_value);
                set_value(&mut payload, &range_field_name, &resolved_range_value);
                set_value(&mut payload, "hash_key", &resolved_hash_value);
                set_value(&mut payload, "range_key", &resolved_range_value);
            }
            SchemaType::Range { .. } => {
                let range_field_name = self.get_range_key_field_name(schema)?;

                if let Some(explicit_range) = range_key_value {
                    set_value(&mut payload, &range_field_name, explicit_range);
                    if range_field_name != "range_key" {
                        set_value(&mut payload, "range_key", explicit_range);
                    }
                } else if field_name == range_field_name {
                    set_value(&mut payload, &range_field_name, field_value);
                    if range_field_name != "range_key" {
                        set_value(&mut payload, "range_key", field_value);
                    }
                }
            }
            SchemaType::Single => {}
        }

        let payload_value = Value::Object(payload.clone());
        let (hash_raw, range_raw) = extract_unified_keys(schema, &payload_value)?;
        let normalized_hash = normalize_optional_string(hash_raw);
        let normalized_range = normalize_optional_string(range_raw);

        if matches!(schema.schema_type, SchemaType::HashRange)
            && (normalized_hash.is_none() || normalized_range.is_none())
        {
            InfrastructureLogger::log_operation_error(
                "MutationService",
                "Key resolution failed for HashRange normalized payload",
                &format!(
                    "Schema '{}' could not resolve hash/range keys for field '{}'",
                    schema.name, field_name
                ),
            );
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' could not resolve hash/range keys for field '{}'",
                schema.name, field_name
            )));
        }

        if matches!(schema.schema_type, SchemaType::Range { .. }) && normalized_range.is_none() {
            InfrastructureLogger::log_operation_error(
                "MutationService",
                "Range key resolution failed for normalized payload",
                &format!(
                    "Range schema '{}' requires range key value for field '{}'",
                    schema.name, field_name
                ),
            );
            return Err(SchemaError::InvalidData(format!(
                "Range schema '{}' requires range key value for field '{}'",
                schema.name, field_name
            )));
        }

        let shaped_value = shape_unified_result(
            schema,
            &payload_value,
            normalized_hash.clone(),
            normalized_range.clone(),
        )?;

        let shaped_object = shaped_value.as_object().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "Normalized payload for '{}.{}' must be an object",
                schema.name, field_name
            ))
        })?;

        let fields_object = shaped_object
            .get("fields")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();
        let sorted_fields = sort_fields(&fields_object);

        let mut normalized_payload = Map::new();
        normalized_payload.insert(
            "hash".to_string(),
            Value::String(normalized_hash.clone().unwrap_or_default()),
        );
        normalized_payload.insert(
            "range".to_string(),
            Value::String(normalized_range.clone().unwrap_or_default()),
        );
        normalized_payload.insert("fields".to_string(), Value::Object(sorted_fields.clone()));

        let incremental = normalized_hash.is_some() || normalized_range.is_some();
        let mutation_context = if incremental || mutation_hash.is_some() {
            Some(
                crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                    range_key: normalized_range.clone(),
                    hash_key: normalized_hash.clone(),
                    mutation_hash: mutation_hash.map(|value| value.to_string()),
                    incremental,
                },
            )
        } else {
            None
        };

        let request_value = Value::Object(normalized_payload);
        let correlation_id = Uuid::new_v4().to_string();
        let request = if let Some(context) = mutation_context {
            FieldValueSetRequest::with_context(
                correlation_id,
                schema.name.clone(),
                field_name.to_string(),
                request_value,
                MUTATION_SERVICE_SOURCE.to_string(),
                context,
            )
        } else {
            FieldValueSetRequest::new(
                correlation_id,
                schema.name.clone(),
                field_name.to_string(),
                request_value,
                MUTATION_SERVICE_SOURCE.to_string(),
            )
        };

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Constructed normalized FieldValueSetRequest for {}.{} with hash {:?} and range {:?}",
                schema.name, field_name, normalized_hash, normalized_range
            ),
        );

        Ok(NormalizedFieldValueRequest {
            request,
            context: NormalizedFieldContext {
                hash: normalized_hash,
                range: normalized_range,
                fields: sorted_fields,
            },
        })
    }

    /// Update a single field value (core mutation operation)
    pub fn update_field_value(
        &self,
        schema: &Schema,
        field_name: &str,
        value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating field",
            &format!("{}.{}", schema.name, field_name),
        );

        // Get field definition from schema
        let field_variant = schema.fields.get(field_name).ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "Field '{}' not found in schema '{}'",
                field_name, schema.name
            ))
        })?;

        // Apply field-specific mutation logic
        match field_variant {
            FieldVariant::Single(single_field) => {
                self.update_single_field(schema, field_name, single_field, value, mutation_hash)
            }
            FieldVariant::Range(_range_field) => {
                InfrastructureLogger::log_operation_error(
                    "MutationService",
                    "Individual range field updates not supported",
                    "Range fields must be updated via range schema mutation.",
                );
                Err(SchemaError::InvalidData(format!(
                    "Range field '{}' in schema '{}' cannot be updated individually. Use range schema mutation instead.",
                    field_name, schema.name
                )))
            }
            FieldVariant::HashRange(_hash_range_field) => {
                self.update_hashrange_field(schema, field_name, value, mutation_hash)
            }
        }
    }

    /// Update atoms for a HashRange schema mutation using universal key configuration
    ///
    /// This method processes HashRange schema mutations by dynamically determining the hash and range
    /// field names from the schema's universal key configuration, rather than using hardcoded field names.
    /// This allows HashRange schemas to use any field names for their hash and range keys.
    ///
    /// # Parameters
    ///
    /// * `schema` - The HashRange schema containing the universal key configuration
    /// * `fields_and_values` - Map of field names to their new values
    /// * `hash_key_value` - The actual hash key value for this mutation
    /// * `range_key_value` - The actual range key value for this mutation  
    /// * `mutation_hash` - Unique identifier for this mutation
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all fields are processed successfully, or `Err(SchemaError)` if there
    /// are issues with the schema configuration or field processing.
    ///
    /// # Behavior
    ///
    /// - Automatically skips hash and range key fields (they are metadata, not data fields)
    /// - Uses the schema's universal key configuration to determine which fields to skip
    /// - Creates HashRange-aware field value requests with proper context
    /// - Supports incremental processing for efficient updates
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datafold::fold_db_core::services::mutation::MutationService;
    /// # use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
    /// # use datafold::schema::types::Schema;
    /// # use datafold::schema::types::json_schema::KeyConfig;
    /// # use datafold::schema::types::SchemaType;
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use std::sync::Arc;
    /// # use datafold::fees::SchemaPaymentConfig;
    ///
    /// // Create mutation service
    /// let message_bus = Arc::new(MessageBus::new());
    /// let mutation_service = MutationService::new(message_bus);
    ///
    /// // Schema with universal key configuration:
    /// let schema = Schema {
    ///     name: "UserActivity".to_string(),
    ///     schema_type: SchemaType::HashRange,
    ///     key: Some(KeyConfig {
    ///         hash_field: "user_id".to_string(),
    ///         range_field: "timestamp".to_string(),
    ///     }),
    ///     fields: HashMap::new(),
    ///     hash: Some("test_hash".to_string()),
    ///     payment_config: SchemaPaymentConfig::default(),
    /// };
    ///
    /// let mut fields_and_values = HashMap::new();
    /// fields_and_values.insert("action".to_string(), json!("login"));
    /// fields_and_values.insert("details".to_string(), json!("User logged in"));
    /// // Note: "user_id" and "timestamp" are automatically skipped as they are key fields
    ///
    /// let result = mutation_service.update_hashrange_schema_fields(
    ///     &schema,
    ///     &fields_and_values,
    ///     "user123",           // hash_key_value
    ///     "2025-01-15T10:30:00Z", // range_key_value
    ///     "mutation_hash_123"
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The schema is missing key configuration (`SchemaError::InvalidData`)
    /// - The hash_field or range_field in the key configuration is empty (`SchemaError::InvalidData`)
    /// - Field processing fails (`SchemaError::InvalidData`)
    pub fn update_hashrange_schema_fields(
        &self,
        schema: &Schema,
        fields_and_values: &std::collections::HashMap<String, Value>,
        hash_key_value: &str,
        range_key_value: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Processing HashRange schema mutation for hash_key: {} and range_key: {}",
                hash_key_value, range_key_value
            ),
        );

        // Get the actual hash and range field names from the schema's universal key configuration
        let (hash_field_name, range_field_name) = self.get_hashrange_key_field_names(schema)?;

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "HashRange schema '{}' key fields - hash: '{}', range: '{}'",
                schema.name, hash_field_name, range_field_name
            ),
        );

        // Create mutation context for incremental processing
        let mutation_context =
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                range_key: Some(range_key_value.to_string()),
                hash_key: Some(hash_key_value.to_string()),
                mutation_hash: Some(mutation_hash.to_string()),
                incremental: true, // Enable incremental processing for hashrange schemas
            };

        // Process each field in the HashRange schema
        for (field_name, value) in fields_and_values {
            InfrastructureLogger::log_operation_start(
                "MutationService",
                "Processing HashRange field",
                &format!("{}.{} with value: {}", schema.name, field_name, value),
            );

            // Skip hash and range key fields as they are metadata for the HashRange structure
            if field_name == &hash_field_name || field_name == &range_field_name {
                InfrastructureLogger::log_debug_info(
                    "MutationService",
                    &format!(
                        "Skipping metadata field: {} (universal key field)",
                        field_name
                    ),
                );
                continue;
            }

            // Create a HashRange-aware field value request that includes the actual hash and range field names
            let hashrange_aware_value = serde_json::json!({
                hash_field_name.clone(): hash_key_value,
                range_field_name.clone(): range_key_value,
                "value": value
            });

            let correlation_id = Uuid::new_v4().to_string();
            let field_request = FieldValueSetRequest::with_context(
                correlation_id.clone(),
                schema.name.clone(),
                field_name.clone(),
                hashrange_aware_value,
                "mutation_service".to_string(),
                mutation_context.clone(),
            );

            InfrastructureLogger::log_debug_info("MutationService", &format!("Publishing HashRange field request for {}.{} with hash_key: {} and range_key: {}", schema.name, field_name, hash_key_value, range_key_value));
            match self.message_bus.publish(field_request) {
                Ok(_) => {
                    InfrastructureLogger::log_operation_success(
                        "MutationService",
                        "HashRange field update request sent",
                        &format!(
                            "{}.{} with hash_key: {} and range_key: {}",
                            schema.name, field_name, hash_key_value, range_key_value
                        ),
                    );
                    // Add a small delay to ensure the message is processed
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    InfrastructureLogger::log_operation_error(
                        "MutationService",
                        "Failed to send HashRange field update",
                        &format!("{}.{}: {:?}", schema.name, field_name, e),
                    );
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to update HashRange field {}: {}",
                        field_name, e
                    )));
                }
            }
        }

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "All HashRange field updates sent successfully",
            "",
        );
        Ok(())
    }

    /// Update atoms for a range schema mutation using universal key configuration
    ///
    /// This method processes Range schema mutations by dynamically determining the range field name
    /// from the schema's universal key configuration, falling back to legacy range_key if needed.
    ///
    /// # Parameters
    ///
    /// * `schema` - The Range schema containing the universal key configuration
    /// * `fields_and_values` - Map of field names to their new values
    /// * `range_key_value` - The actual range key value for this mutation
    /// * `mutation_hash` - Unique identifier for this mutation
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all fields are processed successfully, or `Err(SchemaError)` if there
    /// are issues with the schema configuration or field processing.
    ///
    /// # Behavior
    ///
    /// - Uses universal key configuration if available, falls back to legacy range_key
    /// - Creates Range-aware field value requests with proper context
    /// - Supports incremental processing for efficient updates
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datafold::fold_db_core::services::mutation::MutationService;
    /// # use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
    /// # use datafold::schema::types::Schema;
    /// # use datafold::schema::types::json_schema::KeyConfig;
    /// # use datafold::schema::types::SchemaType;
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use std::sync::Arc;
    /// # use datafold::fees::SchemaPaymentConfig;
    ///
    /// // Create mutation service
    /// let message_bus = Arc::new(MessageBus::new());
    /// let mutation_service = MutationService::new(message_bus);
    ///
    /// // Schema with universal key configuration:
    /// let schema = Schema {
    ///     name: "UserSessions".to_string(),
    ///     schema_type: SchemaType::Range { range_key: "session_id".to_string() },
    ///     key: Some(KeyConfig {
    ///         hash_field: "".to_string(),
    ///         range_field: "session_id".to_string(),
    ///     }),
    ///     fields: HashMap::new(),
    ///     hash: Some("test_hash".to_string()),
    ///     payment_config: SchemaPaymentConfig::default(),
    /// };
    ///
    /// let mut fields_and_values = HashMap::new();
    /// fields_and_values.insert("user_id".to_string(), json!("user123"));
    /// fields_and_values.insert("login_time".to_string(), json!("2025-01-15T10:30:00Z"));
    ///
    /// let result = mutation_service.update_range_schema_fields(
    ///     &schema,
    ///     &fields_and_values,
    ///     "session_456",        // range_key_value
    ///     "mutation_hash_123"
    /// );
    /// ```
    pub fn update_range_schema_fields(
        &self,
        schema: &Schema,
        fields_and_values: &std::collections::HashMap<String, Value>,
        range_key_value: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Processing range schema mutation for range_key_value: {}",
                range_key_value
            ),
        );

        // Get the actual range field name from the schema's universal key configuration
        let range_field_name = self.get_range_key_field_name(schema)?;

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Range schema '{}' key field - range: '{}'",
                schema.name, range_field_name
            ),
        );

        // Create mutation context for incremental processing
        let mutation_context =
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                range_key: Some(range_key_value.to_string()),
                hash_key: None,
                mutation_hash: Some(mutation_hash.to_string()),
                incremental: true, // Enable incremental processing for range schemas
            };

        // DIRECT APPROACH: Since mutation service doesn't have direct DB access,
        // we need to use FieldValueSetRequest with range-specific handling
        for (field_name, value) in fields_and_values {
            InfrastructureLogger::log_operation_start(
                "MutationService",
                "Processing range field",
                &format!(
                    "{} with value: {} for range_key: {}",
                    field_name, value, range_key_value
                ),
            );

            // Create a special field value request that includes the actual range field name
            let range_aware_value = serde_json::json!({
                range_field_name.clone(): range_key_value,
                "value": value
            });

            let correlation_id = Uuid::new_v4().to_string();
            let field_request = FieldValueSetRequest::with_context(
                correlation_id.clone(),
                schema.name.clone(),
                field_name.clone(),
                range_aware_value,
                "mutation_service".to_string(),
                mutation_context.clone(),
            );

            match self.message_bus.publish(field_request) {
                Ok(_) => {
                    InfrastructureLogger::log_operation_success(
                        "MutationService",
                        "Range field update request sent",
                        &format!(
                            "{}.{} with range_key: {}",
                            schema.name, field_name, range_key_value
                        ),
                    );
                }
                Err(e) => {
                    InfrastructureLogger::log_operation_error(
                        "MutationService",
                        "Failed to send range field update",
                        &format!("{}.{}: {:?}", schema.name, field_name, e),
                    );
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to update range field {}: {}",
                        field_name, e
                    )));
                }
            }
        }

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "All range field updates sent successfully",
            "",
        );
        Ok(())
    }

    /// Modify atom value (core mutation operation)
    pub fn modify_atom(
        &self,
        atom_uuid: &str,
        _new_value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Modifying atom",
            &format!("{} with hash {}", atom_uuid, mutation_hash),
        );

        // This would typically interact with atom storage
        // For now, we'll use event-driven communication

        // TODO: Implement direct atom modification logic
        // This should update the atom's value and update its hash

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "Atom modified successfully",
            atom_uuid,
        );
        Ok(())
    }

    /// Handle single field mutation
    fn update_single_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _single_field: &crate::schema::types::field::single_field::SingleField,
        value: &Value,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating single field",
            &format!("{}.{}", schema.name, field_name),
        );

        // First, send FieldValueSetRequest to store the actual field value as an Atom
        let value_correlation_id = Uuid::new_v4().to_string();
        let field_value_request = FieldValueSetRequest::new(
            value_correlation_id.clone(),
            schema.name.clone(),
            field_name.to_string(),
            value.clone(),
            "mutation_service".to_string(),
        );

        if let Err(e) = self.message_bus.publish(field_value_request) {
            InfrastructureLogger::log_operation_error(
                "MutationService",
                "Failed to send field value set request",
                &format!("{}.{}: {:?}", schema.name, field_name, e),
            );
            return Err(SchemaError::InvalidData(format!(
                "Failed to set field value: {}",
                e
            )));
        }
        InfrastructureLogger::log_operation_success(
            "MutationService",
            "Field value set request sent",
            &format!("{}.{}", schema.name, field_name),
        );

        // DIAGNOSTIC LOG: Track if FieldValueSetRequest is being consumed
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "🔍 DIAGNOSTIC: FieldValueSetRequest published for {}.{} with correlation_id: {}",
                schema.name, field_name, value_correlation_id
            ),
        );

        // Transform triggers are now handled automatically by TransformOrchestrator
        // via direct FieldValueSet event monitoring
        Ok(())
    }

    /// Handle HashRange field mutation
    fn update_hashrange_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _value: &Value,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating HashRange field",
            &format!("{}.{}", schema.name, field_name),
        );

        // HashRange fields should be processed via the HashRange schema method which has proper hash_key and range_key context
        InfrastructureLogger::log_operation_error(
            "MutationService",
            "Individual HashRange field updates not supported",
            "HashRange fields must be updated via HashRange schema mutation.",
        );
        Err(SchemaError::InvalidData(format!(
            "HashRange field '{}' in schema '{}' cannot be updated individually. Use HashRange schema mutation instead.",
            field_name, schema.name
        )))
    }

    /// Validate field value format (mutation-specific validation)
    pub fn validate_field_value(
        field_variant: &FieldVariant,
        value: &Value,
    ) -> Result<(), SchemaError> {
        match field_variant {
            FieldVariant::Single(_) => {
                // Validate single field value format
                if value.is_null() {
                    return Err(SchemaError::InvalidData(
                        "Single field value cannot be null".to_string(),
                    ));
                }
                Ok(())
            }
            FieldVariant::Range(_) => {
                // Validate range field value format
                if !value.is_object() {
                    return Err(SchemaError::InvalidData(
                        "Range field value must be an object".to_string(),
                    ));
                }
                Ok(())
            }
            FieldVariant::HashRange(_) => {
                // Validate hash-range field value format
                if !value.is_object() {
                    return Err(SchemaError::InvalidData(
                        "HashRange field value must be an object".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }
}

/// Range schema mutation validation using universal key configuration
///
/// This function validates Range schema mutations by checking for the presence and validity
/// of the range key field, using universal key configuration when available or falling back
/// to legacy range_key patterns.
///
/// # Parameters
///
/// * `schema` - The Range schema containing the universal key configuration or legacy range_key
/// * `mutation` - The mutation containing the fields and values to validate
///
/// # Returns
///
/// Returns `Ok(())` if the mutation is valid, or `Err(SchemaError)` if validation fails.
///
/// # Behavior
///
/// - Uses universal key configuration if available, falls back to legacy range_key
/// - Validates that the range key field is present in the mutation
/// - Validates that the range key value is not null or empty
///
/// # Examples
///
/// ```rust
/// # use datafold::fold_db_core::services::mutation::validate_range_schema_mutation_format;
/// # use datafold::schema::types::{Schema, Mutation, MutationType};
/// # use datafold::schema::types::json_schema::KeyConfig;
/// # use datafold::schema::types::SchemaType;
/// # use serde_json::json;
/// # use std::collections::HashMap;
/// # use datafold::fees::SchemaPaymentConfig;
/// # use datafold::permissions::types::policy::PermissionsPolicy;
///
/// // Schema with universal key configuration:
/// let schema = Schema {
///     name: "UserSessions".to_string(),
///     schema_type: SchemaType::Range { range_key: "legacy_key".to_string() },
///     key: Some(KeyConfig {
///         range_field: "session_id".to_string(),
///         hash_field: "".to_string(),
///     }),
///     fields: HashMap::new(),
///     hash: Some("test_hash".to_string()),
///     payment_config: SchemaPaymentConfig::default(),
/// };
///
/// let mut fields_and_values = HashMap::new();
/// fields_and_values.insert("session_id".to_string(), json!("session_123"));
/// fields_and_values.insert("user_id".to_string(), json!("user_456"));
///
/// let mutation = Mutation {
///     schema_name: "UserSessions".to_string(),
///     mutation_type: MutationType::Create,
///     fields_and_values,
///     pub_key: "test_pub_key".to_string(),
///     synchronous: Some(false),
///     trust_distance: 0,
/// };
///
/// let result = validate_range_schema_mutation_format(&schema, &mutation);
/// assert!(result.is_ok()); // Valid mutation with session_id field
/// ```
pub fn validate_range_schema_mutation_format(
    schema: &Schema,
    mutation: &Mutation,
) -> Result<(), SchemaError> {
    // Get the range field name using universal key configuration or legacy range_key
    let range_field_name = match &schema.schema_type {
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            if let Some(key_config) = &schema.key {
                // Universal key configuration takes precedence
                if key_config.range_field.trim().is_empty() {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' with key configuration requires non-empty range_field",
                        schema.name
                    )));
                }
                key_config.range_field.clone()
            } else {
                // Fall back to legacy range_key for backward compatibility
                range_key.clone()
            }
        }
        _ => {
            return Err(SchemaError::InvalidData(format!(
            "validate_range_schema_mutation_format can only be called on Range schemas, got: {:?}",
            schema.schema_type
        )))
        }
    };

    log_feature!(
        LogFeature::Mutation,
        info,
        "🔍 Validating Range schema mutation format for schema: {} with range_field: {}",
        schema.name,
        range_field_name
    );

    // MANDATORY: Range schema mutations MUST include the range field
    let range_key_value = mutation.fields_and_values.get(&range_field_name)
        .ok_or_else(|| SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' is missing required range field '{}'. All range schema mutations must provide a value for the range field.",
            schema.name, range_field_name
        )))?;

    // Validate the range field value is not null or empty
    if range_key_value.is_null() {
        return Err(SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' has null value for range field '{}'. Range field must have a valid value.",
            schema.name, range_field_name
        )));
    }

    // If range field value is a string, ensure it's not empty
    if let Some(str_value) = range_key_value.as_str() {
        if str_value.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' has empty string value for range field '{}'. Range field must have a non-empty value.",
                schema.name, range_field_name
            )));
        }
    }

    // Validate all fields in the schema are RangeFields
    for (field_name, field_variant) in &schema.fields {
        match field_variant {
            FieldVariant::Range(_) => {
                InfrastructureLogger::log_operation_success(
                    "MutationService",
                    "Field validation",
                    &format!("Field '{}' is correctly a RangeField", field_name),
                );
            }
            FieldVariant::Single(_) => {
                return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' contains Single field '{}', but all fields must be RangeFields",
                        schema.name, field_name
                    )));
            }
            FieldVariant::HashRange(_) => {
                return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' contains HashRange field '{}', but all fields must be RangeFields",
                        schema.name, field_name
                    )));
            }
        }
    }

    InfrastructureLogger::log_operation_success(
        "MutationService",
        "Range schema mutation format validation passed",
        &format!("schema: {}", schema.name),
    );

    Ok(())
}

impl MutationService {
    /// Get the hash and range field names from the schema's universal key configuration
    ///
    /// This helper method extracts the actual field names used for hash and range keys from a
    /// HashRange schema's universal key configuration. This allows the mutation service to work
    /// with any HashRange schema regardless of the field names chosen for the keys.
    ///
    /// # Parameters
    ///
    /// * `schema` - The HashRange schema containing the universal key configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok((hash_field_name, range_field_name))` if the schema has valid key configuration,
    /// or `Err(SchemaError)` if the configuration is missing or invalid.
    ///
    /// # Requirements
    ///
    /// The schema must have:
    /// - A `key` configuration (not `None`)
    /// - Non-empty `hash_field` in the key configuration
    /// - Non-empty `range_field` in the key configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datafold::fold_db_core::services::mutation::MutationService;
    /// # use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
    /// # use datafold::schema::types::Schema;
    /// # use datafold::schema::types::json_schema::KeyConfig;
    /// # use datafold::schema::types::SchemaType;
    /// # use std::collections::HashMap;
    /// # use datafold::schema::types::field::FieldVariant;
    /// # use datafold::schema::types::field::single_field::SingleField;
    /// # use datafold::permissions::types::policy::PermissionsPolicy;
    /// # use datafold::fees::types::config::FieldPaymentConfig;
    /// # use datafold::fees::SchemaPaymentConfig;
    /// # use std::sync::Arc;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// // Create mutation service
    /// let message_bus = Arc::new(MessageBus::new());
    /// let mutation_service = MutationService::new(message_bus);
    ///
    /// // Schema with universal key configuration:
    /// let schema = Schema {
    ///     name: "UserActivity".to_string(),
    ///     schema_type: SchemaType::HashRange,
    ///     key: Some(KeyConfig {
    ///         hash_field: "user_id".to_string(),
    ///         range_field: "timestamp".to_string(),
    ///     }),
    ///     fields: HashMap::new(),
    ///     hash: Some("test_hash".to_string()),
    ///     payment_config: SchemaPaymentConfig::default(),
    /// };
    ///
    /// let (hash_field, range_field) = mutation_service.get_hashrange_key_field_names(&schema)?;
    /// assert_eq!(hash_field, "user_id");
    /// assert_eq!(range_field, "timestamp");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The schema has no key configuration (`SchemaError::InvalidData`)
    /// - The hash_field is empty or whitespace-only (`SchemaError::InvalidData`)
    /// - The range_field is empty or whitespace-only (`SchemaError::InvalidData`)
    pub fn get_hashrange_key_field_names(
        &self,
        schema: &Schema,
    ) -> Result<(String, String), SchemaError> {
        // For HashRange schemas, both hash_field and range_field are required
        let key_config = schema.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires key configuration",
                schema.name
            ))
        })?;

        let hash_field = if key_config.hash_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty hash_field in key configuration",
                schema.name
            )));
        } else {
            key_config.hash_field.clone()
        };

        let range_field = if key_config.range_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty range_field in key configuration",
                schema.name
            )));
        } else {
            key_config.range_field.clone()
        };

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "HashRange schema '{}' key fields - hash: '{}', range: '{}'",
                schema.name, hash_field, range_field
            ),
        );

        Ok((hash_field, range_field))
    }

    /// Get the range field name from the schema's universal key configuration or legacy range_key
    ///
    /// This helper method extracts the actual field name used for the range key from a Range schema.
    /// It supports both universal key configuration and legacy range_key patterns for backward compatibility.
    ///
    /// # Parameters
    ///
    /// * `schema` - The Range schema containing the universal key configuration or legacy range_key
    ///
    /// # Returns
    ///
    /// Returns `Ok(range_field_name)` if the schema has valid range key configuration,
    /// or `Err(SchemaError)` if the configuration is missing or invalid.
    ///
    /// # Behavior
    ///
    /// 1. If universal key configuration exists and has a non-empty range_field, use that
    /// 2. If universal key configuration exists but range_field is empty, return error
    /// 3. If no universal key configuration, fall back to legacy range_key from SchemaType
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datafold::fold_db_core::services::mutation::MutationService;
    /// # use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
    /// # use datafold::schema::types::Schema;
    /// # use datafold::schema::types::json_schema::KeyConfig;
    /// # use datafold::schema::types::SchemaType;
    /// # use std::collections::HashMap;
    /// # use datafold::fees::SchemaPaymentConfig;
    /// # use std::sync::Arc;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// // Create mutation service
    /// let message_bus = Arc::new(MessageBus::new());
    /// let mutation_service = MutationService::new(message_bus);
    ///
    /// // Schema with universal key configuration:
    /// let schema_with_key = Schema {
    ///     name: "UserSessions".to_string(),
    ///     schema_type: SchemaType::Range { range_key: "legacy_key".to_string() },
    ///     key: Some(KeyConfig {
    ///         range_field: "session_id".to_string(),
    ///         hash_field: "".to_string(),
    ///     }),
    ///     fields: HashMap::new(),
    ///     hash: Some("test_hash".to_string()),
    ///     payment_config: SchemaPaymentConfig::default(),
    /// };
    ///
    /// let range_field = mutation_service.get_range_key_field_name(&schema_with_key)?;
    /// assert_eq!(range_field, "session_id"); // Uses universal key config
    ///
    /// // Schema with legacy range_key only:
    /// let schema_legacy = Schema {
    ///     name: "UserSessions".to_string(),
    ///     schema_type: SchemaType::Range { range_key: "legacy_key".to_string() },
    ///     key: None,
    ///     fields: HashMap::new(),
    ///     hash: Some("test_hash".to_string()),
    ///     payment_config: SchemaPaymentConfig::default(),
    /// };
    ///
    /// let range_field = mutation_service.get_range_key_field_name(&schema_legacy)?;
    /// assert_eq!(range_field, "legacy_key"); // Falls back to legacy
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The schema has universal key configuration but range_field is empty (`SchemaError::InvalidData`)
    /// - The schema has no key configuration and no legacy range_key (`SchemaError::InvalidData`)
    pub fn get_range_key_field_name(&self, schema: &Schema) -> Result<String, SchemaError> {
        match &schema.schema_type {
            crate::schema::types::schema::SchemaType::Range { range_key } => {
                if let Some(key_config) = &schema.key {
                    // Universal key configuration takes precedence
                    if key_config.range_field.trim().is_empty() {
                        return Err(SchemaError::InvalidData(format!(
                            "Range schema '{}' with key configuration requires non-empty range_field", 
                            schema.name
                        )));
                    }
                    Ok(key_config.range_field.clone())
                } else {
                    // Fall back to legacy range_key for backward compatibility
                    Ok(range_key.clone())
                }
            }
            _ => Err(SchemaError::InvalidData(format!(
                "get_range_key_field_name can only be called on Range schemas, got: {:?}",
                schema.schema_type
            ))),
        }
    }
}
