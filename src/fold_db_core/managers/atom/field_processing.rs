//! Field value processing logic for AtomManager.
//!
//! The module exclusively relies on the schema-driven universal key helper to
//! derive hash/range metadata and normalized field payloads. All ad-hoc
//! heuristics have been removed in favor of descriptive error propagation.
//!
//! Reference documentation lives in
//! `docs/reference/fold_db_core/field_processing.md` and the
//! universal key workflow guide at
//! `docs/guides/operations/universal-key-migration-guide.md`.

use super::AtomManager;
use crate::atom::{Atom, AtomStatus};
use crate::fold_db_core::infrastructure::message_bus::{
    atom_events::{FieldValueSet, MutationContext},
    request_events::{FieldValueSetRequest, FieldValueSetResponse, KeySnapshot},
};
use crate::schema::schema_operations::{extract_unified_keys, shape_unified_result};
use crate::schema::SchemaError;
use log::{debug, error, info, warn};
use std::time::Instant;

/// Resolved key data structure produced by `resolve_universal_keys`.
#[derive(Debug, Clone)]
pub struct ResolvedAtomKeys {
    pub hash: Option<String>,
    pub range: Option<String>,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl ResolvedAtomKeys {
    /// Create a new ResolvedAtomKeys instance
    pub fn new(
        hash: Option<String>,
        range: Option<String>,
        fields: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            hash,
            range,
            fields,
        }
    }

    /// Get hash value as string (empty string if None)
    pub fn hash_str(&self) -> String {
        self.hash.clone().unwrap_or_default()
    }

    /// Get range value as string (empty string if None)
    pub fn range_str(&self) -> String {
        self.range.clone().unwrap_or_default()
    }

    /// Convert the resolved keys into a reusable snapshot structure
    pub fn to_snapshot(&self) -> KeySnapshot {
        KeySnapshot {
            hash: self.hash.clone(),
            range: self.range.clone(),
            fields: self.fields.clone(),
        }
    }
}

/// Resolve universal keys for field processing using schema-driven approach
///
/// This helper centralizes key extraction logic and provides a normalized
/// snapshot of hash, range, and fields data for any schema type. Errors are
/// surfaced directly when schema lookup or key extraction fails, removing the
/// need for ad-hoc heuristics or silent fallbacks. See the "Dotted-Path
/// Resolution" section in `docs/reference/fold_db_core/field_processing.md`
/// for guidance on dotted key expressions.
pub fn resolve_universal_keys(
    manager: &AtomManager,
    schema_name: &str,
    field_name: &str,
    request_payload: &serde_json::Value,
    mutation_context: Option<&crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
) -> Result<ResolvedAtomKeys, SchemaError> {
    debug!("🔑 Resolving universal keys for schema: {}, field: {}", schema_name, field_name);

    // Load schema from database
    let schema = manager
        .db_ops
        .get_schema(schema_name)
        .map_err(|e| {
            let error_msg = format!("Failed to load schema '{}': {}", schema_name, e);
            error!("❌ {}", error_msg);
            SchemaError::InvalidData(error_msg)
        })?
        .ok_or_else(|| {
            let error_msg = format!("Schema '{}' not found", schema_name);
            error!("❌ {}", error_msg);
            SchemaError::InvalidData(error_msg)
        })?;

    debug!(
        "📋 Schema '{}' loaded successfully, type: {:?}",
        schema_name, schema.schema_type
    );

    // Special handling for range key fields: if the field being processed IS the range key field,
    // then the field value IS the range key value
    let (hash_value, range_value) = if is_range_key_field(&schema, field_name) {
        debug!("🎯 Field '{}' is the range key field, using field value as range key", field_name);
        
        // For range key fields, the request_payload IS the range key value
        let range_key_value = request_payload.to_string().trim_matches('"').to_string();
        debug!("🎯 Range key value from field: '{}'", range_key_value);
        
        (None, Some(range_key_value))
    } else if let Some(context) = mutation_context {
        // Use mutation context if available (for non-range-key fields)
        debug!("🎯 Using mutation context for field '{}' - range: {:?}, hash: {:?}", 
               field_name, context.range_key, context.hash_key);
        (context.hash_key.clone(), context.range_key.clone())
    } else {
        // Extract hash and range keys using universal key configuration
        extract_unified_keys(&schema, request_payload).map_err(|e| {
            let error_msg = format!("Failed to extract keys for schema '{}': {}", schema_name, e);
            error!("❌ {}", error_msg);
            SchemaError::InvalidData(error_msg)
        })?
    };

    debug!(
        "🔑 Extracted keys - hash: {:?}, range: {:?}",
        hash_value, range_value
    );

    // Shape the result to get normalized fields structure
    let shaped_result = shape_unified_result(
        &schema,
        request_payload,
        hash_value.clone(),
        range_value.clone(),
    )
    .map_err(|e| {
        let error_msg = format!("Failed to shape result for schema '{}': {}", schema_name, e);
        error!("❌ {}", error_msg);
        SchemaError::InvalidData(error_msg)
    })?;

    // Extract fields from the shaped result
    let fields = if let Some(fields_obj) = shaped_result.get("fields").and_then(|v| v.as_object()) {
        fields_obj.clone()
    } else {
        // If no fields object, create empty map
        serde_json::Map::new()
    };

    debug!("✅ Successfully resolved keys for schema '{}'", schema_name);

    Ok(ResolvedAtomKeys::new(hash_value, range_value, fields))
}

/// Check if a field is the range key field for a schema
fn is_range_key_field(schema: &crate::schema::types::Schema, field_name: &str) -> bool {
    match &schema.schema_type {
        crate::schema::types::SchemaType::Range { range_key } => {
            // Check if this field is the range key field
            if let Some(key_config) = &schema.key {
                // Use universal key configuration if available
                !key_config.range_field.trim().is_empty() && key_config.range_field == field_name
            } else {
                // Legacy range_key support
                range_key == field_name
            }
        }
        crate::schema::types::SchemaType::HashRange => {
            // For HashRange schemas, check if this is the range field
            if let Some(key_config) = &schema.key {
                !key_config.range_field.trim().is_empty() && key_config.range_field == field_name
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Handle FieldValueSetRequest by creating atom and appropriate Molecule - CRITICAL MUTATION BUG FIX
pub(super) fn handle_fieldvalueset_request(
    manager: &AtomManager,
    request: FieldValueSetRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Processing FieldValueSetRequest (schema: {}, field: {}, correlation_id: {}, value: {})",
        request.schema_name, request.field_name, request.correlation_id, request.value
    );
    info!(
        "📝 Processing FieldValueSetRequest for field: {}.{}",
        request.schema_name, request.field_name
    );
    info!(
        "🔍 DIAGNOSTIC: FieldValueSetRequest details - correlation_id: {}, value: {}",
        request.correlation_id, request.value
    );

    update_processing_stats(manager);

    // Step 1: Create atom with the field value
    let atom_result = create_atom_for_field_value(manager, &request);

    let response = match atom_result {
        Ok(atom) => {
            let atom_uuid = atom.uuid().to_string();
            store_atom_in_cache(manager, atom.clone());

            // Step 2: Create appropriate Molecule based on field type
            let molecule_result = create_molecule_for_field(manager, &request, &atom_uuid);

            match molecule_result {
                Ok((molecule_uuid, resolved_keys)) => handle_successful_field_value_processing(
                    manager,
                    &request,
                    &atom_uuid,
                    &molecule_uuid,
                    &resolved_keys,
                ),
                Err(e) => {
                    update_failure_stats(manager);
                    create_molecule_error_response(&request.correlation_id, e)
                }
            }
        }
        Err(e) => {
            update_failure_stats(manager);
            create_atom_error_response(&request.correlation_id, e)
        }
    };

    // Publish response - Don't fail the operation if response publishing fails
    if let Err(e) = manager.message_bus.publish(response) {
        warn!(
            "⚠️ Failed to publish FieldValueSetResponse: {}. Operation completed successfully.",
            e
        );
    }
    Ok(())
}

/// Update processing stats for a new request
fn update_processing_stats(manager: &AtomManager) {
    let mut stats = manager.stats.lock().unwrap();
    stats.requests_processed += 1;
    stats.last_activity = Some(Instant::now());
}

/// Update failure stats
fn update_failure_stats(manager: &AtomManager) {
    let mut stats = manager.stats.lock().unwrap();
    stats.requests_failed += 1;
}

/// Create atom for field value
fn create_atom_for_field_value(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
) -> Result<Atom, Box<dyn std::error::Error>> {
    info!(
        "🔍 DIAGNOSTIC: Step 1 - Creating atom for schema: {}",
        request.schema_name
    );

    let atom_result = manager.db_ops.create_atom(
        &request.schema_name,
        request.source_pub_key.clone(),
        None, // No previous atom for field value sets
        request.value.clone(),
        Some(AtomStatus::Active),
    );

    match atom_result {
        Ok(atom) => {
            info!(
                "🔍 DIAGNOSTIC: Step 1 SUCCESS - Created atom with UUID: {}",
                atom.uuid()
            );
            Ok(atom)
        }
        Err(e) => Err(Box::new(e)),
    }
}

/// Store atom in memory cache
fn store_atom_in_cache(manager: &AtomManager, atom: Atom) {
    let atom_uuid = atom.uuid().to_string();
    manager.atoms.lock().unwrap().insert(atom_uuid, atom);
    info!("🔍 DIAGNOSTIC: Stored atom in memory cache");
}

/// Create appropriate Molecule for the field based on its type
fn create_molecule_for_field(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    atom_uuid: &str,
) -> Result<(String, ResolvedAtomKeys), Box<dyn std::error::Error>> {
    let field_type = determine_field_type(manager, &request.schema_name, &request.field_name);
    debug!(
        "Creating molecule for field {}.{} with type: {}",
        request.schema_name, request.field_name, field_type
    );
    info!(
        "🔍 DIAGNOSTIC: Step 2 - Determined field type: {}",
        field_type
    );

    // Resolve universal keys or surface descriptive error context
    let resolved_keys = resolve_universal_keys(manager, &request.schema_name, &request.field_name, &request.value, request.mutation_context.as_ref())
        .map_err(|e| {
            let error_msg = format!(
                "Failed to resolve keys for {}.{}: {}",
                request.schema_name, request.field_name, e
            );
            error!("❌ {}", error_msg);
            Box::new(SchemaError::InvalidData(error_msg)) as Box<dyn std::error::Error>
        })?;

    match field_type.as_str() {
        "Range" => {
            let molecule_uuid = create_range_molecule(manager, request, atom_uuid, &resolved_keys)?;
            Ok((molecule_uuid, resolved_keys))
        }
        "HashRange" => {
            let molecule_uuid =
                create_hashrange_molecule(manager, request, atom_uuid, &resolved_keys)?;
            Ok((molecule_uuid, resolved_keys))
        }
        _ => {
            let molecule_uuid =
                create_single_molecule(manager, request, atom_uuid, &resolved_keys)?;
            Ok((molecule_uuid, resolved_keys))
        }
    }
}

/// Create MoleculeRange for Range fields
fn create_range_molecule(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    atom_uuid: &str,
    resolved_keys: &ResolvedAtomKeys,
) -> Result<String, Box<dyn std::error::Error>> {
    // Use resolved range key instead of heuristic extraction
    let range_key = resolved_keys.range_str();
    let molecule_uuid = format!(
        "{}_{}_range_{}",
        request.schema_name, request.field_name, range_key
    );
    debug!(
        "Creating Range molecule with UUID: {} -> atom: {} (range_key: {})",
        molecule_uuid, atom_uuid, range_key
    );
    info!(
        "🔍 DIAGNOSTIC: Creating MoleculeRange with UUID: {} -> atom: {} (range_key: {})",
        molecule_uuid, atom_uuid, range_key
    );
    info!(
        "🔍 DIAGNOSTIC: Using resolved keys - hash: {:?}, range: {:?}",
        resolved_keys.hash, resolved_keys.range
    );

    let range_result = manager.db_ops.update_molecule_range(
        &molecule_uuid,
        atom_uuid.to_string(),
        range_key,
        request.source_pub_key.clone(),
    );

    match range_result {
        Ok(range) => {
            manager
                .molecule_ranges
                .lock()
                .unwrap()
                .insert(molecule_uuid.clone(), range);
            info!(
                "🔍 DIAGNOSTIC: Successfully created and stored MoleculeRange: {}",
                molecule_uuid
            );

            // Verify the MoleculeRange was properly stored in database
            match manager
                .db_ops
                .get_item::<crate::atom::MoleculeRange>(&format!("ref:{}", molecule_uuid))
            {
                Ok(Some(_)) => {
                    info!(
                        "✅ VERIFICATION: MoleculeRange {} confirmed in database",
                        molecule_uuid
                    );
                }
                Ok(None) => {
                    error!("❌ VERIFICATION FAILED: MoleculeRange {} not found in database after storage", molecule_uuid);
                }
                Err(e) => {
                    error!(
                        "❌ VERIFICATION ERROR: Failed to verify MoleculeRange {}: {}",
                        molecule_uuid, e
                    );
                }
            }

            Ok(molecule_uuid)
        }
        Err(e) => {
            error!("❌ DIAGNOSTIC: Failed to create MoleculeRange: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Create MoleculeHashRange for HashRange fields
fn create_hashrange_molecule(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    atom_uuid: &str,
    resolved_keys: &ResolvedAtomKeys,
) -> Result<String, Box<dyn std::error::Error>> {
    let molecule_uuid = format!("{}_{}_hashrange", request.schema_name, request.field_name);
    debug!(
        "Creating HashRange molecule with UUID: {} -> atom: {}",
        molecule_uuid, atom_uuid
    );
    info!(
        "🔍 DIAGNOSTIC: Creating MoleculeHashRange with UUID: {} -> atom: {}",
        molecule_uuid, atom_uuid
    );

    let hash_value = resolved_keys.hash.clone().ok_or_else(|| {
        let error_msg = format!(
            "Missing hash key for HashRange field {}.{}",
            request.schema_name, request.field_name
        );
        error!("❌ {}", error_msg);
        Box::new(SchemaError::InvalidData(error_msg)) as Box<dyn std::error::Error>
    })?;

    let range_value = resolved_keys.range.clone().ok_or_else(|| {
        let error_msg = format!(
            "Missing range key for HashRange field {}.{}",
            request.schema_name, request.field_name
        );
        error!("❌ {}", error_msg);
        Box::new(SchemaError::InvalidData(error_msg)) as Box<dyn std::error::Error>
    })?;

    info!(
        "🔍 DIAGNOSTIC: HashRange resolved values - hash: '{}' and range: '{}' from atom: {}",
        hash_value, range_value, atom_uuid
    );

    let storage_key = format!(
        "{}_{}_{}",
        request.schema_name, request.field_name, hash_value
    );

    let mut existing_btree = match manager
        .db_ops
        .get_item::<serde_json::Map<String, serde_json::Value>>(&storage_key)
    {
        Ok(Some(data)) => data,
        Ok(None) => serde_json::Map::new(),
        Err(e) => {
            let error_msg = format!(
                "Failed to load HashRange map for key {}: {}",
                storage_key, e
            );
            error!("❌ {}", error_msg);
            return Err(Box::new(SchemaError::InvalidData(error_msg)) as Box<dyn std::error::Error>);
        }
    };

    let snapshot = resolved_keys.to_snapshot();
    let snapshot_value =
        serde_json::to_value(&snapshot).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    existing_btree.insert(range_value.clone(), snapshot_value);

    manager
        .db_ops
        .store_item(&storage_key, &existing_btree)
        .map_err(|e| {
            error!("❌ DIAGNOSTIC: Failed to store HashRange data: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

    info!(
        "🔍 DIAGNOSTIC: Successfully stored HashRange snapshot for key: {} (range: {})",
        storage_key, range_value
    );

    Ok(storage_key)
}

/// Create Molecule for Single fields (default)
fn create_single_molecule(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    atom_uuid: &str,
    resolved_keys: &ResolvedAtomKeys,
) -> Result<String, Box<dyn std::error::Error>> {
    let molecule_uuid = format!("{}_{}_single", request.schema_name, request.field_name);
    info!(
        "🔍 DIAGNOSTIC: Creating Molecule (Single) with UUID: {} -> atom: {}",
        molecule_uuid, atom_uuid
    );
    info!(
        "🔍 DIAGNOSTIC: Using resolved keys - hash: {:?}, range: {:?}",
        resolved_keys.hash, resolved_keys.range
    );

    let single_result = manager.db_ops.update_molecule(
        &molecule_uuid,
        atom_uuid.to_string(),
        request.source_pub_key.clone(),
    );

    match single_result {
        Ok(molecule) => {
            info!(
                "🔍 DIAGNOSTIC: Molecule created successfully, final atom_uuid: {}",
                molecule.get_atom_uuid()
            );
            manager
                .molecules
                .lock()
                .unwrap()
                .insert(molecule_uuid.clone(), molecule);
            info!(
                "🔍 DIAGNOSTIC: Successfully created and stored Molecule: {}",
                molecule_uuid
            );

            // Verify the Molecule was properly stored in database
            match manager
                .db_ops
                .get_item::<crate::atom::Molecule>(&format!("ref:{}", molecule_uuid))
            {
                Ok(Some(_)) => {
                    info!(
                        "✅ VERIFICATION: Molecule {} confirmed in database",
                        molecule_uuid
                    );
                }
                Ok(None) => {
                    error!(
                        "❌ VERIFICATION FAILED: Molecule {} not found in database after storage",
                        molecule_uuid
                    );
                }
                Err(e) => {
                    error!(
                        "❌ VERIFICATION ERROR: Failed to verify Molecule {}: {}",
                        molecule_uuid, e
                    );
                }
            }

            Ok(molecule_uuid)
        }
        Err(e) => {
            error!("❌ DIAGNOSTIC: Failed to create Molecule: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Handle successful field value processing
fn handle_successful_field_value_processing(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    atom_uuid: &str,
    molecule_uuid: &str,
    resolved_keys: &ResolvedAtomKeys,
) -> FieldValueSetResponse {
    let mut stats = manager.stats.lock().unwrap();
    stats.atoms_created += 1;
    stats.molecules_created += 1;
    drop(stats);

    info!(
        "✅ Successfully processed FieldValueSetRequest - atom: {}, molecule: {}",
        atom_uuid, molecule_uuid
    );
    info!(
        "🔍 DIAGNOSTIC: Final mapping - Molecule {} -> Atom {}",
        molecule_uuid, atom_uuid
    );
    info!(
        "🔍 DIAGNOSTIC: Key snapshot - hash: {:?}, range: {:?}, fields: {:?}",
        resolved_keys.hash,
        resolved_keys.range,
        resolved_keys.fields.keys()
    );

    // Publish FieldValueSet event to trigger transform chain
    publish_field_value_set_event(manager, request, resolved_keys);

    // Fire DataPersisted event to signal that data is now queryable
    let data_persisted =
        crate::fold_db_core::infrastructure::message_bus::events::schema_events::DataPersisted::new(
            request.schema_name.clone(),
            request.correlation_id.clone(),
        );
    if let Err(e) = manager.message_bus.publish(data_persisted) {
        warn!("⚠️ Failed to publish DataPersisted event: {}", e);
    } else {
        info!(
            "📊 DataPersisted event fired for schema '{}' with correlation_id '{}'",
            request.schema_name, request.correlation_id
        );
    }

    // Create key snapshot for response
    let key_snapshot = resolved_keys.to_snapshot();

    FieldValueSetResponse::with_key_snapshot(
        request.correlation_id.clone(),
        true,
        Some(molecule_uuid.to_string()),
        None,
        Some(key_snapshot),
    )
}

/// Publish FieldValueSet event to trigger transform chain
fn publish_field_value_set_event(
    manager: &AtomManager,
    request: &FieldValueSetRequest,
    resolved_keys: &ResolvedAtomKeys,
) {
    let field_key = format!("{}.{}", request.schema_name, request.field_name);
    let snapshot = resolved_keys.to_snapshot();
    let normalized_context = match request.mutation_context.clone() {
        Some(mut context) => {
            context.hash_key = snapshot.hash.clone();
            context.range_key = snapshot.range.clone();
            Some(context)
        }
        None => {
            if snapshot.hash.is_some() || snapshot.range.is_some() {
                Some(MutationContext {
                    range_key: snapshot.range.clone(),
                    hash_key: snapshot.hash.clone(),
                    mutation_hash: None,
                    incremental: false,
                })
            } else {
                None
            }
        }
    };

    let field_value_event = if let Some(ref context) = normalized_context {
        FieldValueSet::with_context_and_keys(
            field_key.clone(),
            request.value.clone(),
            "AtomManager".to_string(),
            context.clone(),
            snapshot.clone(),
        )
    } else {
        FieldValueSet::with_keys(
            field_key.clone(),
            request.value.clone(),
            "AtomManager".to_string(),
            snapshot.clone(),
        )
    };

    info!(
        "🔔 DIAGNOSTIC FIX: Publishing FieldValueSet event - field: {}, source: AtomManager",
        field_key
    );
    if let Some(ref context) = normalized_context {
        info!(
            "🔔 DIAGNOSTIC FIX: FieldValueSet event includes mutation context - range_key: {:?}, hash_key: {:?}, incremental: {}",
            context.range_key,
            context.hash_key,
            context.incremental
        );
    }

    match manager.message_bus.publish(field_value_event) {
        Ok(_) => {
            info!(
                "✅ DIAGNOSTIC FIX: Successfully published FieldValueSet event for: {}",
                field_key
            );
        }
        Err(e) => {
            error!(
                "❌ DIAGNOSTIC FIX: Failed to publish FieldValueSet event for {}: {}",
                field_key, e
            );
            // Continue processing even if event publication fails
        }
    }
}

/// Create error response for Molecule creation failure
fn create_molecule_error_response(
    correlation_id: &str,
    error: Box<dyn std::error::Error>,
) -> FieldValueSetResponse {
    error!(
        "❌ Failed to create Molecule for FieldValueSetRequest: {}",
        error
    );
    FieldValueSetResponse::new(
        correlation_id.to_string(),
        false,
        None,
        Some(format!("Failed to create Molecule: {}", error)),
    )
}

/// Create error response for Atom creation failure
fn create_atom_error_response(
    correlation_id: &str,
    error: Box<dyn std::error::Error>,
) -> FieldValueSetResponse {
    error!(
        "❌ Failed to create Atom for FieldValueSetRequest: {}",
        error
    );
    FieldValueSetResponse::new(
        correlation_id.to_string(),
        false,
        None,
        Some(format!("Failed to create Atom: {}", error)),
    )
}

/// Determine field type based on schema and field name
fn determine_field_type(manager: &AtomManager, schema_name: &str, field_name: &str) -> String {
    debug!("Determining field type for {}.{}", schema_name, field_name);
    // Look up the actual schema to determine field type
    match manager.db_ops.get_schema(schema_name) {
        Ok(Some(schema)) => {
            let field_names: Vec<&String> = schema.fields.keys().collect();
            debug!(
                "Schema '{}' loaded successfully with fields: {:?}",
                schema_name, field_names
            );
            match schema.fields.get(field_name) {
                Some(field_variant) => {
                    debug!(
                        "Resolved field variant for {}.{}: {:?}",
                        schema_name, field_name, field_variant
                    );
                    match field_variant {
                        crate::schema::types::field::FieldVariant::Range(_) => {
                            info!(
                                "🔍 FIELD TYPE: {} in schema {} is Range",
                                field_name, schema_name
                            );
                            "Range".to_string()
                        }
                        crate::schema::types::field::FieldVariant::Single(_) => {
                            info!(
                                "🔍 FIELD TYPE: {} in schema {} is Single",
                                field_name, schema_name
                            );
                            "Single".to_string()
                        }
                        crate::schema::types::field::FieldVariant::HashRange(_) => {
                            info!(
                                "🔍 FIELD TYPE: {} in schema {} is HashRange",
                                field_name, schema_name
                            );
                            "HashRange".to_string()
                        }
                    }
                }
                None => {
                    debug!(
                        "Field {} not found in schema {}. Defaulting to Single handling.",
                        field_name, schema_name
                    );
                    warn!(
                        "⚠️ FIELD TYPE: Field {} not found in schema {}, defaulting to Single",
                        field_name, schema_name
                    );
                    "Single".to_string()
                }
            }
        }
        Ok(None) => {
            warn!(
                "⚠️ FIELD TYPE: Schema {} not found, defaulting to Single",
                schema_name
            );
            "Single".to_string()
        }
        Err(e) => {
            error!(
                "❌ FIELD TYPE: Error loading schema {}: {}, defaulting to Single",
                schema_name, e
            );
            "Single".to_string()
        }
    }
}
