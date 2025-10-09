//! LLM service for query analysis and summarization.

use super::types::{QueryPlan, Message};
use crate::ingestion::{
    config::{AIProvider, IngestionConfig},
    ollama_service::OllamaService,
    openrouter_service::OpenRouterService,
};
use crate::schema::types::{DeclarativeSchemaDefinition, Query};
use crate::schema::{SchemaWithState};
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
        let response = self.call_llm(&prompt).await?;
        self.parse_query_plan(&response)
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
        let prompt = self.build_chat_prompt(original_query, results, conversation_history, question);
        self.call_llm(&prompt).await
    }

    /// Build the analysis prompt
    fn build_analysis_prompt(&self, user_query: &str, schemas: &[SchemaWithState]) -> String {
        let mut prompt = String::from(
            "You are a database query optimizer. Analyze the following natural language query \
            and available schemas to create an execution plan.\n\n"
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));
            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Determine:\n\
            1. Which schema(s) to query\n\
            2. What fields to retrieve\n\
            3. What filters to apply (if any)\n\
            4. If an index is needed (consider element count > 10,000 as threshold)\n\n\
            FILTER TYPES AVAILABLE:\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match\n\
            - HashKey: {\"HashKey\": \"value\"} - all records with this hash\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - all records with range starting with prefix\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}}\n\
            - Value: {\"Value\": \"search_term\"} - search across all values\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records (NOT sorted)\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern matching\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - hash glob pattern\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - range of values\n\
            - null - no filter (return all records)\n\n\
            IMPORTANT FILTER NOTES:\n\
            - SampleN returns RANDOM records, NOT sorted or ordered\n\
            - For \"most recent\" or \"latest\" queries, use null filter to get all records (backend will handle sorting)\n\
            - For date ranges, use RangeRange with start/end dates\n\
            - Range keys are stored as strings and compared lexicographically\n\n\
            EXAMPLES:\n\
            - Search for word \"ai\" in BlogPostWordIndex: {\"HashKey\": \"ai\"}\n\
            - Get blog post by ID: {\"RangePrefix\": \"post-123\"}\n\
            - Get most recent posts: null (returns all, sorted by backend)\n\
            - Get posts in date range: {\"RangeRange\": {\"start\": \"2025-09-01\", \"end\": \"2025-09-30\"}}\n\
            - Search for \"technology\" anywhere: {\"Value\": \"technology\"}\n\n\
            Respond in JSON format with:\n\
            {\n\
              \"query\": {\n\
                \"schema_name\": \"string\",\n\
                \"fields\": [\"field1\", \"field2\"],\n\
                \"filter\": null or one of the filter types above\n\
              },\n\
              \"index_schema\": null,\n\
              \"reasoning\": \"your analysis\"\n\
            }\n\n\
            IMPORTANT: \n\
            - Return ONLY the JSON object, no additional text\n\
            - Use the EXACT filter format shown above\n\
            - ALWAYS set index_schema to null (index creation is not yet supported)\n\
            - For \"most recent\", \"latest\", or \"newest\" queries, use null filter (NOT SampleN)\n\
            - Choose the most efficient existing schema for the query"
        );

        prompt
    }

    /// Build the summarization prompt
    fn build_summarization_prompt(&self, original_query: &str, results: &[Value]) -> String {
        let results_preview = if results.len() > 1000 {
            &results[..1000]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        format!(
            "Summarize the following query results for the user.\n\n\
            Original Query: {}\n\
            Results ({} total): {}\n\n\
            Provide:\n\
            1. High-level summary\n\
            2. Key insights\n\
            3. Notable patterns or anomalies\n\n\
            Keep the summary concise and informative.",
            original_query,
            results.len(),
            results_str
        )
    }

    /// Build the chat prompt for follow-up questions
    fn build_chat_prompt(
        &self,
        original_query: &str,
        results: &[Value],
        conversation_history: &[Message],
        question: &str,
    ) -> String {
        let results_preview = if results.len() > 1000 {
            &results[..1000]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        let mut prompt = String::from(
            "You are helping a user explore query results. Answer their question based on \
            the context provided.\n\n"
        );

        prompt.push_str(&format!("Original Query: {}\n", original_query));
        prompt.push_str(&format!("Results ({} total): {}\n\n", results.len(), results_str));

        if !conversation_history.is_empty() {
            prompt.push_str("Conversation History:\n");
            for msg in conversation_history {
                prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("User Question: {}\n\n", question));
        prompt.push_str("Provide a clear, concise answer based on the data.");

        prompt
    }

    /// Call the LLM service
    async fn call_llm(&self, prompt: &str) -> Result<String, String> {
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

    /// Parse the LLM response into a QueryPlan
    fn parse_query_plan(&self, response: &str) -> Result<QueryPlan, String> {
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

        let parsed: LlmResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse LLM response: {}. Response: {}", e, json_str))?;

        Ok(QueryPlan {
            query: parsed.query,
            index_schema: parsed.index_schema,
            reasoning: parsed.reasoning,
        })
    }
}

