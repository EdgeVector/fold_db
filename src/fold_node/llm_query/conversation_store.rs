//! Persists AI query conversation turns to FoldDB as searchable records.
//!
//! Each `agent_query` call produces one record keyed by (session_id, timestamp).
//! The schema uses HashRange so all turns in a session can be retrieved via
//! `HashKey(session_id)` in chronological order.

use crate::fold_node::llm_query::types::ToolCallRecord;
use crate::fold_node::node::FoldNode;
use crate::schema::types::{
    DeclarativeSchemaDefinition, KeyConfig, KeyValue, Mutation, MutationType,
    SchemaType,
};
use crate::schema::SchemaState;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

const AI_CONVERSATIONS_SCHEMA: &str = "ai_conversations";

fn build_schema() -> DeclarativeSchemaDefinition {
    let mut schema = DeclarativeSchemaDefinition::new(
        AI_CONVERSATIONS_SCHEMA.to_string(),
        SchemaType::HashRange,
        Some(KeyConfig::new(
            Some("session_id".to_string()),
            Some("timestamp".to_string()),
        )),
        Some(vec![
            "session_id".to_string(),
            "timestamp".to_string(),
            "query".to_string(),
            "answer".to_string(),
            "tool_calls_json".to_string(),
        ]),
        None,
        None,
    );

    schema.field_classifications.insert("session_id".to_string(), vec!["word".to_string()]);
    schema.field_classifications.insert("timestamp".to_string(), vec!["date".to_string()]);
    schema.field_classifications.insert("query".to_string(), vec!["word".to_string()]);
    schema.field_classifications.insert("answer".to_string(), vec!["word".to_string()]);

    schema
}

async fn ensure_schema(node: &FoldNode) {
    let schema_manager = match node.get_fold_db().await {
        Ok(guard) => guard.schema_manager().clone(),
        Err(e) => {
            log::error!("Failed to get FoldDB for conversation schema: {}", e);
            return;
        }
    };

    // Fast path: already approved
    match schema_manager.get_schema_states() {
        Ok(states) => {
            if states.get(AI_CONVERSATIONS_SCHEMA) == Some(&SchemaState::Approved) {
                return;
            }
        }
        Err(e) => {
            log::error!("Failed to get schema states: {}", e);
            return;
        }
    }

    let schema = build_schema();
    let schema_json = match serde_json::to_string(&schema) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to serialize conversation schema: {}", e);
            return;
        }
    };

    if let Err(e) = schema_manager.load_schema_from_json(&schema_json).await {
        log::error!("Failed to load conversation schema: {}", e);
        return;
    }

    if let Err(e) = schema_manager.approve(AI_CONVERSATIONS_SCHEMA).await {
        log::error!("Failed to approve conversation schema: {}", e);
    }
}

pub async fn save_conversation_turn(
    node: &FoldNode,
    session_id: String,
    query: String,
    answer: String,
    tool_calls: Vec<ToolCallRecord>,
) {
    ensure_schema(node).await;

    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let tool_calls_json = serde_json::to_string(&tool_calls).unwrap_or_default();

    let mut fields: HashMap<String, serde_json::Value> = HashMap::new();
    fields.insert("session_id".to_string(), json!(session_id));
    fields.insert("timestamp".to_string(), json!(timestamp));
    fields.insert("query".to_string(), json!(query));
    fields.insert("answer".to_string(), json!(answer));
    fields.insert("tool_calls_json".to_string(), json!(tool_calls_json));

    let key_value = KeyValue::new(Some(session_id.clone()), Some(timestamp));

    let pub_key = node.get_node_public_key().to_string();

    let mutation = Mutation::new(
        AI_CONVERSATIONS_SCHEMA.to_string(),
        fields,
        key_value,
        pub_key,
        0,
        MutationType::Create,
    );

    match node.mutate_batch(vec![mutation]).await {
        Ok(ids) => log::info!(
            "Saved conversation turn for session {}: {:?}",
            session_id,
            ids
        ),
        Err(e) => log::error!(
            "Failed to save conversation turn for session {}: {}",
            session_id,
            e
        ),
    }
}
