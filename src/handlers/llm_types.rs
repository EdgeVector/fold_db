use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Response for chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHandlerResponse {
    pub answer: String,
    pub context_used: bool,
}

/// Response for analyze followup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFollowupHandlerResponse {
    pub needs_query: bool,
    pub query: Option<crate::schema::types::Query>,
    pub reasoning: String,
}

/// Response for AI native index query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiNativeIndexHandlerResponse {
    pub ai_interpretation: String,
    pub raw_results: Vec<Value>,
    pub query: String,
    pub session_id: String,
}

/// Request for agent query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryHandlerRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub max_iterations: Option<usize>,
}

/// Response for agent query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryHandlerResponse {
    pub answer: String,
    pub tool_calls: Vec<crate::fold_node::llm_query::types::ToolCallRecord>,
    pub session_id: String,
}
