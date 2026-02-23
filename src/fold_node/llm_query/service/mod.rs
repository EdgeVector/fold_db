//! LLM service for query analysis and summarization.

mod native_index;
mod parsers;
mod prompts;

use super::types::{FollowupAnalysis, Message, QueryPlan};
use crate::ingestion::{
    config::{AIProvider, IngestionConfig},
    ollama_service::OllamaService,
    openrouter_service::OpenRouterService,
};
use crate::schema::SchemaWithState;
use serde_json::Value;

/// Service for LLM-based query analysis and summarization
pub struct LlmQueryService {
    provider: AIProvider,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
}

impl LlmQueryService {
    /// Create a new LLM query service
    pub fn new(config: IngestionConfig) -> Result<Self, String> {
        let openrouter_service = if config.provider == AIProvider::OpenRouter {
            Some(
                OpenRouterService::new(
                    config.openrouter.clone(),
                    config.timeout_seconds,
                    config.max_retries,
                )
                .map_err(|e| format!("Failed to create OpenRouter service: {}", e))?,
            )
        } else {
            None
        };

        let ollama_service = if config.provider == AIProvider::Ollama {
            Some(
                OllamaService::new(
                    config.ollama.clone(),
                    config.timeout_seconds,
                    config.max_retries,
                )
                .map_err(|e| format!("Failed to create Ollama service: {}", e))?,
            )
        } else {
            None
        };

        Ok(Self {
            provider: config.provider,
            openrouter_service,
            ollama_service,
        })
    }

    /// Analyze a natural language query and create an execution plan
    pub async fn analyze_query(
        &self,
        user_query: &str,
        schemas: &[SchemaWithState],
    ) -> Result<QueryPlan, String> {
        let prompt = self.build_analysis_prompt(user_query, schemas);

        // Log prompt for debugging (truncated to avoid too much output)
        let prompt_preview = if prompt.len() > 500 {
            format!(
                "{}... [truncated, total {} chars]",
                &prompt[..500],
                prompt.len()
            )
        } else {
            prompt.clone()
        };
        log::debug!("AI Query Prompt Preview: {}", prompt_preview);

        let response = self.call_llm(&prompt).await?;

        let mut query_plan = self.parse_query_plan(&response)?;

        // Canonicalize schema name to ensure strict case match (backend is strict)
        // This handles AI hallucinations where it might output "Myschema" instead of "MySchema"
        let target_schema_lower = query_plan.query.schema_name.to_lowercase();
        for schema_state in schemas {
            if schema_state.schema.name.to_lowercase() == target_schema_lower {
                if query_plan.query.schema_name != schema_state.schema.name {
                    log::info!(
                        "🤖 AI Autocorrect: Normalizing schema name '{}' -> '{}'",
                        query_plan.query.schema_name,
                        schema_state.schema.name
                    );
                    query_plan.query.schema_name = schema_state.schema.name.clone();
                }
                break;
            }
        }

        Ok(query_plan)
    }

    /// Summarize query results
    pub async fn summarize_results(
        &self,
        original_query: &str,
        results: &[Value],
    ) -> Result<String, String> {
        let prompt = self.build_summarization_prompt(original_query, results);
        self.call_llm(&prompt).await
    }

    /// Answer a follow-up question based on context
    pub async fn answer_question(
        &self,
        original_query: &str,
        results: &[Value],
        conversation_history: &[Message],
        question: &str,
    ) -> Result<String, String> {
        let prompt =
            self.build_chat_prompt(original_query, results, conversation_history, question);
        self.call_llm(&prompt).await
    }

    /// Analyze if a follow-up question needs a new query or can be answered from existing results
    pub async fn analyze_followup_question(
        &self,
        original_query: &str,
        results: &[Value],
        question: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> Result<FollowupAnalysis, String> {
        let prompt =
            self.build_followup_analysis_prompt(original_query, results, question, schemas);
        let response = self.call_llm(&prompt).await?;
        self.parse_followup_analysis(&response)
    }

    /// Build the system prompt with tool definitions for the agent
    fn build_agent_system_prompt(&self, schemas: &[SchemaWithState]) -> String {
        let mut prompt = String::from(
            "You are a helpful database assistant with access to tools. Use the tools to query and manipulate data to answer the user's question.\n\n"
        );

        prompt.push_str("## Available Tools\n\n");

        prompt.push_str("### query\n");
        prompt.push_str("Query data from a schema.\n");
        prompt.push_str("Parameters:\n");
        prompt.push_str("- schema_name (string, required): Name of the schema to query\n");
        prompt.push_str("- fields (array of strings, optional): Fields to return. If omitted, returns all fields.\n");
        prompt.push_str("- filter (object, optional): Filter to apply. Examples:\n");
        prompt.push_str("  - {\"HashKey\": \"value\"} - exact match on hash key\n");
        prompt.push_str("  - {\"RangePrefix\": \"prefix\"} - prefix match on range key\n");
        prompt.push_str("  - {\"SampleN\": 10} - random sample of N records\n");
        prompt.push_str("  - null - no filter (all records)\n");
        prompt.push_str("Example: {\"tool\": \"query\", \"params\": {\"schema_name\": \"Tweet\", \"fields\": [\"content\", \"author\"], \"filter\": {\"SampleN\": 5}}}\n\n");

        prompt.push_str("### list_schemas\n");
        prompt.push_str("List all available schemas.\n");
        prompt.push_str("Parameters: none\n");
        prompt.push_str("Example: {\"tool\": \"list_schemas\", \"params\": {}}\n\n");

        prompt.push_str("### get_schema\n");
        prompt.push_str("Get details of a specific schema including its fields and key configuration.\n");
        prompt.push_str("Parameters:\n");
        prompt.push_str("- name (string, required): Name of the schema\n");
        prompt.push_str("Example: {\"tool\": \"get_schema\", \"params\": {\"name\": \"Tweet\"}}\n\n");

        prompt.push_str("### search\n");
        prompt.push_str("Full-text search across all indexed fields.\n");
        prompt.push_str("Parameters:\n");
        prompt.push_str("- terms (string, required): Search terms\n");
        prompt.push_str("Example: {\"tool\": \"search\", \"params\": {\"terms\": \"rust programming\"}}\n\n");

        prompt.push_str("## Available Schemas\n\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- **{}** (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  - Hash Key: {}\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  - Range Key: {}\n", range_field));
                }
            }

            prompt.push_str("  - Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\n## Instructions\n\n");
        prompt.push_str("1. Analyze the user's request\n");
        prompt.push_str("2. Use tools to gather information or perform actions\n");
        prompt.push_str("3. When you have enough information to answer, provide your final response\n\n");
        prompt.push_str("## Reference Fields\n\n");
        prompt.push_str("Some fields are References to records in other schemas. Query results automatically resolve references one level deep.\n");
        prompt.push_str("If a field value is an array of objects with \"schema\" and \"key\" properties, those are references to child records.\n");
        prompt.push_str("The referenced data will be included inline when available. If you need deeper data (references within references), ");
        prompt.push_str("use get_schema to find the child schema's fields, then use query to fetch the child schema's data directly.\n\n");
        prompt.push_str("IMPORTANT: Always respond with valid JSON. Either:\n");
        prompt.push_str("- {\"tool\": \"tool_name\", \"params\": {...}} to call a tool\n");
        prompt.push_str("- {\"answer\": \"your response\"} to provide the final answer\n");

        prompt
    }

    /// Call the LLM service
    pub(super) async fn call_llm(&self, prompt: &str) -> Result<String, String> {
        match self.provider {
            AIProvider::OpenRouter => {
                if let Some(ref service) = self.openrouter_service {
                    service
                        .call_openrouter_api(prompt)
                        .await
                        .map_err(|e| format!("OpenRouter API error: {}", e))
                } else {
                    Err("OpenRouter service not initialized".to_string())
                }
            }
            AIProvider::Ollama => {
                if let Some(ref service) = self.ollama_service {
                    service
                        .call_ollama_api(prompt)
                        .await
                        .map_err(|e| format!("Ollama API error: {}", e))
                } else {
                    Err("Ollama service not initialized".to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{
        DeclarativeSchemaDefinition, JsonTopology, KeyConfig, PrimitiveType, TopologyNode,
    };
    use crate::schema::{SchemaState, SchemaWithState};
    use std::collections::HashMap;

    fn create_test_hash_range_schema() -> SchemaWithState {
        let mut field_topologies = HashMap::new();
        field_topologies.insert(
            "author".to_string(),
            JsonTopology {
                root: TopologyNode::Primitive {
                    value: PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            },
        );
        field_topologies.insert(
            "publish_date".to_string(),
            JsonTopology {
                root: TopologyNode::Primitive {
                    value: PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            },
        );

        let mut schema = DeclarativeSchemaDefinition::new(
            "BlogPostAuthorIndex".to_string(),
            crate::schema::types::schema::DeclarativeSchemaType::HashRange,
            Some(KeyConfig {
                hash_field: Some("author".to_string()),
                range_field: Some("publish_date".to_string()),
            }),
            None, // fields
            None, // transform_fields
            None, // field_mappers
        );

        schema.descriptive_name = Some("Blog Post Author Index".to_string());
        schema.field_topologies = field_topologies;

        SchemaWithState {
            schema,
            state: SchemaState::Approved,
        }
    }

    #[test]
    fn test_prompt_includes_hash_and_range_keys() {
        let mut config = crate::ingestion::config::IngestionConfig::default();
        config.provider = crate::ingestion::config::AIProvider::Ollama;

        let service = LlmQueryService::new(config).expect("Failed to create service");
        let schemas = vec![create_test_hash_range_schema()];

        let prompt = service.build_analysis_prompt("Find posts by Jennifer Liu", &schemas);

        // Verify prompt includes hash key information
        assert!(
            prompt.contains("Hash Key: author"),
            "Prompt should include Hash Key field"
        );
        assert!(
            prompt.contains("Range Key: publish_date"),
            "Prompt should include Range Key field"
        );

        // Verify prompt includes filter guidance
        assert!(
            prompt.contains("HashKey"),
            "Prompt should mention HashKey filter"
        );
        assert!(
            prompt.contains("CRITICAL"),
            "Prompt should include critical filter selection guidance"
        );
        assert!(
            prompt.contains("Jennifer Liu"),
            "Prompt should include the example with Jennifer Liu"
        );
    }

    #[test]
    fn test_prompt_shows_correct_vs_incorrect_examples() {
        let mut config = crate::ingestion::config::IngestionConfig::default();
        config.provider = crate::ingestion::config::AIProvider::Ollama;

        let service = LlmQueryService::new(config).expect("Failed to create service");
        let schemas = vec![create_test_hash_range_schema()];

        let prompt = service.build_analysis_prompt("Test query", &schemas);

        // Verify prompt includes correct examples
        assert!(
            prompt.contains("✓ CORRECT"),
            "Prompt should show correct examples"
        );
    }
}
