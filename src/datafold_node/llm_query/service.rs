//! LLM service for query analysis and summarization.

use super::types::{QueryPlan, Message, FollowupAnalysis};
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

    /// Analyze if a follow-up question needs a new query or can be answered from existing results
    pub async fn analyze_followup_question(
        &self,
        original_query: &str,
        results: &[Value],
        question: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> Result<FollowupAnalysis, String> {
        let prompt = self.build_followup_analysis_prompt(original_query, results, question, schemas);
        let response = self.call_llm(&prompt).await?;
        self.parse_followup_analysis(&response)
    }

    /// Build prompt to analyze if a followup needs a new query
    fn build_followup_analysis_prompt(
        &self,
        original_query: &str,
        results: &[Value],
        question: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let results_preview = if results.len() > 100 {
            &results[..100]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        let mut prompt = String::from(
            "You are analyzing whether a follow-up question can be answered from existing query results or needs a new query.\n\n"
        );

        prompt.push_str(&format!("Original Query: {}\n", original_query));
        prompt.push_str(&format!("Existing Results ({} total): {}\n\n", results.len(), results_str));
        prompt.push_str(&format!("Follow-up Question: {}\n\n", question));

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?})\n",
                schema.schema.name, schema.schema.schema_type
            ));
            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nDetermine if:\n");
        prompt.push_str("1. The question can be FULLY answered from the existing results (needs_query: false)\n");
        prompt.push_str("2. The question needs NEW data that requires a query (needs_query: true)\n\n");

        prompt.push_str("If a new query is needed, provide:\n");
        prompt.push_str("- query: The Query object to execute (same format as before)\n");
        prompt.push_str("- reasoning: Why a new query is needed\n\n");

        prompt.push_str(
            "FILTER TYPES AVAILABLE:\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match\n\
            - HashKey: {\"HashKey\": \"value\"} - all records with this hash\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - all records with range starting with prefix\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}}\n\
            - Value: {\"Value\": \"search_term\"} - search across all values\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records (NOT sorted)\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern matching\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - hash glob pattern\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - range of values\n\
            - null - no filter (return all records)\n\n"
        );

        prompt.push_str(
            "Respond in JSON format:\n\
            {\n\
              \"needs_query\": true/false,\n\
              \"query\": null or {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ...},\n\
              \"reasoning\": \"explanation\"\n\
            }\n\n\
            IMPORTANT: Return ONLY the JSON object, no additional text."
        );

        prompt
    }

    /// Parse the followup analysis response
    fn parse_followup_analysis(&self, response: &str) -> Result<FollowupAnalysis, String> {
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

        let parsed: LlmFollowupResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse followup analysis: {}. Response: {}", e, json_str))?;

        Ok(FollowupAnalysis {
            needs_query: parsed.needs_query,
            query: parsed.query,
            reasoning: parsed.reasoning,
        })
    }

    /// Suggest alternative query strategies when results are empty
    pub async fn suggest_alternative_query(
        &self,
        original_user_query: &str,
        failed_query: &Query,
        schemas: &[crate::schema::SchemaWithState],
        previous_attempts: &[String],
    ) -> Result<Option<QueryPlan>, String> {
        let prompt = self.build_alternative_query_prompt(
            original_user_query,
            failed_query,
            schemas,
            previous_attempts,
        );
        let response = self.call_llm(&prompt).await?;
        self.parse_alternative_query(&response)
    }

    /// Build prompt to suggest alternative query strategies
    fn build_alternative_query_prompt(
        &self,
        original_user_query: &str,
        failed_query: &Query,
        schemas: &[crate::schema::SchemaWithState],
        previous_attempts: &[String],
    ) -> String {
        let mut prompt = String::from(
            "A query returned no results. Suggest an alternative approach to find the data the user wants.\n\n"
        );

        prompt.push_str(&format!("User's Original Question: {}\n\n", original_user_query));
        
        prompt.push_str("Failed Query:\n");
        prompt.push_str(&format!("  Schema: {}\n", failed_query.schema_name));
        prompt.push_str(&format!("  Fields: {:?}\n", failed_query.fields));
        prompt.push_str(&format!("  Filter: {:?}\n\n", failed_query.filter));

        if !previous_attempts.is_empty() {
            prompt.push_str("Previous Failed Attempts:\n");
            for (i, attempt) in previous_attempts.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, attempt));
            }
            prompt.push('\n');
        }

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));
            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nSuggest ONE alternative approach:\n");
        prompt.push_str("1. Try a different schema that might have the data\n");
        prompt.push_str("2. Broaden the filter (e.g., remove date constraints, use pattern matching)\n");
        prompt.push_str("3. Try a different filter type (e.g., null filter for all records)\n");
        prompt.push_str("4. Search in related/index schemas\n\n");

        prompt.push_str("If you believe there are NO reasonable alternatives left, respond with:\n");
        prompt.push_str("{\"has_alternative\": false, \"query\": null, \"reasoning\": \"explanation\"}\n\n");

        prompt.push_str("Otherwise, respond with:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"has_alternative\": true,\n");
        prompt.push_str("  \"query\": {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ...},\n");
        prompt.push_str("  \"reasoning\": \"why this approach might work\"\n");
        prompt.push_str("}\n\n");

        prompt.push_str(
            "FILTER TYPES:\n\
            - HashRangeKey, HashKey, RangePrefix, HashRangePrefix, Value, SampleN, \n\
            - RangePattern, HashPattern, RangeRange, null (all records)\n\n\
            IMPORTANT: Return ONLY the JSON object."
        );

        prompt
    }

    /// Parse alternative query response
    fn parse_alternative_query(&self, response: &str) -> Result<Option<QueryPlan>, String> {
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

        let parsed: LlmAlternativeResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse alternative query: {}. Response: {}", e, json_str))?;

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
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
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

