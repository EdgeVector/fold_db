//! Type definitions for LLM query workflow.

use crate::schema::types::DeclarativeSchemaDefinition;
use crate::schema::types::Query;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

/// Request to analyze a natural language query
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyzeQueryRequest {
    pub query: String,
    pub session_id: Option<String>,
}

/// Response from query analysis
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyzeQueryResponse {
    pub session_id: String,
    pub query_plan: QueryPlan,
}

/// The plan for executing a query
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QueryPlan {
    pub query: Query,
    pub index_schema: Option<DeclarativeSchemaDefinition>,
    pub reasoning: String,
}

/// Request to execute a query plan
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExecuteQueryPlanRequest {
    pub session_id: String,
    pub query_plan: QueryPlan,
}

/// Status of query execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryExecutionStatus {
    Pending,
    Running,
    Complete,
}

/// Response from query execution
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExecuteQueryPlanResponse {
    pub status: QueryExecutionStatus,
    pub backfill_progress: Option<f64>,
    pub results: Option<Vec<serde_json::Value>>,
    pub summary: Option<String>,
}

/// Request for follow-up question
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    pub session_id: String,
    pub question: String,
}

/// Response to follow-up question
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatResponse {
    pub answer: String,
    pub context_used: bool,
}

/// Backfill status response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BackfillStatusResponse {
    pub status: String,
    pub progress: f64,
    pub total_records: u64,
    pub processed_records: u64,
    pub estimated_completion: Option<String>,
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: SystemTime,
}

/// Analysis of whether a followup question needs a new query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowupAnalysis {
    pub needs_query: bool,
    pub query: Option<Query>,
    pub reasoning: String,
}

/// Request to run a query (single-step analyze and execute)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RunQueryRequest {
    pub query: String,
    pub session_id: Option<String>,
}

/// Session context stored for each user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub original_query: String,
    pub query_results: Option<Vec<serde_json::Value>>,
    pub conversation_history: Vec<Message>,
    pub schema_created: Option<String>,
    pub ttl_seconds: u64,
}

// ============================================================================
// Agent Query Types
// ============================================================================

/// Request for agent query - an autonomous LLM agent that can use tools
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentQueryRequest {
    /// The natural language query or task for the agent
    pub query: String,
    /// Optional session ID for conversation continuity
    pub session_id: Option<String>,
    /// Maximum number of iterations before stopping (default: 10)
    pub max_iterations: Option<usize>,
}

/// Response from agent query
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentQueryResponse {
    /// The final answer from the agent
    pub answer: String,
    /// Record of all tool calls made during execution
    pub tool_calls: Vec<ToolCallRecord>,
    /// Session ID for follow-up queries
    pub session_id: String,
}

/// Record of a single tool call made by the agent
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ToolCallRecord {
    /// Name of the tool called
    pub tool: String,
    /// Parameters passed to the tool
    pub params: Value,
    /// Result returned from the tool
    pub result: Value,
}

/// Parsed LLM response - either a tool call or a final answer
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// The LLM wants to call a tool
    ToolCall { tool: String, params: Value },
    /// The LLM has a final answer
    Answer(String),
}

impl SessionContext {
    pub fn new(session_id: String, original_query: String) -> Self {
        let now = SystemTime::now();
        Self {
            session_id,
            created_at: now,
            last_active: now,
            original_query,
            query_results: None,
            conversation_history: Vec::new(),
            schema_created: None,
            ttl_seconds: 3600, // 1 hour default
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Ok(duration) = SystemTime::now().duration_since(self.last_active) {
            duration.as_secs() > self.ttl_seconds
        } else {
            true
        }
    }

    pub fn update_activity(&mut self) {
        self.last_active = SystemTime::now();
    }

    pub fn add_message(&mut self, role: String, content: String) {
        self.conversation_history.push(Message {
            role,
            content,
            timestamp: SystemTime::now(),
        });
    }
}
