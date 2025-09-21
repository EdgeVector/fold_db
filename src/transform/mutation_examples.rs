//! Mutation-Based Data Storage Examples
//!
//! This module demonstrates how to use mutations for data storage instead of
//! direct database access, following the standardized transform execution pattern.

use crate::schema::constants::DATA_STORAGE_SYSTEM_ID;
#[allow(unused_imports)]
use crate::schema::types::{Mutation, MutationType, Transform};
use crate::schema::SchemaError;
// Note: StandardizedTransformExecutor has been removed as it was unused
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::fold_db_core::services::mutation::MutationService;
use log::{error, info};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Generate a simple hash for mutation tracking
fn generate_mutation_hash(mutation: &Mutation) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(mutation.schema_name.as_bytes());
    hasher.update(format!("{:?}", mutation.mutation_type).as_bytes());

    // Add field names and values to hash
    let mut field_entries: Vec<_> = mutation.fields_and_values.iter().collect();
    field_entries.sort_by_key(|(key, _)| *key);

    for (field_name, field_value) in field_entries {
        hasher.update(field_name.as_bytes());
        hasher.update(field_value.to_string().as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

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
        info!(
            "📝 Storing user profile using mutations for user: {}",
            user_id
        );

        // Create mutation for user profile data
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert(
            "user_id".to_string(),
            JsonValue::String(user_id.to_string()),
        );
        fields_and_values.insert(
            "username".to_string(),
            JsonValue::String(username.to_string()),
        );
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
        info!(
            "📝 Updating blog post using mutations for post: {}",
            post_id
        );

        // Create mutation for blog post data
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert(
            "post_id".to_string(),
            JsonValue::String(post_id.to_string()),
        );
        fields_and_values.insert("title".to_string(), JsonValue::String(title.to_string()));
        fields_and_values.insert(
            "content".to_string(),
            JsonValue::String(content.to_string()),
        );
        fields_and_values.insert("author".to_string(), JsonValue::String(author.to_string()));
        fields_and_values.insert(
            "publish_date".to_string(),
            JsonValue::String(publish_date.to_string()),
        );

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
        info!(
            "📝 Storing inventory data using mutations for product: {} at location: {}",
            product_id, location
        );

        // For range schemas, we need to include the range key in the mutation
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert(
            "product_id".to_string(),
            JsonValue::String(product_id.to_string()),
        );
        fields_and_values.insert(
            "location".to_string(),
            JsonValue::String(location.to_string()),
        );
        fields_and_values.insert("quantity".to_string(), JsonValue::Number(quantity.into()));
        fields_and_values.insert(
            "price".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(price).unwrap()),
        );

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
        let mutation_hash = generate_mutation_hash(mutation);

        info!("🔄 Executing mutation with hash: {}", mutation_hash);

        // For this example, we'll simulate mutation execution
        // In practice, this would use the mutation service to update field values
        info!("📊 Mutation details:");
        info!("  Schema: {}", mutation.schema_name);
        info!("  Type: {:?}", mutation.mutation_type);
        info!(
            "  Fields: {:?}",
            mutation.fields_and_values.keys().collect::<Vec<_>>()
        );
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

// Note: TransformWithMutationStorage and related code removed as it depended on StandardizedTransformExecutor

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
            let mutation_hash = generate_mutation_hash(mutation);

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
                Err(SchemaError::InvalidData(
                    "Mutation validation failed".to_string(),
                ))
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
            info!(
                "🔄 Executing mutation (attempt {}/{})",
                attempt, max_retries
            );

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
        let mutation_hash = generate_mutation_hash(mutation);

        // TODO: Implement actual mutation execution
        // This would use the mutation service to update field values
        info!("🔄 Executing mutation with hash: {}", mutation_hash);

        Ok(())
    }
}

// Note: Tests removed as they depended on deleted StandardizedTransformExecutor
