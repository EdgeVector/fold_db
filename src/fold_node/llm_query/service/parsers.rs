//! LLM response parsers for query plans, followup analysis, and alternatives.

use super::super::types::{AgentAction, FollowupAnalysis, QueryPlan};
use crate::schema::types::{DeclarativeSchemaDefinition, Query};
use serde_json::Value;

use super::LlmQueryService;

impl LlmQueryService {
    /// Parse the LLM response into a QueryPlan
    pub(super) fn parse_query_plan(&self, response: &str) -> Result<QueryPlan, String> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        #[derive(serde::Deserialize)]
        struct LlmResponse {
            query: Query,
            index_schema: Option<DeclarativeSchemaDefinition>,
            reasoning: String,
        }

        let parsed: LlmResponse = serde_json::from_str(json_str).map_err(|e| {
            format!(
                "Failed to parse LLM response: {}. Response: {}",
                e, json_str
            )
        })?;

        Ok(QueryPlan {
            query: parsed.query,
            index_schema: parsed.index_schema,
            reasoning: parsed.reasoning,
        })
    }

    /// Parse the followup analysis response
    pub(super) fn parse_followup_analysis(&self, response: &str) -> Result<FollowupAnalysis, String> {
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        #[derive(serde::Deserialize)]
        struct LlmFollowupResponse {
            needs_query: bool,
            query: Option<Query>,
            reasoning: String,
        }

        let parsed: LlmFollowupResponse = serde_json::from_str(json_str).map_err(|e| {
            format!(
                "Failed to parse followup analysis: {}. Response: {}",
                e, json_str
            )
        })?;

        Ok(FollowupAnalysis {
            needs_query: parsed.needs_query,
            query: parsed.query,
            reasoning: parsed.reasoning,
        })
    }

    /// Parse the query terms response
    pub(super) fn parse_query_terms_response(&self, response: &str) -> Result<Vec<String>, String> {
        // Try to extract JSON array from the response
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let terms: Vec<String> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse query terms: {}. Response: {}", e, json_str))?;

        if terms.is_empty() {
            return Err("No query terms generated".to_string());
        }

        Ok(terms)
    }

    /// Parse alternative query response
    pub(super) fn parse_alternative_query(&self, response: &str) -> Result<Option<QueryPlan>, String> {
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        #[derive(serde::Deserialize)]
        struct LlmAlternativeResponse {
            has_alternative: bool,
            query: Option<Query>,
            reasoning: String,
        }

        let parsed: LlmAlternativeResponse = serde_json::from_str(json_str).map_err(|e| {
            format!(
                "Failed to parse alternative query: {}. Response: {}",
                e, json_str
            )
        })?;

        if parsed.has_alternative {
            if let Some(query) = parsed.query {
                Ok(Some(QueryPlan {
                    query,
                    index_schema: None,
                    reasoning: parsed.reasoning,
                }))
            } else {
                Err("has_alternative is true but no query provided".to_string())
            }
        } else {
            Ok(None)
        }
    }

    /// Extract the first complete JSON object from a string by tracking brace depth.
    /// Returns None if no complete object is found.
    fn extract_first_json_object(text: &str) -> Option<&str> {
        let start = text.find('{')?;
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escape_next = false;
        for (i, ch) in text[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&text[start..start + i + 1]);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Parse an LLM response into an AgentAction
    pub(super) fn parse_agent_response(&self, response: &str) -> Result<AgentAction, String> {
        // Extract the first complete JSON object (not first '{' to last '}')
        let json_str = match Self::extract_first_json_object(response) {
            Some(s) => s,
            None => {
                // No JSON object found - treat entire response as a plain-text answer
                return Ok(AgentAction::Answer(response.trim().to_string()));
            }
        };

        // Try parsing as-is first; if that fails, sanitize control characters
        // inside string values (LLMs sometimes put raw newlines inside JSON strings)
        let parsed: Value = match serde_json::from_str(json_str).or_else(|_| {
            let sanitized = json_str
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            serde_json::from_str::<Value>(&sanitized)
        }) {
            Ok(v) => v,
            Err(_) => {
                // JSON parsing failed entirely - treat response as plain-text answer
                return Ok(AgentAction::Answer(response.trim().to_string()));
            }
        };

        // Check if it's a tool call
        if let Some(tool) = parsed.get("tool").and_then(|t| t.as_str()) {
            let params = parsed.get("params").cloned().unwrap_or(Value::Object(serde_json::Map::new()));
            return Ok(AgentAction::ToolCall {
                tool: tool.to_string(),
                params,
            });
        }

        // Check if it's a final answer
        if let Some(answer) = parsed.get("answer").and_then(|a| a.as_str()) {
            return Ok(AgentAction::Answer(answer.to_string()));
        }

        Err(format!("Agent response must contain either 'tool' or 'answer' field. Got: {}", json_str))
    }
}
