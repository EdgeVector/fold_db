use super::manager::TransformManager;
use crate::fold_db_core::transform_manager::utils::*;
use crate::transform::executor::TransformExecutor;
use crate::schema::types::{Schema, SchemaError, SchemaType};
use log::info;
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::{Value as JsonValue, json};

impl TransformManager {

    /// Execute a single transform with input fetching and computation
    pub fn execute_single_transform(_transform_id: &str, transform: &crate::schema::types::Transform, db_ops: &Arc<crate::db_operations::DbOperations>) -> Result<JsonValue, SchemaError> {
        println!("🔧 TransformManager: Executing transform '{}'", _transform_id);
        println!("🔧 TransformManager: Transform inputs: {:?}", transform.get_inputs());
        
        let mut input_values = HashMap::new();
        let inputs_to_process = if transform.get_inputs().is_empty() { transform.analyze_dependencies().into_iter().collect::<Vec<_>>() } else { transform.get_inputs().to_vec() };
        
        println!("🔍 TransformManager: Processing {} inputs for transform", inputs_to_process.len());
        
        for input_field in inputs_to_process {
            info!("🔍 TransformManager: Processing input: {}", input_field);
            
            if let Some(dot_pos) = input_field.find('.') {
                // Input is in format "schema.field" - fetch specific field
                let input_schema = &input_field[..dot_pos];
                let input_field_name = &input_field[dot_pos + 1..];
                let value = Self::fetch_field_value(db_ops, input_schema, input_field_name).unwrap_or_else(|_| DefaultValueHelper::get_default_value_for_field(input_field_name));
                input_values.insert(input_field.clone(), value);
                info!("✅ TransformManager: Fetched field value for {}.{}", input_schema, input_field_name);
            } else {
                // Input is just a schema name - fetch entire schema data for declarative transforms
                println!("🔍 TransformManager: Input '{}' is schema name, fetching entire schema data", input_field);
                let schema_data = Self::fetch_entire_schema_data(db_ops, &input_field)?;
                input_values.insert(input_field.clone(), schema_data);
                println!("✅ TransformManager: Fetched entire schema data for {}", input_field);
            }
        }
        
        info!("📊 TransformManager: Final input values: {:?}", input_values.keys().collect::<Vec<_>>());
        TransformExecutor::execute_transform(transform, input_values)
    }
    
    /// Fetch field value from a specific schema
    fn fetch_field_value(db_ops: &Arc<crate::db_operations::DbOperations>, schema_name: &str, field_name: &str) -> Result<JsonValue, SchemaError> {
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
        Self::get_field_value_from_schema(db_ops, &schema, field_name)
    }
    
    /// Fetch entire schema data for declarative transforms
    fn fetch_entire_schema_data(db_ops: &Arc<crate::db_operations::DbOperations>, schema_name: &str) -> Result<JsonValue, SchemaError> {
        println!("🔍 TransformManager: Fetching entire schema data for '{}'", schema_name);
        
        // Get the schema definition
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
        
        // Get all field names from the schema
        let field_names: Vec<String> = schema.fields.keys().cloned().collect();
        println!("🔍 TransformManager: Schema '{}' has fields: {:?}", schema_name, field_names);
        
        // For range schemas, we need to get all the data without a range filter
        // This means we need to query all atoms for this schema
        let mut schema_array = Vec::new();
        
        if matches!(schema.schema_type, SchemaType::Range { .. }) {
            info!("🔍 TransformManager: Processing Range schema '{}'", schema_name);
            
            // Get all field names from the schema definition
            let field_names: Vec<String> = schema.fields.keys().cloned()
                .collect();
            
            println!("🔍 TransformManager: Schema '{}' has fields: {:?}", schema_name, field_names);
            
            // For each field, get the MoleculeRange and collect atoms
            let mut blog_posts_by_date: std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>> = std::collections::HashMap::new();
            
            for field_name in field_names {
                let molecule_uuid = format!("{}_{}_range", schema_name, field_name);
                println!("🔍 TransformManager: Looking for MoleculeRange: {}", molecule_uuid);
                
                match db_ops.get_item::<crate::atom::MoleculeRange>(&format!("ref:{}", molecule_uuid)) {
                    Ok(Some(range_molecule)) => {
                        println!("✅ Found MoleculeRange for field '{}' with {} entries", field_name, range_molecule.atom_uuids.len());
                        
                        // Process each atom in the range
                        for (range_key, atom_uuid) in &range_molecule.atom_uuids {
                            let blog_post = blog_posts_by_date.entry(range_key.clone()).or_default();
                            
                            // Load the atom and extract its value
                            match db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid)) {
                                Ok(Some(atom)) => {
                                    let content = atom.content();
                                    println!("🔍 TransformManager: Atom {} content: {}", atom_uuid, content);
                                    
                                    // Extract the value from the atom content
                                    if let Some(value) = content.get("value") {
                                        blog_post.insert(field_name.clone(), value.clone());
                                        println!("🔍 TransformManager: Added field '{}' = {} to blog post with range_key '{}'", field_name, value, range_key);
                                    }
                                }
                                Ok(None) => {
                                    println!("⚠️ Atom {} not found for field '{}'", atom_uuid, field_name);
                                }
                                Err(e) => {
                                    println!("❌ Error loading atom {} for field '{}': {}", atom_uuid, field_name, e);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        println!("⚠️ MoleculeRange '{}' not found for field '{}'", molecule_uuid, field_name);
                    }
                    Err(e) => {
                        println!("❌ Error loading MoleculeRange '{}' for field '{}': {}", molecule_uuid, field_name, e);
                    }
                }
            }
            
            // Convert grouped data into array format
            for (range_key, blog_post_data) in blog_posts_by_date {
                let mut schema_item = serde_json::Map::new();
                schema_item.insert("publish_date".to_string(), json!(range_key));
                
                // Add all field values to the schema item
                for (field_name, field_value) in blog_post_data {
                    schema_item.insert(field_name, field_value);
                }
                
                schema_array.push(json!(schema_item));
            }
            
            info!("🔍 TransformManager: Found {} blog posts for schema '{}'", schema_array.len(), schema_name);
        } else {
            // For non-range schemas, use the original approach
            let mut schema_data = serde_json::Map::new();
            
            for field_name in &field_names {
                match Self::get_field_value_from_schema(db_ops, &schema, field_name) {
                    Ok(value) => {
                        schema_data.insert(field_name.clone(), value);
                    }
                    Err(e) => {
                        info!("⚠️ TransformManager: Failed to get field '{}' for schema '{}': {}", field_name, schema_name, e);
                        schema_data.insert(field_name.clone(), JsonValue::Null);
                    }
                }
            }
            
            let result = JsonValue::Object(schema_data);
            info!("✅ TransformManager: Retrieved schema data for '{}': {}", schema_name, result);
            
            // Format the data as expected by declarative transforms
            if let Some(obj) = result.as_object() {
                // Get the range keys (timestamps for range schemas)
                let range_keys: Vec<String> = obj.values()
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
                    for (field_name, field_data) in obj {
                        if let Some(field_obj) = field_data.as_object() {
                            if let Some(value) = field_obj.get(&range_key) {
                                schema_item.insert(field_name.clone(), value.clone());
                            }
                        }
                    }
                    
                    schema_array.push(json!(schema_item));
                }
            }
        }
        
        let formatted_data = json!({
            schema_name: schema_array
        });
        
        info!("✅ TransformManager: Formatted schema data for '{}': {}", schema_name, formatted_data);
        Ok(formatted_data)
    }
    
    
    
    /// Generic result storage for any transform
    pub fn store_transform_result_generic(db_ops: &Arc<crate::db_operations::DbOperations>, transform: &crate::schema::types::Transform, result: &JsonValue) -> Result<(), SchemaError> {
        if let Some(dot_pos) = transform.get_output().find('.') {
            let schema_name = &transform.get_output()[..dot_pos];
            let field_name = &transform.get_output()[dot_pos + 1..];
            
            // Check if this is a HashRange schema and handle it specially
            if let Ok(Some(schema)) = db_ops.get_schema(schema_name) {
                if matches!(schema.schema_type, crate::schema::types::SchemaType::HashRange) {
                    info!("🔑 Storing HashRange transform result for schema '{}'", schema_name);
                    return Self::store_hashrange_transform_result(db_ops, schema_name, result);
                }
            }
            
            // For non-HashRange schemas, use the original single-field storage
            let atom = db_ops.create_atom(schema_name, "transform_system".to_string(), None, result.clone(), None)?;
            Self::update_field_reference(db_ops, schema_name, field_name, atom.uuid())
        } else {
            Err(SchemaError::InvalidField(format!("Invalid output field format '{}', expected 'Schema.field'", transform.get_output())))
        }
    }
    
    /// Special storage for HashRange schema transform results
    fn store_hashrange_transform_result(db_ops: &Arc<crate::db_operations::DbOperations>, schema_name: &str, result: &JsonValue) -> Result<(), SchemaError> {
        info!("🔑 Storing HashRange transform result for schema '{}': {}", schema_name, result);
        
        // For HashRange schemas, we need to create separate atoms for each hash key (word)
        if let Some(result_obj) = result.as_object() {
            // Extract the hash_key array (words) and corresponding data arrays
            let hash_keys = result_obj.get("hash_key")
                .and_then(|h| h.as_array())
                .ok_or_else(|| SchemaError::InvalidData("HashRange result must contain hash_key array".to_string()))?;
            
            let range_keys = result_obj.get("range_key")
                .and_then(|r| r.as_array())
                .ok_or_else(|| SchemaError::InvalidData("HashRange result must contain range_key array".to_string()))?;
            
            // Extract field values - handle both single values and arrays
            let author_values = result_obj.get("author").map(|a| {
                if a.is_array() { a.clone() } else { json!([a.clone()]) }
            }).unwrap_or_default();
            let blog_values = result_obj.get("blog").map(|b| {
                if b.is_array() { b.clone() } else { json!([b.clone()]) }
            }).unwrap_or_default();
            let title_values = result_obj.get("title").map(|t| {
                if t.is_array() { t.clone() } else { json!([t.clone()]) }
            }).unwrap_or_default();
            let tags_values = result_obj.get("tags").map(|t| {
                if t.is_array() { t.clone() } else { json!([t.clone()]) }
            }).unwrap_or_default();
            
            // Convert to arrays for processing
            let author_array = author_values.as_array().cloned().unwrap_or_default();
            let blog_array = blog_values.as_array().cloned().unwrap_or_default();
            let title_array = title_values.as_array().cloned().unwrap_or_default();
            let tags_array = tags_values.as_array().cloned().unwrap_or_default();
            
            println!("🔍 DEBUG: Processing {} hash keys from {} blog posts", hash_keys.len(), blog_array.len());
            
            // Process each blog post individually and create/update atoms for each word
            for (blog_index, blog_post) in blog_array.iter().enumerate() {
                println!("🔍 DEBUG: Processing blog post {}: {:?}", blog_index, blog_post);
                
                // Extract blog post metadata
                let author = author_array.get(blog_index).cloned().unwrap_or(JsonValue::Null);
                let title = title_array.get(blog_index).cloned().unwrap_or(JsonValue::Null);
                let tags = tags_array.get(blog_index).cloned().unwrap_or(JsonValue::Null);
                let range = range_keys.get(blog_index).cloned().unwrap_or(JsonValue::Null);
                
                // Extract content from blog post to find words
                let content = if let Some(blog_obj) = blog_post.as_object() {
                    blog_obj.get("content").and_then(|c| c.as_str()).unwrap_or("")
                } else {
                    ""
                };
                
                // Extract words from content (split on whitespace and punctuation)
                let words: Vec<&str> = content
                    .split_whitespace()
                    .map(|word| word.trim_matches(|c: char| c.is_ascii_punctuation()))
                    .filter(|word| !word.is_empty())
                    .collect();
                
                println!("🔍 DEBUG: Extracted {} words from blog post {}: {:?}", words.len(), blog_index, &words[..std::cmp::min(10, words.len())]);
                
                // For each word in this blog post, create or update a HashRange atom
                for word in words {
                    let atom_uuid = format!("{}_{}", schema_name, word);
                    
                    // Create the data for this word from this blog post
                    let mut word_data = serde_json::Map::new();
                    word_data.insert("hash".to_string(), json!(word));
                    word_data.insert("author".to_string(), author.clone());
                    word_data.insert("blog".to_string(), blog_post.clone());
                    word_data.insert("title".to_string(), title.clone());
                    word_data.insert("tags".to_string(), tags.clone());
                    word_data.insert("range".to_string(), range.clone());
                    
                    let word_result = json!(word_data);
                    
                    println!("🔑 Processing word '{}' from blog post {}", word, blog_index);
                    println!("🔍 DEBUG: Word data: {}", word_result);
                    
                    // Check if atom already exists for this word
                    let existing_atom = db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid));
                    
                    let final_word_result = match existing_atom {
                        Ok(Some(existing_atom)) => {
                            println!("🔄 Found existing atom for '{}', checking for duplicates", word);
                            let existing_content = existing_atom.content();
                            
                            // Check if this blog post is already associated with this word
                            let current_blog = existing_content.get("blog");
                            let is_duplicate = match current_blog {
                                Some(existing_blog) if existing_blog.is_array() => {
                                    existing_blog.as_array().unwrap().contains(blog_post)
                                }
                                Some(existing_blog) => existing_blog == blog_post,
                                None => false
                            };
                            
                            if is_duplicate {
                                println!("🔍 DEBUG: Blog post already associated with word '{}', skipping", word);
                                continue;
                            }
                            
                            println!("🔄 Merging new blog post data for word '{}'", word);
                            let mut merged_data = existing_content.as_object().unwrap().clone();
                            
                            // Merge each field, converting to arrays as needed
                            for field_name in ["author", "blog", "title", "tags", "range"] {
                                if let Some(new_value) = word_data.get(field_name) {
                                    if let Some(existing_value) = merged_data.get(field_name) {
                                        let mut merged_array = Vec::new();
                                        
                                        // Add existing values to array
                                        if let Some(existing_array) = existing_value.as_array() {
                                            merged_array.extend(existing_array.iter().cloned());
                                        } else {
                                            merged_array.push(existing_value.clone());
                                        }
                                        
                                        // Add new value to array
                                        merged_array.push(new_value.clone());
                                        
                                        merged_data.insert(field_name.to_string(), json!(merged_array));
                                    } else {
                                        merged_data.insert(field_name.to_string(), new_value.clone());
                                    }
                                }
                            }
                            
                            json!(merged_data)
                        }
                        _ => {
                            println!("🆕 Creating new atom for word '{}'", word);
                            word_result
                        }
                    };
                    
                    // Create atom using the standard method (generates random UUID)
                    let atom = db_ops.create_atom(schema_name, "transform_system".to_string(), None, final_word_result.clone(), None)?;
                    
                    // Store the atom with our predictable UUID pattern
                    println!("🔍 DEBUG: Storing atom for word '{}' with key: atom:{}", word, atom_uuid);
                    let store_result = db_ops.store_item(&format!("atom:{}", atom_uuid), &atom);
                    match store_result {
                        Ok(_) => println!("✅ Successfully stored atom for word '{}' with key: atom:{}", word, atom_uuid),
                        Err(ref e) => println!("❌ Failed to store atom for word '{}' with key: atom:{} - Error: {}", word, atom_uuid, e),
                    }
                    store_result?;
                    
                    println!("✅ HashRange atom stored successfully for word '{}' with UUID: {}", word, atom_uuid);
                }
            }
            
            Ok(())
        } else {
            Err(SchemaError::InvalidData(format!("HashRange transform result must be an object, got: {}", result)))
        }
    }
    
    /// Update a field's molecule_uuid to point to a new atom and create proper linking
    /// SCHEMA-003: Only updates field values, NOT schema structure (schemas are immutable)
    fn update_field_reference(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        atom_uuid: &str,
    ) -> Result<(), SchemaError> {
        info!("🔗 Updating field reference: {}.{} -> atom {}", schema_name, field_name, atom_uuid);
        
        // 1. Load the schema (read-only - we will NOT modify it)
        let schema = db_ops.get_schema(schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
        
        // 2. Get the field (read-only)
        let _field = schema.fields.get(field_name)
            .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}' not found in schema '{}'", field_name, schema_name)))?;
        
        // 3. Get the field's molecule_uuid (should already exist in schema)
        let molecule_uuid = format!("{}_{}_single", schema_name, field_name);
        
        // 4. Create/update Molecule to point to the new atom (this is a field VALUE update, not schema structure)
        let molecule = crate::atom::Molecule::new(atom_uuid.to_string(), "transform_system".to_string());
        db_ops.store_item(&format!("ref:{}", molecule_uuid), &molecule)?;
        
        info!("✅ Updated field value reference for '{}.{}' to point to atom {}", schema_name, field_name, atom_uuid);
        LoggingHelper::log_molecule_operation(&molecule_uuid, atom_uuid, "creation");
        
        // SCHEMA-003: Do NOT modify schema structure - only update field value through Molecule
        // The schema remains immutable, we only updated what the field's reference points to
        
        Ok(())
    }

    /// Get field value from a schema using database operations (consolidated implementation)
    fn get_field_value_from_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        // Use the unified FieldValueResolver instead of duplicate implementation
        crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(db_ops, schema, field_name, None, None)
    }
}