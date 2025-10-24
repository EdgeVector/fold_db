//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::infrastructure::message_bus::events::query_events::MutationExecuted;
use super::infrastructure::message_bus::request_events::MutationRequest;
use super::infrastructure::MessageBus;
use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::schema::types::{KeyValue, Mutation, Schema};
use crate::schema::{SchemaCore, SchemaError};
use log::{error, warn};

struct MutationExecution {
    schema_name: String,
    mutation_id: String,
    key_value: KeyValue,
    fields_affected: Vec<String>,
    backfill_hash: Option<String>,
}

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<MessageBus>,
    /// Flag to track if the event listener is running
    is_listening: Arc<std::sync::atomic::AtomicBool>,
}

impl MutationManager {
    /// Creates a new MutationManager instance
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<MessageBus>,
    ) -> Self {
        Self {
            db_ops,
            schema_manager,
            message_bus,
            is_listening: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Write schema operation - main orchestration method for mutations
    pub fn write_mutation(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        let start_time = std::time::Instant::now();

        let mut schema = self
            .schema_manager
            .get_schema(&mutation.schema_name)?
            .ok_or_else(|| {
                SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name))
            })?;

        let execution = Self::process_single_mutation(&self.db_ops, &mut schema, mutation)?;
        Self::persist_schema_state(
            &self.db_ops,
            &self.schema_manager,
            &execution.schema_name,
            &schema,
        )?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        Self::publish_mutation_event(
            &self.message_bus,
            "write_mutation",
            &execution,
            execution_time_ms,
        )?;

        self.db_ops.flush()?;

        Ok(execution.mutation_id)
    }

    /// Write a batch of mutations while minimizing flush calls and schema reloads
    pub fn write_mutations_batch(
        &mut self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError> {
        if mutations.is_empty() {
            return Ok(Vec::new());
        }

        let mut schema_cache: HashMap<String, Schema> = HashMap::new();
        let mut mutation_ids = Vec::with_capacity(mutations.len());

        for mutation in mutations {
            let schema_name = mutation.schema_name.clone();
            let schema = if let Some(schema) = schema_cache.get_mut(&schema_name) {
                schema
            } else {
                let loaded_schema =
                    self.schema_manager
                        .get_schema(&schema_name)?
                        .ok_or_else(|| {
                            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
                        })?;
                schema_cache.insert(schema_name.clone(), loaded_schema);
                schema_cache
                    .get_mut(&schema_name)
                    .expect("schema inserted into cache")
            };

            let start_time = std::time::Instant::now();
            let execution = Self::process_single_mutation(&self.db_ops, schema, mutation)?;
            Self::persist_schema_state(
                &self.db_ops,
                &self.schema_manager,
                &execution.schema_name,
                schema,
            )?;
            let execution_time_ms = start_time.elapsed().as_millis() as u64;
            let mutation_id = execution.mutation_id.clone();
            Self::publish_mutation_event(
                &self.message_bus,
                "write_mutation",
                &execution,
                execution_time_ms,
            )?;
            mutation_ids.push(mutation_id);
        }

        self.db_ops.flush()?;

        Ok(mutation_ids)
    }

    /// Start listening for MutationRequest events in a background thread
    pub fn start_event_listener(&self) -> Result<(), SchemaError> {
        if self.is_listening.load(std::sync::atomic::Ordering::Acquire) {
            warn!("MutationManager event listener is already running");
            return Ok(());
        }

        let db_ops = Arc::clone(&self.db_ops);
        let schema_manager = Arc::clone(&self.schema_manager);
        let message_bus = Arc::clone(&self.message_bus);
        let is_listening = Arc::clone(&self.is_listening);

        is_listening.store(true, std::sync::atomic::Ordering::Release);

        thread::spawn(move || {
            // Subscribe to MutationRequest events
            let mut consumer = message_bus.subscribe::<MutationRequest>();

            // Main event processing loop
            while is_listening.load(std::sync::atomic::Ordering::Acquire) {
                match consumer.try_recv() {
                    Ok(mutation_request) => {
                        let backfill_hash = mutation_request.mutation.backfill_hash.clone();
                        if let Err(e) = Self::handle_mutation_request_event(
                            &mutation_request,
                            &db_ops,
                            &schema_manager,
                            &message_bus,
                        ) {
                            error!("MutationManager failed to handle mutation request: {}", e);

                            // If this was part of a backfill, publish a failure event
                            if let Some(hash) = backfill_hash {
                                let fail_event = crate::fold_db_core::infrastructure::message_bus::request_events::BackfillMutationFailed {
                                    backfill_hash: hash,
                                    error: e.to_string(),
                                };
                                let _ = message_bus.publish(fail_event);
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No events available, sleep briefly to avoid busy waiting
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        error!("MutationManager message bus consumer disconnected");
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    /// Stop the event listener
    pub fn stop_event_listener(&self) {
        self.is_listening
            .store(false, std::sync::atomic::Ordering::Release);
    }

    /// Check if the event listener is currently running
    pub fn is_listening(&self) -> bool {
        self.is_listening.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Handle a mutation request event by executing the mutation
    fn handle_mutation_request_event(
        mutation_request: &MutationRequest,
        db_ops: &Arc<DbOperations>,
        schema_manager: &Arc<SchemaCore>,
        message_bus: &MessageBus,
    ) -> Result<(), SchemaError> {
        let start_time = std::time::Instant::now();

        let mutation = mutation_request.mutation.clone();
        let mut schema = schema_manager
            .get_schema(&mutation.schema_name)?
            .ok_or_else(|| {
                SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name))
            })?;

        let execution = Self::process_single_mutation(db_ops, &mut schema, mutation)?;
        Self::persist_schema_state(db_ops, schema_manager, &execution.schema_name, &schema)?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        Self::publish_mutation_event(
            message_bus,
            "mutation_request_handler",
            &execution,
            execution_time_ms,
        )?;

        db_ops.flush()?;

        Ok(())
    }

    fn process_single_mutation(
        db_ops: &DbOperations,
        schema: &mut Schema,
        mutation: Mutation,
    ) -> Result<MutationExecution, SchemaError> {
        let key_config = schema.key.clone();
        let key_config = key_config.as_ref().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "Schema '{}' is missing key configuration",
                schema.name
            ))
        })?;

        let Mutation {
            uuid: mutation_id,
            schema_name,
            fields_and_values,
            pub_key,
            backfill_hash,
            ..
        } = mutation;

        for (field_name, value) in &fields_and_values {
            schema.validate_field_value(field_name, value)?;
        }

        let key_value = KeyValue::from_mutation(&fields_and_values, key_config);
        let fields_affected: Vec<String> = fields_and_values.keys().cloned().collect();

        for (field_name, value) in fields_and_values {
            let field_classifications = schema.get_field_classifications(&field_name);

            if let Some(schema_field) = schema.runtime_fields.get_mut(&field_name) {
                db_ops.process_mutation_field_with_schema(
                    &schema_name,
                    &field_name,
                    &pub_key,
                    value,
                    &key_value,
                    schema_field,
                    field_classifications,
                )?;
            } else {
                let available_fields = schema.runtime_fields.keys().cloned().collect::<Vec<_>>();
                return Err(SchemaError::InvalidData(format!(
                    "Field '{}' not found in runtime_fields for schema '{}'. Available fields: {:?}",
                    field_name,
                    schema_name,
                    available_fields
                )));
            }
        }

        schema.sync_molecule_uuids();

        Ok(MutationExecution {
            schema_name,
            mutation_id,
            key_value,
            fields_affected,
            backfill_hash,
        })
    }

    fn persist_schema_state(
        db_ops: &DbOperations,
        schema_manager: &SchemaCore,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        db_ops.store_schema(schema_name, schema)?;
        schema_manager.load_schema_internal(schema.clone())?;
        Ok(())
    }

    fn publish_mutation_event(
        message_bus: &MessageBus,
        source: &str,
        execution: &MutationExecution,
        execution_time_ms: u64,
    ) -> Result<(), SchemaError> {
        let mutation_context = Some(MutationContext {
            key_value: Some(execution.key_value.clone()),
            mutation_hash: Some(execution.mutation_id.clone()),
            incremental: true,
            backfill_hash: execution.backfill_hash.clone(),
        });

        let event = MutationExecuted::with_context(
            source,
            execution.schema_name.clone(),
            execution_time_ms,
            execution.fields_affected.clone(),
            mutation_context,
        );

        message_bus.publish(event)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_operations::DbOperations;
    use crate::fold_db_core::infrastructure::MessageBus;
    use crate::schema::types::operations::MutationType;
    use crate::schema::types::topology::{JsonTopology, PrimitiveValueType, TopologyNode};
    use crate::schema::types::{KeyConfig, SchemaType};
    use crate::testing_utils::TestDatabaseFactory;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    const TEST_SCHEMA_NAME: &str = "MutationBatch";

    fn setup_manager() -> (
        MutationManager,
        Arc<SchemaCore>,
        Arc<MessageBus>,
        Arc<DbOperations>,
    ) {
        let (db_ops, message_bus) = TestDatabaseFactory::create_test_environment()
            .expect("failed to create test environment");

        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
                .expect("failed to create schema manager"),
        );

        let mut schema = Schema::new(
            TEST_SCHEMA_NAME.to_string(),
            SchemaType::Single,
            Some(KeyConfig::new(Some("id".to_string()), None)),
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );
        schema.populate_runtime_fields().expect("runtime fields");

        let string_topology = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        });
        schema.set_field_topology("id".to_string(), string_topology.clone());
        schema.set_field_topology("value".to_string(), string_topology);

        schema_manager
            .load_schema_internal(schema)
            .expect("failed to load schema");

        let mutation_manager = MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
        );

        (mutation_manager, schema_manager, message_bus, db_ops)
    }

    #[test]
    fn write_mutations_batch_returns_empty_for_empty_input() {
        let (mut mutation_manager, _, _, _) = setup_manager();
        let result = mutation_manager
            .write_mutations_batch(Vec::new())
            .expect("batch execution should succeed");
        assert!(result.is_empty());
    }

    #[test]
    fn write_mutations_batch_processes_multiple_mutations() {
        let (mut mutation_manager, schema_manager, message_bus, _) = setup_manager();
        let mut consumer = message_bus.subscribe::<MutationExecuted>();

        let mut fields_one: HashMap<String, serde_json::Value> = HashMap::new();
        fields_one.insert("id".to_string(), json!("key-1"));
        fields_one.insert("value".to_string(), json!("first"));

        let mut fields_two: HashMap<String, serde_json::Value> = HashMap::new();
        fields_two.insert("id".to_string(), json!("key-2"));
        fields_two.insert("value".to_string(), json!("second"));

        let mutation_one = Mutation::new(
            TEST_SCHEMA_NAME.to_string(),
            fields_one,
            KeyValue::new(Some("key-1".to_string()), None),
            "pub-key".to_string(),
            1,
            MutationType::Update,
        );
        let mutation_two = Mutation::new(
            TEST_SCHEMA_NAME.to_string(),
            fields_two,
            KeyValue::new(Some("key-2".to_string()), None),
            "pub-key".to_string(),
            1,
            MutationType::Update,
        );

        let expected_ids = vec![mutation_one.uuid.clone(), mutation_two.uuid.clone()];
        let result_ids = mutation_manager
            .write_mutations_batch(vec![mutation_one.clone(), mutation_two.clone()])
            .expect("batch execution should succeed");

        assert_eq!(result_ids, expected_ids);

        let first_event = consumer
            .recv_timeout(Duration::from_secs(1))
            .expect("expected first mutation event");
        assert_eq!(first_event.operation, "write_mutation");
        assert_eq!(first_event.schema, TEST_SCHEMA_NAME);
        assert!(first_event.fields_affected.contains(&"value".to_string()));
        let first_context = first_event
            .mutation_context
            .expect("mutation context should be present");
        assert_eq!(first_context.mutation_hash, Some(expected_ids[0].clone()));

        let second_event = consumer
            .recv_timeout(Duration::from_secs(1))
            .expect("expected second mutation event");
        assert_eq!(second_event.operation, "write_mutation");
        assert_eq!(second_event.schema, TEST_SCHEMA_NAME);
        let second_context = second_event
            .mutation_context
            .expect("mutation context should be present");
        assert_eq!(second_context.mutation_hash, Some(expected_ids[1].clone()));

        let stored_schema = schema_manager
            .get_schema(TEST_SCHEMA_NAME)
            .expect("schema lookup should succeed")
            .expect("schema should exist");
        let molecule_map = stored_schema
            .field_molecule_uuids
            .as_ref()
            .expect("molecule uuids should be recorded");
        assert!(molecule_map.contains_key("value"));
    }
}
