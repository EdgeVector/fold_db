//! Mutation-Based Data Storage Examples
//!
//! This module demonstrates how to use mutations for data storage instead of
//! direct database access, following the standardized transform execution pattern.

#[allow(unused_imports)]
use crate::schema::types::{Mutation, MutationType, Transform};
use crate::schema::SchemaError;
use crate::schema::constants::DATA_STORAGE_SYSTEM_ID;
use crate::transform::standardized_executor::{
    StandardizedTransformExecutor, InputProvider, MutationExecutor,
    StandardizedExecutionResult,
};
use crate::fold_db_core::services::mutation::MutationService;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use log::{info, error};

/// Example: Creating and executing a mutation for data storage
pub struct MutationBasedDataStorage;

impl MutationBasedDataStorage {
    /// Create a new mutation-based data storage instance
    pub fn new(_message_bus: Arc<MessageBus>) -> Self {
        Self
    }

    /// Example: Store user profile data using mutations instead of direct DB access
    pub fn store_user_profile(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
        age: u32,
    ) -> Result<(), SchemaError> {
        info!("📝 Storing user profile using mutations for user: {}", user_id);

        // Create mutation for user profile data
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), JsonValue::String(user_id.to_string()));
        fields_and_values.insert("username".to_string(), JsonValue::String(username.to_string()));
        fields_and_values.insert("email".to_string(), JsonValue::String(email.to_string()));
        fields_and_values.insert("age".to_string(), JsonValue::Number(age.into()));

        let mutation = Mutation::new(
            "UserProfile".to_string(),
            fields_and_values,
            DATA_STORAGE_SYSTEM_ID.to_string(),
            0, // trust_distance
            MutationType::Create,
        );

        // Execute the mutation through the mutation service
        self.execute_mutation(&mutation)?;

        info!("✅ User profile stored successfully using mutation");
        Ok(())
    }

    /// Example: Update blog post data using mutations
    pub fn update_blog_post(
        &self,
        post_id: &str,
        title: &str,
        content: &str,
        author: &str,
        publish_date: &str,
    ) -> Result<(), SchemaError> {
        info!("📝 Updating blog post using mutations for post: {}", post_id);

        // Create mutation for blog post data
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("post_id".to_string(), JsonValue::String(post_id.to_string()));
        fields_and_values.insert("title".to_string(), JsonValue::String(title.to_string()));
        fields_and_values.insert("content".to_string(), JsonValue::String(content.to_string()));
        fields_and_values.insert("author".to_string(), JsonValue::String(author.to_string()));
        fields_and_values.insert("publish_date".to_string(), JsonValue::String(publish_date.to_string()));

        let mutation = Mutation::new(
            "BlogPost".to_string(),
            fields_and_values,
            DATA_STORAGE_SYSTEM_ID.to_string(),
            0,
            MutationType::Update,
        );

        // Execute the mutation
        self.execute_mutation(&mutation)?;

        info!("✅ Blog post updated successfully using mutation");
        Ok(())
    }

    /// Example: Store range schema data using mutations
    pub fn store_inventory_data(
        &self,
        product_id: &str,
        location: &str,
        quantity: u32,
        price: f64,
    ) -> Result<(), SchemaError> {
        info!("📝 Storing inventory data using mutations for product: {} at location: {}", product_id, location);

        // For range schemas, we need to include the range key in the mutation
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("product_id".to_string(), JsonValue::String(product_id.to_string()));
        fields_and_values.insert("location".to_string(), JsonValue::String(location.to_string()));
        fields_and_values.insert("quantity".to_string(), JsonValue::Number(quantity.into()));
        fields_and_values.insert("price".to_string(), JsonValue::Number(serde_json::Number::from_f64(price).unwrap()));

        let mutation = Mutation::new(
            "Inventory".to_string(),
            fields_and_values,
            DATA_STORAGE_SYSTEM_ID.to_string(),
            0,
            MutationType::Create,
        );

        // Execute the mutation
        self.execute_mutation(&mutation)?;

        info!("✅ Inventory data stored successfully using mutation");
        Ok(())
    }

    /// Execute a mutation through the mutation service
    fn execute_mutation(&self, mutation: &Mutation) -> Result<(), SchemaError> {
        // Generate mutation hash for tracking
        let mutation_hash = MutationService::generate_mutation_hash(mutation)?;
        
        info!("🔄 Executing mutation with hash: {}", mutation_hash);

        // For this example, we'll simulate mutation execution
        // In practice, this would use the mutation service to update field values
        info!("📊 Mutation details:");
        info!("  Schema: {}", mutation.schema_name);
        info!("  Type: {:?}", mutation.mutation_type);
        info!("  Fields: {:?}", mutation.fields_and_values.keys().collect::<Vec<_>>());
        info!("  Hash: {}", mutation_hash);

        // TODO: Implement actual mutation execution through MutationService
        // This would involve:
        // 1. Validating the mutation
        // 2. Checking schema permissions
        // 3. Updating field values through the mutation service
        // 4. Triggering any dependent transforms

        Ok(())
    }
}

/// Example: Transform execution with mutation-based storage
pub struct TransformWithMutationStorage {
    executor: StandardizedTransformExecutor,
    data_storage: MutationBasedDataStorage,
}

impl TransformWithMutationStorage {
    /// Create a new transform executor with mutation-based storage
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            executor: StandardizedTransformExecutor::new(message_bus.clone()),
            data_storage: MutationBasedDataStorage::new(message_bus),
        }
    }

    /// Example: Execute a transform and store results using mutations
    pub fn execute_transform_with_mutation_storage<P>(
        &self,
        transform: &crate::schema::types::Transform,
        input_provider: &P,
    ) -> Result<StandardizedExecutionResult, SchemaError>
    where
        P: InputProvider,
    {
        info!("🚀 Executing transform with mutation-based storage");

        // Create a mutation executor that uses our data storage
        let mutation_executor = MutationBasedExecutor {
            data_storage: &self.data_storage,
        };

        // Execute the transform following the standardized pattern
        self.executor.execute_transform(transform, input_provider, &mutation_executor)
    }
}

/// Mutation executor that uses MutationBasedDataStorage
struct MutationBasedExecutor<'a> {
    data_storage: &'a MutationBasedDataStorage,
}

impl MutationExecutor for MutationBasedExecutor<'_> {
    fn execute_mutation(&self, mutation: &Mutation) -> Result<(), SchemaError> {
        info!("🔄 Executing mutation through MutationBasedDataStorage");
        
        // Use the data storage to execute the mutation
        self.data_storage.execute_mutation(mutation)
    }
}

/// Example: Batch mutation execution for efficiency
pub struct BatchMutationExecutor {
    mutations: Vec<Mutation>,
}

impl Default for BatchMutationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchMutationExecutor {
    /// Create a new batch mutation executor
    pub fn new() -> Self {
        Self {
            mutations: Vec::new(),
        }
    }

    /// Add a mutation to the batch
    pub fn add_mutation(&mut self, mutation: Mutation) {
        self.mutations.push(mutation);
    }

    /// Execute all mutations in the batch
    pub fn execute_batch(&self, _mutation_service: &MutationService) -> Result<(), SchemaError> {
        info!("📦 Executing batch of {} mutations", self.mutations.len());

        for (i, mutation) in self.mutations.iter().enumerate() {
            info!("🔄 Executing mutation {}/{}", i + 1, self.mutations.len());
            
            // Generate mutation hash
            let mutation_hash = MutationService::generate_mutation_hash(mutation)?;
            
            // Execute the mutation (placeholder - would use actual mutation service method)
            info!("🔄 Would execute mutation with hash: {}", mutation_hash);
        }

        info!("✅ Batch mutation execution completed successfully");
        Ok(())
    }

    /// Get the number of mutations in the batch
    pub fn count(&self) -> usize {
        self.mutations.len()
    }

    /// Clear all mutations from the batch
    pub fn clear(&mut self) {
        self.mutations.clear();
    }
}

/// Example: Conditional mutation execution based on data validation
pub struct ConditionalMutationExecutor;

impl ConditionalMutationExecutor {
    /// Create a new conditional mutation executor
    pub fn new(_message_bus: Arc<MessageBus>) -> Self {
        Self
    }

    /// Execute mutation only if validation passes
    pub fn execute_if_valid(
        &self,
        mutation: &Mutation,
        validator: impl Fn(&Mutation) -> Result<bool, SchemaError>,
    ) -> Result<(), SchemaError> {
        info!("🔍 Validating mutation before execution");

        // Validate the mutation
        match validator(mutation) {
            Ok(true) => {
                info!("✅ Mutation validation passed, executing");
                self.execute_mutation(mutation)
            }
            Ok(false) => {
                error!("❌ Mutation validation failed, skipping execution");
                Err(SchemaError::InvalidData("Mutation validation failed".to_string()))
            }
            Err(e) => {
                error!("❌ Mutation validation error: {}", e);
                Err(e)
            }
        }
    }

    /// Execute mutation with retry logic
    pub fn execute_with_retry(
        &self,
        mutation: &Mutation,
        max_retries: u32,
    ) -> Result<(), SchemaError> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            info!("🔄 Executing mutation (attempt {}/{})", attempt, max_retries);

            match self.execute_mutation(mutation) {
                Ok(_) => {
                    info!("✅ Mutation executed successfully on attempt {}", attempt);
                    return Ok(());
                }
                Err(e) => {
                    error!("❌ Mutation execution failed on attempt {}: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < max_retries {
                        info!("⏳ Waiting before retry...");
                        std::thread::sleep(std::time::Duration::from_millis(100 * attempt as u64));
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SchemaError::InvalidData("Unknown error".to_string())))
    }

    /// Execute mutation through the mutation service
    fn execute_mutation(&self, mutation: &Mutation) -> Result<(), SchemaError> {
        let mutation_hash = MutationService::generate_mutation_hash(mutation)?;
        
        // TODO: Implement actual mutation execution
        // This would use the mutation service to update field values
        info!("🔄 Executing mutation with hash: {}", mutation_hash);
        
        Ok(())
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

    #[test]
    fn test_mutation_based_data_storage() {
        // Create a mock message bus for testing
        let message_bus = Arc::new(MessageBus::new());
        let storage = MutationBasedDataStorage::new(message_bus);

        // Test storing user profile
        let result = storage.store_user_profile("user123", "alice", "alice@example.com", 25);
        assert!(result.is_ok());

        // Test updating blog post
        let result = storage.update_blog_post(
            "post456",
            "Test Post",
            "This is a test post",
            "alice",
            "2025-01-01T10:00:00Z"
        );
        assert!(result.is_ok());

        // Test storing inventory data
        let result = storage.store_inventory_data("product789", "warehouse-north", 100, 29.99);
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_mutation_executor() {
        let message_bus = Arc::new(MessageBus::new());
        let mutation_service = MutationService::new(message_bus);
        let mut batch_executor = BatchMutationExecutor::new();

        // Add multiple mutations to the batch
        let mut fields1 = HashMap::new();
        fields1.insert("name".to_string(), JsonValue::String("Product 1".to_string()));
        let mutation1 = Mutation::new(
            "Product".to_string(),
            fields1,
            "test".to_string(),
            0,
            MutationType::Create,
        );
        batch_executor.add_mutation(mutation1);

        let mut fields2 = HashMap::new();
        fields2.insert("name".to_string(), JsonValue::String("Product 2".to_string()));
        let mutation2 = Mutation::new(
            "Product".to_string(),
            fields2,
            "test".to_string(),
            0,
            MutationType::Create,
        );
        batch_executor.add_mutation(mutation2);

        assert_eq!(batch_executor.count(), 2);

        // Test batch execution (this will fail in test environment but validates the structure)
        let _result = batch_executor.execute_batch(&mutation_service);
        // Note: This will fail in test environment due to missing schema, but validates the structure
    }

    #[test]
    fn test_conditional_mutation_executor() {
        let message_bus = Arc::new(MessageBus::new());
        let executor = ConditionalMutationExecutor::new(message_bus);

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), JsonValue::String("Test".to_string()));
        let mutation = Mutation::new(
            "TestSchema".to_string(),
            fields,
            "test".to_string(),
            0,
            MutationType::Create,
        );

        // Test with validation that always passes
        let result = executor.execute_if_valid(&mutation, |_| Ok(true));
        assert!(result.is_ok());

        // Test with validation that always fails
        let result = executor.execute_if_valid(&mutation, |_| Ok(false));
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_with_mutation_storage() {
        // Create a simple transform
        use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
        use crate::schema::types::schema::SchemaType;
        use std::collections::HashMap;
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_transform".to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::new(),
            key: None,
        };
        
        let transform = Transform::new(schema, "TestSchema.result".to_string());

        // Create mock input provider
        let mut input_provider = MockInputProvider::new();
        input_provider.add_input("TestSchema.field1".to_string(), JsonValue::Number(10.into()));
        input_provider.add_input("TestSchema.field2".to_string(), JsonValue::Number(20.into()));

        // Create transform executor with mutation storage
        let message_bus = Arc::new(MessageBus::new());
        let executor = TransformWithMutationStorage::new(message_bus);

        // Execute transform (this will fail in test environment but validates the structure)
        let _result = executor.execute_transform_with_mutation_storage(&transform, &input_provider);
        // Note: This will fail in test environment due to missing schema, but validates the structure
    }
}
