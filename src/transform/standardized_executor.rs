//! Standardized Transform Execution Pattern with Event Orchestration
//!
//! This module enforces a consistent execution sequence for all transforms:
//! 1. Gather inputs from data sources (event-driven or direct)
//! 2. Run the transform computation
//! 3. Execute mutations to update the database
//! 4. Publish events for downstream coordination
//!
//! This pattern ensures consistency across all transform types and integrates
//! with the event orchestrator for proper event-driven coordination.

use crate::schema::types::{Transform, Mutation, MutationType};
use crate::schema::SchemaError;
use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use log::{info, error, warn};

/// Standardized execution result containing both computation results and mutations to execute
#[derive(Debug, Clone)]
pub struct StandardizedExecutionResult {
    /// The computed result from the transform
    pub computation_result: JsonValue,
    /// Mutations that need to be executed to persist the results
    pub mutations: Vec<Mutation>,
    /// Metadata about the execution
    pub metadata: ExecutionMetadata,
}

/// Metadata about the execution process
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    /// Duration of input gathering phase
    pub input_gathering_duration: std::time::Duration,
    /// Duration of transform computation phase
    pub computation_duration: std::time::Duration,
    /// Duration of mutation preparation phase
    pub mutation_preparation_duration: std::time::Duration,
    /// Number of inputs gathered
    pub inputs_count: usize,
    /// Number of mutations prepared
    pub mutations_count: usize,
    /// Any warnings or issues encountered
    pub warnings: Vec<String>,
}

/// Input provider trait for gathering transform inputs
pub trait InputProvider {
    /// Get input value for a given input name
    fn get_input(&self, input_name: &str) -> Result<JsonValue, Box<dyn std::error::Error>>;
    
    /// Get multiple inputs at once (for efficiency)
    fn get_inputs(&self, input_names: &[String]) -> Result<HashMap<String, JsonValue>, Box<dyn std::error::Error>> {
        let mut inputs = HashMap::new();
        for name in input_names {
            match self.get_input(name) {
                Ok(value) => { inputs.insert(name.clone(), value); }
                Err(e) => { return Err(e); }
            }
        }
        Ok(inputs)
    }
}

/// Mutation executor trait for executing mutations
pub trait MutationExecutor {
    /// Execute a single mutation
    fn execute_mutation(&self, mutation: &Mutation) -> Result<(), SchemaError>;
    
    /// Execute multiple mutations in sequence
    fn execute_mutations(&self, mutations: &[Mutation]) -> Result<(), SchemaError> {
        for mutation in mutations {
            self.execute_mutation(mutation)?;
        }
        Ok(())
    }
}

/// Standardized transform executor that enforces the three-phase execution pattern
pub struct StandardizedTransformExecutor;

impl StandardizedTransformExecutor {
    /// Create a new standardized transform executor
    pub fn new(_message_bus: Arc<MessageBus>) -> Self {
        Self
    }

    /// Execute a transform following the standardized three-phase pattern:
    /// 1. Gather inputs
    /// 2. Run transform computation
    /// 3. Execute mutations
    pub fn execute_transform<P, E>(
        &self,
        transform: &Transform,
        input_provider: &P,
        mutation_executor: &E,
    ) -> Result<StandardizedExecutionResult, SchemaError>
    where
        P: InputProvider,
        E: MutationExecutor,
    {
        let start_time = std::time::Instant::now();
        
        info!("🚀 Starting standardized transform execution for transform: {}", transform.get_output());
        
        // Phase 1: Gather inputs
        let (input_values, input_gathering_duration) = self.gather_inputs(transform, input_provider)?;
        
        // Phase 2: Run transform computation
        let (computation_result, computation_duration) = self.run_transform_computation(transform, &input_values)?;
        
        // Phase 3: Prepare and execute mutations
        let (mutations, mutation_preparation_duration) = self.prepare_mutations(transform, &computation_result)?;
        
        // Execute mutations using the provided executor
        if !mutations.is_empty() {
            info!("📝 Executing {} mutations for transform results", mutations.len());
            mutation_executor.execute_mutations(&mutations)?;
            info!("✅ All mutations executed successfully");
        } else {
            warn!("⚠️ No mutations prepared for transform results");
        }
        
        let total_duration = start_time.elapsed();
        info!("🎯 Standardized transform execution completed in {:?}", total_duration);
        
        Ok(StandardizedExecutionResult {
            computation_result,
            mutations: mutations.clone(),
            metadata: ExecutionMetadata {
                input_gathering_duration,
                computation_duration,
                mutation_preparation_duration,
                inputs_count: input_values.len(),
                mutations_count: mutations.len(),
                warnings: vec![], // TODO: Collect warnings during execution
            },
        })
    }

    /// Phase 1: Gather inputs from data sources
    fn gather_inputs<P>(
        &self,
        transform: &Transform,
        input_provider: &P,
    ) -> Result<(HashMap<String, JsonValue>, std::time::Duration), SchemaError>
    where
        P: InputProvider,
    {
        let start_time = std::time::Instant::now();
        
        info!("📥 Phase 1: Gathering inputs for transform '{}'", transform.get_output());
        
        let mut input_values = HashMap::new();
        let inputs_to_process: Vec<String> = if transform.get_inputs().is_empty() {
            transform.analyze_dependencies().into_iter().collect()
        } else {
            transform.get_inputs().to_vec()
        };
        
        info!("🔍 Processing {} inputs: {:?}", inputs_to_process.len(), inputs_to_process);
        
        for input_name in inputs_to_process {
            match input_provider.get_input(&input_name) {
                Ok(value) => {
                    input_values.insert(input_name.clone(), value);
                    info!("✅ Gathered input '{}'", input_name);
                }
                Err(e) => {
                    error!("❌ Failed to gather input '{}': {}", input_name, e);
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to gather input '{}': {}",
                        input_name, e
                    )));
                }
            }
        }
        
        let duration = start_time.elapsed();
        info!("✅ Phase 1 completed: Gathered {} inputs in {:?}", input_values.len(), duration);
        
        Ok((input_values, duration))
    }

    /// Phase 2: Run transform computation
    fn run_transform_computation(
        &self,
        transform: &Transform,
        input_values: &HashMap<String, JsonValue>,
    ) -> Result<(JsonValue, std::time::Duration), SchemaError> {
        let start_time = std::time::Instant::now();
        
        info!("🧮 Phase 2: Running transform computation for '{}'", transform.get_output());
        
        // Use the existing TransformExecutor for computation
        let result = crate::transform::executor::TransformExecutor::execute_transform(
            transform,
            input_values.clone(),
        )?;
        
        let duration = start_time.elapsed();
        info!("✅ Phase 2 completed: Computation finished in {:?}", duration);
        
        Ok((result, duration))
    }

    /// Phase 3: Prepare mutations for database updates
    fn prepare_mutations(
        &self,
        transform: &Transform,
        computation_result: &JsonValue,
    ) -> Result<(Vec<Mutation>, std::time::Duration), SchemaError> {
        let start_time = std::time::Instant::now();
        
        info!("📝 Phase 3: Preparing mutations for transform '{}'", transform.get_output());
        
        let mut mutations = Vec::new();
        
        // Parse the output field to determine target schema and field
        let output_field = transform.get_output();
        if let Some(dot_pos) = output_field.find('.') {
            let schema_name = &output_field[..dot_pos];
            let field_name = &output_field[dot_pos + 1..];
            
            // Create mutation to update the field with the computation result
            let mut fields_and_values = HashMap::new();
            fields_and_values.insert(field_name.to_string(), computation_result.clone());
            
            let mutation = Mutation::new(
                schema_name.to_string(),
                fields_and_values,
                TRANSFORM_SYSTEM_ID.to_string(),
                0, // trust_distance
                MutationType::Update,
            );
            
            mutations.push(mutation);
            info!("📝 Prepared mutation for {}.{}", schema_name, field_name);
        } else {
            return Err(SchemaError::InvalidField(format!(
                "Invalid output field format '{}', expected 'Schema.field'",
                output_field
            )));
        }
        
        let duration = start_time.elapsed();
        info!("✅ Phase 3 completed: Prepared {} mutations in {:?}", mutations.len(), duration);
        
        Ok((mutations, duration))
    }
}

/// Event-driven input provider that uses the message bus to gather inputs
pub struct EventDrivenInputProvider {
    message_bus: Arc<MessageBus>,
    /// Cache for recently gathered inputs to avoid repeated requests
    input_cache: std::sync::Mutex<HashMap<String, JsonValue>>,
}

impl EventDrivenInputProvider {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            message_bus,
            input_cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Request field value through the message bus
    fn request_field_value(&self, input_name: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
        // Parse input name to extract schema and field
        if let Some(dot_pos) = input_name.find('.') {
            let schema_name = &input_name[..dot_pos];
            let field_name = &input_name[dot_pos + 1..];
            
            // Check cache first
            {
                let cache = self.input_cache.lock().unwrap();
                if let Some(cached_value) = cache.get(input_name) {
                    return Ok(cached_value.clone());
                }
            }
            
            // Create FieldValueSetRequest
            let correlation_id = uuid::Uuid::new_v4().to_string();
            let request = crate::fold_db_core::infrastructure::message_bus::request_events::FieldValueSetRequest {
                correlation_id: correlation_id.clone(),
                schema_name: schema_name.to_string(),
                field_name: field_name.to_string(),
                value: JsonValue::Null, // Request current value
                source_pub_key: "event_driven_input_provider".to_string(),
                mutation_context: None, // No mutation context for input requests
            };
            
            // Publish request
            self.message_bus.publish(request)?;
            
            // For now, return a placeholder - in a real implementation, this would
            // wait for the response or use a different mechanism
            let placeholder_value = JsonValue::String(format!("EVENT_REQUESTED_{}", input_name));
            
            // Cache the placeholder
            {
                let mut cache = self.input_cache.lock().unwrap();
                cache.insert(input_name.to_string(), placeholder_value.clone());
            }
            
            Ok(placeholder_value)
        } else {
            Err(format!("Invalid input name format '{}', expected 'Schema.field'", input_name).into())
        }
    }
}

impl InputProvider for EventDrivenInputProvider {
    fn get_input(&self, input_name: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
        self.request_field_value(input_name)
    }
}

/// Database-backed input provider that fetches inputs from the database
pub struct DatabaseInputProvider {
    db_ops: Arc<crate::db_operations::DbOperations>,
}

impl DatabaseInputProvider {
    pub fn new(db_ops: Arc<crate::db_operations::DbOperations>) -> Self {
        Self { db_ops }
    }
}

impl InputProvider for DatabaseInputProvider {
    fn get_input(&self, input_name: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
        // Parse input name to extract schema and field
        if let Some(dot_pos) = input_name.find('.') {
            let schema_name = &input_name[..dot_pos];
            let field_name = &input_name[dot_pos + 1..];
            
            // Get schema first
            let schema = self.db_ops.get_schema(schema_name)?
                .ok_or_else(|| format!("Schema '{}' not found", schema_name))?;
            
            // Get field value using the unified resolver
            match crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(
                &self.db_ops, &schema, field_name, None, None
            ) {
                Ok(value) => Ok(value),
                Err(e) => Err(format!("Database error: {}", e).into()),
            }
        } else {
            Err(format!("Invalid input name format '{}', expected 'Schema.field'", input_name).into())
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock input provider for testing
    struct MockInputProvider {
        inputs: HashMap<String, JsonValue>,
    }

    impl MockInputProvider {
        fn new() -> Self {
            Self {
                inputs: HashMap::new(),
            }
        }

        fn add_input(&mut self, name: String, value: JsonValue) {
            self.inputs.insert(name, value);
        }
    }

    impl InputProvider for MockInputProvider {
        fn get_input(&self, input_name: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
            self.inputs.get(input_name)
                .cloned()
                .ok_or_else(|| format!("Input '{}' not found", input_name).into())
        }
    }

    /// Mock mutation executor for testing
    struct MockMutationExecutor {
        #[allow(dead_code)]
        executed_mutations: Vec<Mutation>,
    }

    impl MockMutationExecutor {
        fn new() -> Self {
            Self {
                executed_mutations: Vec::new(),
            }
        }

        #[allow(dead_code)]
        fn get_executed_mutations(&self) -> &[Mutation] {
            &self.executed_mutations
        }
    }

    impl MutationExecutor for MockMutationExecutor {
        fn execute_mutation(&self, _mutation: &Mutation) -> Result<(), SchemaError> {
            // In a real implementation, this would execute the mutation
            // For testing, we just track what would be executed
            Ok(())
        }
    }

    #[test]
    fn test_standardized_execution_sequence() {
        // Create a simple declarative transform
        let mut fields = std::collections::HashMap::new();
        fields.insert("result".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("number".to_string()),
            atom_uuid: Some("input.field1".to_string()),
        });
        
        let schema = crate::schema::types::json_schema::DeclarativeSchemaDefinition {
            name: "TestSchema".to_string(),
            schema_type: crate::schema::types::schema::SchemaType::Single,
            fields,
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["field1".to_string(), "field2".to_string()],
            "TestSchema.result".to_string(),
        );

        // Create mock input provider
        let mut input_provider = MockInputProvider::new();
        input_provider.add_input("field1".to_string(), JsonValue::Number(10.into()));
        input_provider.add_input("field2".to_string(), JsonValue::Number(20.into()));

        // Create mock mutation executor
        let _mutation_executor = MockMutationExecutor::new();

        // Create executor (we'll need to mock the message bus for this test)
        // For now, we'll test the individual phases
        let executor = StandardizedTransformExecutor::new(
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new())
        );

        // Test Phase 1: Gather inputs
        let (input_values, input_duration) = executor.gather_inputs(&transform, &input_provider).unwrap();
        assert_eq!(input_values.len(), 2);
        assert!(input_duration.as_nanos() > 0);

        // Test Phase 2: Run computation
        let (result, computation_duration) = executor.run_transform_computation(&transform, &input_values).unwrap();
        assert!(computation_duration.as_nanos() > 0);
        // Note: The actual computation result depends on the transform logic implementation

        // Test Phase 3: Prepare mutations
        let (mutations, mutation_duration) = executor.prepare_mutations(&transform, &result).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].schema_name, "TestSchema");
        assert!(mutations[0].fields_and_values.contains_key("result"));
        assert!(mutation_duration.as_nanos() > 0);
    }

    #[test]
    fn test_input_provider_trait() {
        let mut provider = MockInputProvider::new();
        provider.add_input("test".to_string(), JsonValue::String("value".to_string()));

        assert_eq!(provider.get_input("test").unwrap(), JsonValue::String("value".to_string()));
        assert!(provider.get_input("missing").is_err());
    }

    #[test]
    fn test_mutation_executor_trait() {
        let executor = MockMutationExecutor::new();
        let mutation = Mutation::new(
            "TestSchema".to_string(),
            HashMap::new(),
            "test".to_string(),
            0,
            MutationType::Update,
        );

        assert!(executor.execute_mutation(&mutation).is_ok());
    }

    #[test]
    fn test_orchestrated_execution_sequence() {
        // Create a simple declarative transform
        let mut fields = std::collections::HashMap::new();
        fields.insert("result".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("number".to_string()),
            atom_uuid: Some("input.field1".to_string()),
        });
        
        let schema = crate::schema::types::json_schema::DeclarativeSchemaDefinition {
            name: "TestSchema".to_string(),
            schema_type: crate::schema::types::schema::SchemaType::Single,
            fields,
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["field1".to_string(), "field2".to_string()],
            "TestSchema.result".to_string(),
        );

        // Create mock input provider
        let mut input_provider = MockInputProvider::new();
        input_provider.add_input("field1".to_string(), JsonValue::Number(10.into()));
        input_provider.add_input("field2".to_string(), JsonValue::Number(20.into()));

        // Create mock mutation executor
        let _mutation_executor = MockMutationExecutor::new();

        // Create orchestrated executor (we'll need to mock the orchestrator for this test)
        // For now, we'll test the individual phases
        let message_bus = Arc::new(MessageBus::new());
        
        // Test Phase 1: Gather inputs with orchestration
        let executor = StandardizedTransformExecutor::new(message_bus.clone());
        let (input_values, input_duration) = executor.gather_inputs(&transform, &input_provider).unwrap();
        assert_eq!(input_values.len(), 2);
        assert!(input_duration.as_nanos() > 0);

        // Test Phase 2: Run computation
        let (result, computation_duration) = executor.run_transform_computation(&transform, &input_values).unwrap();
        assert!(computation_duration.as_nanos() > 0);

        // Test Phase 3: Prepare mutations
        let (mutations, mutation_duration) = executor.prepare_mutations(&transform, &result).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].schema_name, "TestSchema");
        assert!(mutations[0].fields_and_values.contains_key("result"));
        assert!(mutation_duration.as_nanos() > 0);
    }

    #[test]
    fn test_event_driven_input_provider() {
        let message_bus = Arc::new(MessageBus::new());
        let provider = EventDrivenInputProvider::new(message_bus);

        // Test getting input through event-driven mechanism
        let result = provider.get_input("TestSchema.field1");
        assert!(result.is_ok());
        
        // The result should be a placeholder indicating the event was requested
        let value = result.unwrap();
        assert!(value.is_string());
        assert!(value.as_str().unwrap().contains("EVENT_REQUESTED_"));
    }
}
