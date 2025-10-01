use super::types::TransformRunner;
use super::result_storage::ResultStorage;
use crate::schema::types::SchemaError;
use serde_json::Value as JsonValue;
use std::collections::{HashSet, HashMap};
use super::input_fetcher::InputFetcher;
use crate::transform::aggregation::{aggregate_results_unified_typed};
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
    ) -> Result<JsonValue, SchemaError> {
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
        let expressions: Vec<(String, String)> = schema.hash_to_code().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
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
        let result = aggregate_results_unified_typed(
            schema,
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
        )?;

        // Store the result using message bus
        let mut result_map = std::collections::HashMap::new();
        result_map.insert("result".to_string(), result.clone());
        ResultStorage::store_transform_result_generic(
            &transform,
            result_map,
            mutation_context
                .as_ref()
                .and_then(|ctx| ctx.key_value.clone())
                .expect("Mutation context key_value required for result storage"),
            Some(&self.message_bus)
        )?;

        Ok(result)
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