use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::fold_db_core::transform_manager::utils::DefaultValueHelper;
use crate::schema::types::Transform;
use crate::schema::types::{Schema, SchemaError};
use crate::transform::executor::TransformExecutor;
use log::info;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;

/// Handles fetching input data for transform execution
pub struct InputFetcher;

impl InputFetcher {
    /// Execute a single transform with input fetching and computation
    pub fn execute_single_transform(
        _transform_id: &str,
        transform: &Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        _fold_db: Option<&mut crate::fold_db_core::FoldDB>,
    ) -> Result<JsonValue, SchemaError> {
        let mut input_values = HashMap::new();
        let inputs_to_process = Self::get_inputs_to_process(transform);

        for input_field in inputs_to_process {
            info!("🔍 TransformManager: Processing input: {}", input_field);

            let value = Self::fetch_input_value(db_ops, &input_field)?;
            input_values.insert(input_field.clone(), value);
        }

        info!(
            "📊 TransformManager: Final input values: {:?}",
            input_values.keys().collect::<Vec<_>>()
        );
        TransformExecutor::execute_transform(transform, input_values)
    }

    /// Execute a single transform with mutation context for incremental processing
    pub fn execute_single_transform_with_context(
        _transform_id: &str,
        transform: &Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        mutation_context: &Option<MutationContext>,
        _fold_db: Option<&mut crate::fold_db_core::FoldDB>,
    ) -> Result<JsonValue, SchemaError> {
        let mut input_values = HashMap::new();
        let inputs_to_process = Self::get_inputs_to_process(transform);

        for input_field in inputs_to_process {
            info!("🔍 TransformManager: Processing input: {}", input_field);

            let value =
                Self::fetch_input_value_with_context(db_ops, &input_field, mutation_context)?;
            input_values.insert(input_field.clone(), value);
        }

        info!(
            "📊 TransformManager: Final input values with context: {:?}",
            input_values.keys().collect::<Vec<_>>()
        );
        
        TransformExecutor::execute_transform(transform, input_values)
    }

    /// Get the list of inputs to process for a transform
    fn get_inputs_to_process(transform: &Transform) -> Vec<String> {
        if transform.get_inputs().is_empty() {
            transform
                .analyze_dependencies()
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            transform.get_inputs().to_vec()
        }
    }

    /// Fetch input value for a single input field
    fn fetch_input_value(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        input_field: &str,
    ) -> Result<JsonValue, SchemaError> {
        if let Some(dot_pos) = input_field.find('.') {
            // Input is in format "schema.field" - fetch specific field
            let input_schema = &input_field[..dot_pos];
            let input_field_name = &input_field[dot_pos + 1..];
            let value = Self::fetch_field_value(db_ops, input_schema, input_field_name)
                .unwrap_or_else(|_| {
                    DefaultValueHelper::get_default_value_for_field(input_field_name)
                });
            info!(
                "✅ TransformManager: Fetched field value for {}.{}",
                input_schema, input_field_name
            );
            Ok(value)
        } else {
            // Input is just a schema name - fetch entire schema data for declarative transforms
            println!(
                "🔍 TransformManager: Input '{}' is schema name, fetching entire schema data",
                input_field
            );
            let schema_data = Self::fetch_entire_schema_data(db_ops, input_field)?;
            println!(
                "✅ TransformManager: Fetched entire schema data for {}",
                input_field
            );
            Ok(schema_data)
        }
    }

    /// Fetch input value with mutation context for incremental processing
    fn fetch_input_value_with_context(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        input_field: &str,
        mutation_context: &Option<MutationContext>,
    ) -> Result<JsonValue, SchemaError> {
        if let Some(dot_pos) = input_field.find('.') {
            // Input is in format "schema.field" - fetch specific field
            let input_schema = &input_field[..dot_pos];
            let input_field_name = &input_field[dot_pos + 1..];
            let value = Self::fetch_field_value(db_ops, input_schema, input_field_name)
                .unwrap_or_else(|_| {
                    DefaultValueHelper::get_default_value_for_field(input_field_name)
                });
            info!(
                "✅ TransformManager: Fetched field value for {}.{}",
                input_schema, input_field_name
            );
            Ok(value)
        } else {
            // Input is just a schema name - use smart input gathering with mutation context
            println!(
                "🔍 TransformManager: Input '{}' is schema name, using smart input gathering",
                input_field
            );
            let schema_data =
                Self::fetch_schema_data_with_context(db_ops, input_field, mutation_context)?;
            println!(
                "✅ TransformManager: Fetched schema data with context for {}",
                input_field
            );
            Ok(schema_data)
        }
    }

    /// Fetch field value from a specific schema
    fn fetch_field_value(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
        })?;
        Self::get_field_value_from_schema(db_ops, &schema, field_name)
    }

    /// Fetch entire schema data for declarative transforms
    fn fetch_entire_schema_data(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🔍 TransformManager: Fetching entire schema data for '{}'",
            schema_name
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

        let schema_array = Self::fetch_schema_data_by_type(db_ops, &schema, schema_name)?;

        let formatted_data = json!({
            schema_name: schema_array
        });

        info!(
            "✅ TransformManager: Formatted schema data for '{}': {}",
            schema_name, formatted_data
        );
        Ok(formatted_data)
    }

    /// Fetch schema data based on schema type
    fn fetch_schema_data_by_type(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        schema_name: &str,
    ) -> Result<Vec<JsonValue>, SchemaError> {
        match schema.schema_type {
            crate::schema::types::SchemaType::Range { .. } => {
                crate::fold_db_core::transform_manager::schema_data_fetcher::SchemaDataFetcher::fetch_range_schema_data(db_ops, schema, schema_name)
            }
            crate::schema::types::SchemaType::HashRange => {
                crate::fold_db_core::transform_manager::schema_data_fetcher::SchemaDataFetcher::fetch_hashrange_schema_data(db_ops, schema, schema_name)
            }
            _ => {
                crate::fold_db_core::transform_manager::schema_data_fetcher::SchemaDataFetcher::fetch_simple_schema_data(db_ops, schema, schema_name)
            }
        }
    }

    /// Fetch schema data with mutation context for incremental processing
    fn fetch_schema_data_with_context(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        mutation_context: &Option<MutationContext>,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🔍 TransformManager: Fetching schema data with context for '{}'",
            schema_name
        );

        if let Some(ref context) = mutation_context {
            if context.incremental {
                return Self::handle_incremental_fetch(db_ops, schema_name, context);
            }
        }

        // Fall back to fetching entire schema data
        println!(
            "🔍 TransformManager: Falling back to full schema data fetch for '{}'",
            schema_name
        );
        Self::fetch_entire_schema_data(db_ops, schema_name)
    }

    /// Handles incremental fetch based on schema type and context.
    ///
    /// # Arguments
    ///
    /// * `db_ops` - Database operations
    /// * `schema_name` - Name of the schema
    /// * `context` - Mutation context with keys
    ///
    /// # Returns
    ///
    /// Schema data for specific keys or full schema data
    fn handle_incremental_fetch(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        context: &MutationContext,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🎯 TransformManager: Using incremental processing for schema '{}'",
            schema_name
        );

        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
        })?;

        if let Some(range_key) = &context.range_key {
            if matches!(
                schema.schema_type,
                crate::schema::types::SchemaType::Range { .. }
            ) {
                return Self::fetch_range_schema_incremental(db_ops, schema_name, range_key);
            }
        }

        if let (Some(hash_key), Some(range_key)) = (&context.hash_key, &context.range_key) {
            if matches!(
                schema.schema_type,
                crate::schema::types::SchemaType::HashRange
            ) {
                return Self::fetch_hashrange_schema_incremental(
                    db_ops,
                    schema_name,
                    hash_key,
                    range_key,
                );
            }
        }

        println!("⚠️ TransformManager: Incremental processing requested but no specific keys provided, falling back to full schema fetch");
        Self::fetch_entire_schema_data(db_ops, schema_name)
    }

    /// Fetches range schema data for specific range key.
    ///
    /// # Arguments
    ///
    /// * `db_ops` - Database operations
    /// * `schema_name` - Name of the schema
    /// * `range_key` - Range key to fetch
    ///
    /// # Returns
    ///
    /// Schema data for the specific range key
    fn fetch_range_schema_incremental(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🎯 TransformManager: Fetching only range_key '{}' for schema '{}'",
            range_key, schema_name
        );
        Self::fetch_schema_data_for_range_key(db_ops, schema_name, range_key)
    }

    /// Fetches hashrange schema data for specific hash and range keys.
    ///
    /// # Arguments
    ///
    /// * `db_ops` - Database operations
    /// * `schema_name` - Name of the schema
    /// * `hash_key` - Hash key to fetch
    /// * `range_key` - Range key to fetch
    ///
    /// # Returns
    ///
    /// Schema data for the specific hash and range key combination
    fn fetch_hashrange_schema_incremental(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        hash_key: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        println!(
            "🎯 TransformManager: Fetching only hash_key '{}' and range_key '{}' for schema '{}'",
            hash_key, range_key, schema_name
        );
        Self::fetch_schema_data_for_hashrange_key(db_ops, schema_name, hash_key, range_key)
    }

    /// Fetch schema data for a specific range key only
    fn fetch_schema_data_for_range_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        crate::fold_db_core::transform_manager::schema_data_fetcher::SchemaDataFetcher::fetch_schema_data_for_range_key(db_ops, schema_name, range_key)
    }

    /// Fetch schema data for a specific hash_key and range_key combination
    fn fetch_schema_data_for_hashrange_key(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        hash_key: &str,
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        crate::fold_db_core::transform_manager::schema_data_fetcher::SchemaDataFetcher::fetch_schema_data_for_hashrange_key(db_ops, schema_name, hash_key, range_key)
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
