//! Types for AI query functionality

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context for stateless follow-up queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    pub original_query: String,
    pub query_results: Vec<Value>,
    pub conversation_history: Vec<ConversationMessage>,
    pub query_plan: Option<QueryPlanInfo>,
}

/// Conversation message for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

/// Query plan information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanInfo {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub filter_type: Option<String>,
    pub reasoning: String,
}

/// Response from AI query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIQueryResponse {
    pub ai_interpretation: String,
    pub raw_results: Vec<Value>,
    pub context: QueryContext,
}

/// Complete query response with planning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteQueryResponse {
    pub query_plan: QueryPlanInfo,
    pub results: Vec<Value>,
    pub summary: Option<String>,
    pub context: QueryContext,
}

/// Request for follow-up question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowupRequest {
    pub context: QueryContext,
    pub question: String,
}

/// Response from follow-up
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowupResponse {
    pub answer: String,
    pub executed_new_query: bool,
    pub context: QueryContext,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_context_serialization() {
        let context = QueryContext {
            original_query: "test query".to_string(),
            query_results: vec![serde_json::json!({"key": "value"})],
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: 1234567890,
                }
            ],
            query_plan: None,
        };
        
        // Should serialize and deserialize without errors
        let json = serde_json::to_string(&context).unwrap();
        let deserialized: QueryContext = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.original_query, "test query");
        assert_eq!(deserialized.conversation_history.len(), 1);
        assert_eq!(deserialized.conversation_history[0].role, "user");
    }

    #[test]
    fn test_followup_request_serialization() {
        let context = QueryContext {
            original_query: "original".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: None,
        };
        
        let request = FollowupRequest {
            context,
            question: "follow-up question".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: FollowupRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.question, "follow-up question");
        assert_eq!(deserialized.context.original_query, "original");
    }

    #[test]
    fn test_ai_query_response_structure() {
        let context = QueryContext {
            original_query: "test".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: None,
        };
        
        let response = AIQueryResponse {
            ai_interpretation: "AI response".to_string(),
            raw_results: vec![serde_json::json!({"test": "data"})],
            context,
        };
        
        assert_eq!(response.ai_interpretation, "AI response");
        assert_eq!(response.raw_results.len(), 1);
        assert_eq!(response.context.original_query, "test");
    }

    #[test]
    fn test_complete_query_response_structure() {
        let query_plan = QueryPlanInfo {
            schema_name: "TestSchema".to_string(),
            fields: vec!["field1".to_string(), "field2".to_string()],
            filter_type: Some("HashKey".to_string()),
            reasoning: "Test reasoning".to_string(),
        };
        
        let context = QueryContext {
            original_query: "test".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: Some(query_plan.clone()),
        };
        
        let response = CompleteQueryResponse {
            query_plan,
            results: vec![],
            summary: Some("Test summary".to_string()),
            context,
        };
        
        assert_eq!(response.query_plan.schema_name, "TestSchema");
        assert_eq!(response.summary, Some("Test summary".to_string()));
        assert!(response.context.query_plan.is_some());
    }

    #[test]
    fn test_conversation_message_timestamp() {
        let msg = ConversationMessage {
            role: "assistant".to_string(),
            content: "response".to_string(),
            timestamp: 1700000000,
        };
        
        assert_eq!(msg.timestamp, 1700000000);
        assert_eq!(msg.role, "assistant");
    }

    #[test]
    fn test_query_plan_info_serialization() {
        let plan = QueryPlanInfo {
            schema_name: "BlogPost".to_string(),
            fields: vec!["title".to_string(), "content".to_string()],
            filter_type: Some("RangePrefix".to_string()),
            reasoning: "Using BlogPost schema for efficiency".to_string(),
        };
        
        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: QueryPlanInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.schema_name, "BlogPost");
        assert_eq!(deserialized.fields.len(), 2);
        assert_eq!(deserialized.filter_type, Some("RangePrefix".to_string()));
    }

    #[test]
    fn test_followup_response_with_new_query() {
        let context = QueryContext {
            original_query: "original".to_string(),
            query_results: vec![serde_json::json!({"new": "data"})],
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: "question".to_string(),
                    timestamp: 1234567890,
                },
                ConversationMessage {
                    role: "assistant".to_string(),
                    content: "answer".to_string(),
                    timestamp: 1234567891,
                },
            ],
            query_plan: None,
        };
        
        let response = FollowupResponse {
            answer: "Here's the answer".to_string(),
            executed_new_query: true,
            context,
        };
        
        assert_eq!(response.executed_new_query, true);
        assert_eq!(response.context.conversation_history.len(), 2);
        assert_eq!(response.context.query_results.len(), 1);
    }
}
