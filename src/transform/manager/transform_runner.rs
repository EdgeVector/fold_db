use super::types::{TransformRunner, TransformResult};
use super::result_storage::ResultStorage;
use crate::schema::types::SchemaError;
use crate::schema::types::key_value::KeyValue;
use std::collections::{HashSet, HashMap};
use super::input_fetcher::InputFetcher;
use crate::transform::aggregation::{aggregate_results_unified_typed_as_records};
use crate::transform::iterator_stack_typed::adapter::execute_fields_typed;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
// Legacy ExecutionEngine removed; using typed engine via adapter
use crate::transform::shared_utilities::parse_expressions_batch;


impl TransformRunner for super::TransformManager {
    /// Execute the transform with the given context
    /// this is the meat of the transform execution
    /// @tomtang keep -- main path
    fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<TransformResult, SchemaError> {
        // Load the transform from in-memory registered transforms
        let transforms = self.registered_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        let transform = transforms.get(transform_id)
            .cloned()
            .ok_or_else(|| SchemaError::InvalidData(format!("Transform '{}' not found", transform_id)))?;
        drop(transforms); // Release the lock early
        // Execute the transform using the execution module with mutation context
        let input_values = InputFetcher::fetch_input_values_with_context(
            &transform, 
            &self.db_ops, 
            mutation_context,
        )?;
        
        let schema = transform.get_declarative_schema().unwrap();

        // Execute multi-chain coordination
        // Use field names instead of hash codes for proper key derivation
        let field_to_hash_code = schema.get_field_to_hash_code();
        let hash_to_code = schema.hash_to_code();
        let expressions: Vec<(String, String)> = field_to_hash_code
            .iter()
            .filter_map(|(field_name, hash_code)| {
                hash_to_code.get(hash_code).map(|expression| (field_name.clone(), expression.clone()))
            })
            .collect();
        let parsed_chains = parse_expressions_batch(&expressions)?;
        // Convert Vec<(String, ParsedChain)> to HashMap<String, ParsedChain>
        let chains_map: HashMap<String, ParsedChain> = parsed_chains
            .iter()
            .map(|(field_name, parsed_chain)| (field_name.clone(), parsed_chain.clone()))
            .collect();
        
        // Use the new typed engine end-to-end
        let execution_result = execute_fields_typed(&chains_map, &input_values);
        
        // Reconstruct expressions from parsed chains for unified aggregation
        let all_expressions: Vec<(String, String)> = parsed_chains
            .iter()
            .map(|(field_name, parsed_chain)| (field_name.clone(), parsed_chain.expression.clone()))
            .collect();
        let records = aggregate_results_unified_typed_as_records(
            schema,
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
        )?;

        // Store each result row as a separate mutation
        let field_to_hash_code = schema.get_field_to_hash_code();
        
        for record in &records {
            // For storage, we need to create a key - using the first field's key or a default
            let key_config = schema.key.clone();
            let row_key = KeyValue::from_mutation(&record.fields, key_config.as_ref().unwrap());
            
            // Convert field names to hash codes for storage
            let mut code_hash_to_result = std::collections::HashMap::new();
            for (field_name, field_value) in &record.fields {
                // Convert field name to hash code for storage
                if let Some(hash_code) = field_to_hash_code.get(field_name) {
                    code_hash_to_result.insert(hash_code.clone(), field_value.clone());
                }
            }
            
            // Store this row as a mutation
            ResultStorage::store_transform_result_generic(
                &transform,
                code_hash_to_result,
                row_key,
                Some(&self.message_bus)
            )?;
        }

        Ok(TransformResult::new(records))
    }

    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
        let transforms = self.registered_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        Ok(transforms.contains_key(transform_id))
    }

    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        let key = format!("{}.{}", schema_name, field_name);
        let mappings = self.schema_field_to_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        Ok(mappings.get(&key).cloned().unwrap_or_default())
    }

}