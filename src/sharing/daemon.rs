use crate::messaging::{AsyncMessageBus, Event};
use crate::sharing::store::list_share_rules;
use crate::sharing::types::{ShareRule, ShareScope};
use crate::storage::SledPool;
use std::sync::Arc;

pub struct SharingPushDaemon {
    pool: Arc<SledPool>,
}

impl SharingPushDaemon {
    pub fn new(pool: Arc<SledPool>) -> Self {
        Self { pool }
    }

    pub async fn start_event_listener(&self, message_bus: Arc<AsyncMessageBus>) {
        log::info!("Starting SharingPushDaemon event listener");
        let mut receiver = message_bus.subscribe();

        loop {
            match receiver.recv().await {
                Ok(Event::MutationExecuted(mutation)) => {
                    self.handle_mutation(&mutation).await;
                }
                Ok(_) => {
                    // Ignore other events
                }
                Err(std::sync::mpsc::RecvError) => {
                    log::warn!("SharingPushDaemon receiver channel closed. Exiting listener loop.");
                    break;
                }
            }
        }
    }

    async fn handle_mutation(&self, mutation: &crate::messaging::events::MutationExecuted) {
        if mutation.metadata.source == "sync" {
            // Do not re-share data that came from sync to avoid infinite loops
            return;
        }
        
        let schema_name = &mutation.schema_name;

        // Fetch active rules
        let rules = match list_share_rules(&self.pool) {
            Ok(r) => r,
            Err(e) => {
                log::error!("SharingPushDaemon failed to load share rules: {}", e);
                return;
            }
        };

        let mut matched_rules = Vec::new();
        for rule in rules {
            if !rule.active {
                continue;
            }
            let is_match = match &rule.scope {
                ShareScope::AllSchemas => true,
                ShareScope::Schema { schema } => schema == schema_name,
                ShareScope::SchemaField { schema, .. } => schema == schema_name,
            };

            if is_match {
                matched_rules.push(rule);
            }
        }

        if matched_rules.is_empty() {
            return;
        }
        
        // We have matching rules!
        // We'll duplicate the writes into the Sled log using the share prefix.
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        
        // Wait, what exactly did the mutation write? 
        // We might not have the raw LogOp here. We just know the `key_value` and `schema_name`.
        // To construct the LogOp for the sync engine, we need to read what was written.
    }
}
