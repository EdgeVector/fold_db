use crate::schema::types::{Schema, SchemaError, KeyConfig};
use log::info;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

// Constants for repeated values following DRY principle
const DEFAULT_RANGE_KEY: &str = "default";
const MOLECULE_PREFIX: &str = "ref:";
const ATOM_PREFIX: &str = "atom:";

/// Handles fetching data for different schema types
pub struct SchemaDataFetcher;

impl SchemaDataFetcher {
    /// Fetch schema data for a specific range key only
    pub fn fetch_schema_data_for_range_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🎯 TransformManager: Fetching data for range_key '{}' in schema '{}'",
            range_key, schema_name
        );

        // Get the schema definition
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
        })?;

        // Get all field names from the schema
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();
        println!(
            "🔍 TransformManager: Schema '{}' has fields: {:?}",
            schema_name, field_names
        );

        // Create a single record for the specific range key
        let mut record: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        for field_name in field_names {
            Self::process_field_for_range_key(
                db_ops,
                schema_name,
                &field_name,
                range_key,
                &mut record,
            )?;
        }

        // Return the single record as an array (to maintain compatibility with existing transform logic)
        let result = serde_json::json!([record]);
        println!(
            "✅ TransformManager: Returning incremental data for range_key '{}': {}",
            range_key, result
        );
        Ok(result)
    }

    /// Fetch schema data for a specific hash_key and range_key combination
    pub fn fetch_schema_data_for_hashrange_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        hash_key: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        println!("🎯 TransformManager: Fetching data for hash_key '{}' and range_key '{}' in schema '{}'", hash_key, range_key, schema_name);

        // Get the schema definition
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
        })?;

        // Get all field names from the schema
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();
        println!(
            "🔍 TransformManager: Schema '{}' has fields: {:?}",
            schema_name, field_names
        );

        // Create a single record for the specific hash_key and range_key combination
        let mut record: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        for field_name in field_names {
            Self::process_field_for_hashrange_key(
                db_ops,
                schema_name,
                &field_name,
                hash_key,
                range_key,
                &mut record,
            )?;
        }

        // Return the single record as an array (to maintain compatibility with existing transform logic)
        let result = serde_json::json!([record]);
        println!("✅ TransformManager: Returning incremental data for hash_key '{}' and range_key '{}': {}", hash_key, range_key, result);
        Ok(result)
    }

    /// Fetch data for Range schemas
    pub fn fetch_range_schema_data(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        schema_name: &str,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        info!(
            "🔍 TransformManager: Processing Range schema '{}'",
            schema_name
        );

        // Get all field names from the schema definition
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();
        println!(
            "🔍 TransformManager: Schema '{}' has fields: {:?}",
            schema_name, field_names
        );

        // For each field, get the MoleculeRange and collect atoms
        let mut records_by_range_key: std::collections::HashMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        > = std::collections::HashMap::new();

        for field_name in field_names {
            Self::process_range_field(db_ops, schema_name, &field_name, &mut records_by_range_key)?;
        }

        // Convert grouped data into array format
        let schema_array =
            Self::convert_range_records_to_array(schema, schema_name, records_by_range_key)?;

        info!(
            "🔍 TransformManager: Found {} records for schema '{}'",
            schema_array.len(),
            schema_name
        );
        Ok(schema_array)
    }

    /// Fetch data for HashRange schemas
    pub fn fetch_hashrange_schema_data(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        schema_name: &str,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        info!(
            "🔍 TransformManager: Processing HashRange schema '{}'",
            schema_name
        );

        // Get all field names from the schema definition
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();
        println!(
            "🔍 TransformManager: Schema '{}' has fields: {:?}",
            schema_name, field_names
        );

        // For HashRange schemas, we need to collect all hash_key and range_key combinations
        let mut records_by_keys: std::collections::HashMap<
            (String, String),
            serde_json::Map<String, serde_json::Value>,
        > = std::collections::HashMap::new();

        for field_name in field_names {
            Self::process_hashrange_field(db_ops, schema_name, &field_name, &mut records_by_keys)?;
        }

        // Convert grouped data into array format
        let schema_array = Self::convert_hashrange_records_to_array(schema_name, records_by_keys)?;

        info!(
            "🔍 TransformManager: Found {} records for HashRange schema '{}'",
            schema_array.len(),
            schema_name
        );
        Ok(schema_array)
    }

    /// Fetch data for simple (non-range, non-hashrange) schemas
    pub fn fetch_simple_schema_data(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        schema_name: &str,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        info!(
            "🔍 TransformManager: Processing simple schema '{}'",
            schema_name
        );

        // Get all field names from the schema
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();

        // For simple schemas, use the original approach
        let mut schema_data = serde_json::Map::new();

        for field_name in &field_names {
            match Self::get_field_value_from_schema(db_ops, schema, field_name) {
                Ok(value) => {
                    schema_data.insert(field_name.clone(), value);
                }
                Err(e) => {
                    info!(
                        "⚠️ TransformManager: Failed to get field '{}' for schema '{}': {}",
                        field_name, schema_name, e
                    );
                    schema_data.insert(field_name.clone(), JsonValue::Null);
                }
            }
        }

        // Format the data as expected by declarative transforms
        let schema_array = Self::format_simple_schema_data(schema_data)?;

        info!(
            "🔍 TransformManager: Found {} records for simple schema '{}'",
            schema_array.len(),
            schema_name
        );
        Ok(schema_array)
    }

    /// Process a field for range key fetching
    fn process_field_for_range_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        range_key: &str,
        record: &mut serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        println!(
            "🔍 TransformManager: Looking for Range molecule for field '{}' with range_key '{}'",
            field_name, range_key
        );

        // Look for the specific Range molecule: {schema_name}_{field_name}_range_{range_key}
        let molecule_key = format!("ref:{}_{}_range_{}", schema_name, field_name, range_key);
        println!(
            "🔍 TransformManager: Looking for molecule: {}",
            molecule_key
        );

        match db_ops.get_item::<crate::atom::MoleculeRange>(&molecule_key) {
            Ok(Some(range_molecule)) => {
                println!(
                    "✅ Found MoleculeRange for field '{}' with range_key '{}'",
                    field_name, range_key
                );

                // Process each atom in the range
                for (molecule_range_key, atom_uuid) in &range_molecule.atom_uuids {
                    if molecule_range_key == range_key {
                        Self::process_atom_for_field(db_ops, atom_uuid, field_name, record)?;
                    }
                }
            }
            Ok(None) => {
                println!(
                    "⚠️ MoleculeRange '{}' not found for field '{}'",
                    molecule_key, field_name
                );
            }
            Err(e) => {
                println!(
                    "❌ Error loading MoleculeRange '{}' for field '{}': {}",
                    molecule_key, field_name, e
                );
            }
        }

        Ok(())
    }

    /// Process a field for hashrange key fetching
    fn process_field_for_hashrange_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        hash_key: &str,
        range_key: &str,
        record: &mut serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        println!("🔍 TransformManager: Looking for HashRange molecule for field '{}' with hash_key '{}' and range_key '{}'", field_name, hash_key, range_key);

        // Look for the specific HashRange molecule: {schema_name}_{field_name}_hashrange_{hash_key}_{range_key}
        let molecule_key = format!(
            "ref:{}_{}_hashrange_{}_{}",
            schema_name, field_name, hash_key, range_key
        );
        println!(
            "🔍 TransformManager: Looking for molecule: {}",
            molecule_key
        );

        match db_ops.get_item::<crate::atom::MoleculeHashRange>(&molecule_key) {
            Ok(Some(hashrange_molecule)) => {
                println!("✅ Found MoleculeHashRange for field '{}' with hash_key '{}' and range_key '{}'", field_name, hash_key, range_key);

                // Check if the hash and range values match
                if hashrange_molecule.hash_value == hash_key
                    && hashrange_molecule.range_value == range_key
                {
                    // Process each atom in the hashrange
                    for atom_uuid in &hashrange_molecule.atom_uuids {
                        Self::process_atom_for_field(db_ops, atom_uuid, field_name, record)?;
                    }
                } else {
                    println!("⚠️ MoleculeHashRange hash/range values don't match - expected hash_key '{}' range_key '{}', got hash_value '{}' range_value '{}'", 
                             hash_key, range_key, hashrange_molecule.hash_value, hashrange_molecule.range_value);
                }
            }
            Ok(None) => {
                println!(
                    "⚠️ MoleculeHashRange '{}' not found for field '{}'",
                    molecule_key, field_name
                );
            }
            Err(e) => {
                println!(
                    "❌ Error loading MoleculeHashRange '{}' for field '{}': {}",
                    molecule_key, field_name, e
                );
            }
        }

        Ok(())
    }

    /// Process an atom and extract its value for a field
    fn process_atom_for_field(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        atom_uuid: &str,
        field_name: &str,
        record: &mut serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        // Load the atom and extract its value
        match db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid)) {
            Ok(Some(atom)) => {
                let content = atom.content();
                println!(
                    "🔍 TransformManager: Atom {} content: {}",
                    atom_uuid, content
                );

                // Extract the value from the atom content
                // The atom content structure is: {"fields": {"field_name": value, ...}, "hash": "...", "range": "..."}
                if let Some(fields) = content.get("fields").and_then(|f| f.as_object()) {
                    if let Some(value) = fields.get(field_name) {
                        record.insert(field_name.to_string(), value.clone());
                        println!(
                            "🔍 TransformManager: Added field '{}' = {} to record",
                            field_name, value
                        );
                    } else {
                        println!("⚠️ Field '{}' not found in atom fields", field_name);
                    }
                } else {
                    println!("⚠️ No 'fields' object found in atom content");
                }
            }
            Ok(None) => {
                println!("⚠️ Atom {} not found for field '{}'", atom_uuid, field_name);
            }
            Err(e) => {
                println!(
                    "❌ Error loading atom {} for field '{}': {}",
                    atom_uuid, field_name, e
                );
            }
        }

        Ok(())
    }

    /// Process a range field and collect atoms
    fn process_range_field(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        records_by_range_key: &mut std::collections::HashMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<(), SchemaError> {
        println!(
            "🔍 TransformManager: Looking for Range molecules for field '{}'",
            field_name
        );

        let mut found_data = false;

        // Look for all Range molecules that match the pattern: {schema_name}_{field_name}_range_{range_key}
        // Use prefix scanning for better performance
        let range_prefix = format!("{}{}_{}_range_", MOLECULE_PREFIX, schema_name, field_name);
        let range_keys = db_ops.list_items_with_prefix(&range_prefix)?;

        for key_str in range_keys {
            Self::process_range_molecule(
                db_ops,
                schema_name,
                field_name,
                &key_str,
                records_by_range_key,
                &mut found_data,
            )?;
        }

        // If no Range molecules found, try the old format for backward compatibility
        if !found_data {
            Self::try_legacy_range_format(db_ops, schema_name, field_name, records_by_range_key)?;
        }

        if !found_data {
            println!(
                "⚠️ No data found for field '{}' in schema '{}'",
                field_name, schema_name
            );
        }

        Ok(())
    }

    /// Process a single range molecule
    fn process_range_molecule(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        key_str: &str,
        records_by_range_key: &mut std::collections::HashMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        >,
        found_data: &mut bool,
    ) -> Result<(), SchemaError> {
        println!("🔍 TransformManager: Found Range molecule: {}", key_str);

        // Extract the range key from the molecule UUID
        let molecule_uuid = key_str.strip_prefix(MOLECULE_PREFIX).unwrap();
        let range_key = molecule_uuid
            .strip_prefix(&format!("{}_{}_range_", schema_name, field_name))
            .unwrap();

        println!(
            "🔍 TransformManager: Processing Range molecule with range_key: {}",
            range_key
        );

        // Load the MoleculeRange
        match db_ops.get_item::<crate::atom::MoleculeRange>(key_str) {
            Ok(Some(range_molecule)) => {
                println!(
                    "✅ Found MoleculeRange for field '{}' with range_key '{}'",
                    field_name, range_key
                );
                *found_data = true;

                // Process each atom in the range
                for (molecule_range_key, atom_uuid) in &range_molecule.atom_uuids {
                    let record = records_by_range_key
                        .entry(molecule_range_key.clone())
                        .or_default();

                    // Load the atom and extract its value
                    match db_ops
                        .get_item::<crate::atom::Atom>(&format!("{}{}", ATOM_PREFIX, atom_uuid))
                    {
                        Ok(Some(atom)) => {
                            let content = atom.content();
                            println!(
                                "🔍 TransformManager: Atom {} content: {}",
                                atom_uuid, content
                            );

                            // Extract the value from the atom content
                            // The atom content structure is: {"fields": {"field_name": value, "range_key": "...", ...}, "hash": "...", "range": "..."}
                            if let Some(fields) = content.get("fields").and_then(|f| f.as_object()) {
                                if let Some(value) = fields.get(field_name) {
                                    record.insert(field_name.to_string(), value.clone());
                                    println!("🔍 TransformManager: Added field '{}' = {} to record with range_key '{}'", field_name, value, molecule_range_key);
                                } else {
                                    println!("⚠️ Field '{}' not found in atom fields for range_key '{}'", field_name, molecule_range_key);
                                }
                            } else {
                                println!("⚠️ No 'fields' object found in atom content for range_key '{}'", molecule_range_key);
                            }
                        }
                        Ok(None) => {
                            println!("⚠️ Atom {} not found for field '{}'", atom_uuid, field_name);
                        }
                        Err(e) => {
                            println!(
                                "❌ Error loading atom {} for field '{}': {}",
                                atom_uuid, field_name, e
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                println!(
                    "⚠️ MoleculeRange '{}' not found for field '{}'",
                    key_str, field_name
                );
            }
            Err(e) => {
                println!(
                    "❌ Error loading MoleculeRange '{}' for field '{}': {}",
                    key_str, field_name, e
                );
            }
        }

        Ok(())
    }

    /// Try legacy range format for backward compatibility
    fn try_legacy_range_format(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        records_by_range_key: &mut std::collections::HashMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<(), SchemaError> {
        let range_molecule_uuid = format!("{}_{}_range", schema_name, field_name);
        let single_molecule_uuid = format!("{}_{}_single", schema_name, field_name);

        println!(
            "🔍 TransformManager: No Range molecules found, trying legacy format: {}",
            range_molecule_uuid
        );
        println!(
            "🔍 TransformManager: Also checking Molecule: {}",
            single_molecule_uuid
        );

        match db_ops.get_item::<crate::atom::Molecule>(&format!(
            "{}{}",
            MOLECULE_PREFIX, single_molecule_uuid
        )) {
            Ok(Some(single_molecule)) => {
                let atom_uuid = single_molecule.get_atom_uuid();
                println!(
                    "✅ Found Molecule (Single) for field '{}' with atom: {}",
                    field_name, atom_uuid
                );

                // Load the atom and extract its value
                match db_ops.get_item::<crate::atom::Atom>(&format!("{}{}", ATOM_PREFIX, atom_uuid))
                {
                    Ok(Some(atom)) => {
                        let content = atom.content();
                        println!(
                            "🔍 TransformManager: Atom {} content: {}",
                            atom_uuid, content
                        );

                        // Extract the value from the atom content
                        // The atom content structure is: {"fields": {"field_name": value, ...}, "hash": "...", "range": "..."}
                        if let Some(fields) = content.get("fields").and_then(|f| f.as_object()) {
                            if let Some(value) = fields.get(field_name) {
                                // For single molecules, use a default range key
                                let record = records_by_range_key
                                    .entry(DEFAULT_RANGE_KEY.to_string())
                                    .or_default();
                                record.insert(field_name.to_string(), value.clone());
                                println!("🔍 TransformManager: Added field '{}' = {} to record with default range_key", field_name, value);
                            } else {
                                println!("⚠️ Field '{}' not found in atom fields for single molecule", field_name);
                            }
                        } else {
                            println!("⚠️ No 'fields' object found in atom content for single molecule");
                        }
                    }
                    Ok(None) => {
                        println!("⚠️ Atom {} not found for field '{}'", atom_uuid, field_name);
                    }
                    Err(e) => {
                        println!(
                            "❌ Error loading atom {} for field '{}': {}",
                            atom_uuid, field_name, e
                        );
                    }
                }
            }
            Ok(None) => {
                println!(
                    "⚠️ Molecule '{}' not found for field '{}'",
                    single_molecule_uuid, field_name
                );
            }
            Err(e) => {
                println!(
                    "❌ Error loading Molecule '{}' for field '{}': {}",
                    single_molecule_uuid, field_name, e
                );
            }
        }

        Ok(())
    }

    /// Process a hashrange field and collect atoms
    fn process_hashrange_field(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        records_by_keys: &mut std::collections::HashMap<
            (String, String),
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<(), SchemaError> {
        println!(
            "🔍 TransformManager: Looking for HashRange molecules for field '{}'",
            field_name
        );

        // Look for all HashRange molecules that match the pattern: {schema_name}_{field_name}_hashrange_{hash_key}_{range_key}
        // Use prefix scanning for better performance
        let hashrange_prefix = format!(
            "{}{}_{}_hashrange_",
            MOLECULE_PREFIX, schema_name, field_name
        );
        let hashrange_keys = db_ops.list_items_with_prefix(&hashrange_prefix)?;

        for key_str in hashrange_keys {
            Self::process_hashrange_molecule(
                db_ops,
                schema_name,
                field_name,
                &key_str,
                records_by_keys,
            )?;
        }

        Ok(())
    }

    /// Process a single hashrange molecule
    fn process_hashrange_molecule(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        key_str: &str,
        records_by_keys: &mut std::collections::HashMap<
            (String, String),
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<(), SchemaError> {
        println!("🔍 TransformManager: Found HashRange molecule: {}", key_str);

        // Extract the hash_key and range_key from the molecule UUID
        let molecule_uuid = key_str.strip_prefix(MOLECULE_PREFIX).unwrap();
        let prefix = format!("{}_{}_hashrange_", schema_name, field_name);

        if let Some(remaining) = molecule_uuid.strip_prefix(&prefix) {
            // Split on the last underscore to separate hash_key and range_key
            if let Some(last_underscore_pos) = remaining.rfind('_') {
                let hash_key = &remaining[..last_underscore_pos];
                let range_key = &remaining[last_underscore_pos + 1..];

                println!("🔍 TransformManager: Processing HashRange molecule with hash_key '{}' and range_key '{}'", hash_key, range_key);

                // Load the MoleculeHashRange
                match db_ops.get_item::<crate::atom::MoleculeHashRange>(key_str) {
                    Ok(Some(hashrange_molecule)) => {
                        println!("✅ Found MoleculeHashRange for field '{}' with hash_key '{}' and range_key '{}'", field_name, hash_key, range_key);

                        // Check if the hash and range values match
                        if hashrange_molecule.hash_value == hash_key
                            && hashrange_molecule.range_value == range_key
                        {
                            let key_tuple = (hash_key.to_string(), range_key.to_string());
                            let record = records_by_keys.entry(key_tuple).or_default();

                            // Process each atom in the hashrange
                            for atom_uuid in &hashrange_molecule.atom_uuids {
                                Self::process_atom_for_field(
                                    db_ops, atom_uuid, field_name, record,
                                )?;
                            }
                        } else {
                            println!("⚠️ MoleculeHashRange hash/range values don't match - expected hash_key '{}' range_key '{}', got hash_value '{}' range_value '{}'", 
                                     hash_key, range_key, hashrange_molecule.hash_value, hashrange_molecule.range_value);
                        }
                    }
                    Ok(None) => {
                        println!(
                            "⚠️ MoleculeHashRange '{}' not found for field '{}'",
                            key_str, field_name
                        );
                    }
                    Err(e) => {
                        println!(
                            "❌ Error loading MoleculeHashRange '{}' for field '{}': {}",
                            key_str, field_name, e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert range records to array format
    fn convert_range_records_to_array(
        schema: &Schema,
        schema_name: &str,
        records_by_range_key: std::collections::HashMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        // Get the actual range_key field name from the schema
        let range_key_field_name = schema
            .range_key()
            .ok_or_else(|| {
                SchemaError::InvalidData(format!("Schema '{}' is not a Range schema", schema_name))
            })?
            .to_string();

        let mut schema_array = Vec::new();
        for (range_key_value, record_data) in records_by_range_key {
            let mut schema_item = serde_json::Map::new();
            schema_item.insert(range_key_field_name.clone(), json!(range_key_value));

            // Add all field values to the schema item
            for (field_name, field_value) in record_data {
                schema_item.insert(field_name, field_value);
            }

            schema_array.push(json!(schema_item));
        }

        Ok(schema_array)
    }

    /// Convert hashrange records to array format
    fn convert_hashrange_records_to_array(
        schema_name: &str,
        records_by_keys: std::collections::HashMap<
            (String, String),
            serde_json::Map<String, serde_json::Value>,
        >,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        // Get the actual hash_key and range_key field names from the schema
        // For HashRange schemas, we need to get the key configuration from the JSON schema file
        let key_config =
            Self::get_universal_key_config_from_json(schema_name)?.ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "No key configuration found for HashRange schema '{}'",
                    schema_name
                ))
            })?;

        let hash_key_field_name = key_config.hash_field;
        let range_key_field_name = key_config.range_field;

        let mut schema_array = Vec::new();
        for ((hash_key_value, range_key_value), record_data) in records_by_keys {
            let mut schema_item = serde_json::Map::new();
            schema_item.insert(hash_key_field_name.clone(), json!(hash_key_value));
            schema_item.insert(range_key_field_name.clone(), json!(range_key_value));

            // Add all field values to the schema item
            for (field_name, field_value) in record_data {
                schema_item.insert(field_name, field_value);
            }

            schema_array.push(json!(schema_item));
        }

        Ok(schema_array)
    }

    /// Format simple schema data
    fn format_simple_schema_data(
        schema_data: serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        // Format the data as expected by declarative transforms
        let mut schema_array = Vec::new();

        // Check if we have range-like data structure
        let has_range_structure = schema_data.values().any(|v| v.is_object());

        if has_range_structure {
            // Get the range keys (timestamps for range schemas)
            let range_keys: Vec<String> = schema_data
                .values()
                .filter_map(|v| v.as_object())
                .flat_map(|obj| obj.keys())
                .cloned()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            info!("🔍 TransformManager: Found range keys: {:?}", range_keys);

            for range_key in range_keys {
                let mut schema_item = serde_json::Map::new();

                // Add all field values for this range key
                for (field_name, field_data) in &schema_data {
                    if let Some(field_obj) = field_data.as_object() {
                        if let Some(value) = field_obj.get(&range_key) {
                            schema_item.insert(field_name.clone(), value.clone());
                        }
                    }
                }

                schema_array.push(json!(schema_item));
            }
        }

        // If no range keys found, create a single record with all field values
        if schema_array.is_empty() {
            let mut schema_item = serde_json::Map::new();
            for (field_name, field_value) in &schema_data {
                schema_item.insert(field_name.clone(), field_value.clone());
            }
            schema_array.push(json!(schema_item));
        }

        Ok(schema_array)
    }

    /// Get universal key configuration from the original JSON schema file
    ///
    /// This is a static version of the unified key config reader that can be used
    /// without requiring a SchemaCore instance.
    fn get_universal_key_config_from_json(
        schema_name: &str,
    ) -> Result<Option<KeyConfig>, SchemaError> {
        use crate::schema::constants::{
            KEY_CONFIG_HASH_FIELD, KEY_CONFIG_RANGE_FIELD, KEY_FIELD_NAME,
        };
        use KeyConfig;
        use serde_json::Value;

        let schema_file_path = format!("available_schemas/{}.json", schema_name);
        info!("🔍 Reading universal key config from: {}", schema_file_path);

        let content = std::fs::read_to_string(&schema_file_path).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to read schema file {}: {}",
                schema_file_path, e
            ))
        })?;

        let json_value: Value = serde_json::from_str(&content).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to parse JSON from {}: {}",
                schema_file_path, e
            ))
        })?;

        if let Some(key_obj) = json_value.get(KEY_FIELD_NAME).and_then(|v| v.as_object()) {
            let hash_field = key_obj
                .get(KEY_CONFIG_HASH_FIELD)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let range_field = key_obj
                .get(KEY_CONFIG_RANGE_FIELD)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            info!(
                "🔑 Found key config - hash_field: '{}', range_field: '{}'",
                hash_field, range_field
            );

            Ok(Some(KeyConfig {
                hash_field,
                range_field,
            }))
        } else {
            info!("⚠️ No key configuration found in schema file");
            Ok(None)
        }
    }

    /// Get field value from a schema using database operations (consolidated implementation)
    fn get_field_value_from_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        // Use the unified FieldValueResolver instead of duplicate implementation
        crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(
            db_ops, schema, field_name, None, None,
        )
    }
}
