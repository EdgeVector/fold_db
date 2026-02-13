//! LLM service for query analysis and summarization.

use super::types::{AgentAction, FollowupAnalysis, Message, QueryPlan, ToolCallRecord};
use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::ingestion::{
    config::{AIProvider, IngestionConfig},
    ollama_service::OllamaService,
    openrouter_service::OpenRouterService,
};
use crate::schema::types::{DeclarativeSchemaDefinition, Query};
use crate::schema::SchemaWithState;
use serde_json::Value;
use std::collections::HashSet;

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

    // ========================================================================
    // Agent Methods
    // ========================================================================

    /// Run an autonomous agent query that can use tools to accomplish tasks
    ///
    /// The agent will iteratively:
    /// 1. Send the conversation to the LLM
    /// 2. Parse the response for tool calls or final answer
    /// 3. Execute tool calls and add results to conversation
    /// 4. Repeat until a final answer is given or max_iterations reached
    pub async fn run_agent_query(
        &self,
        user_query: &str,
        schemas: &[SchemaWithState],
        node: &FoldNode,
        _user_hash: &str,
        max_iterations: usize,
    ) -> Result<(String, Vec<ToolCallRecord>), String> {
        let mut tool_calls: Vec<ToolCallRecord> = Vec::new();
        let mut conversation_context = String::new();

        // Build the initial system prompt with tool definitions
        let system_prompt = self.build_agent_system_prompt(schemas);

        log::info!(
            "Agent: Starting query with max {} iterations: {}",
            max_iterations,
            user_query
        );

        for iteration in 0..max_iterations {
            // Build the full prompt with conversation history
            let full_prompt = format!(
                "{}\n\n{}\n\nUser Query: {}\n\nRespond with a JSON object. Either:\n- {{\"tool\": \"tool_name\", \"params\": {{...}}}} to use a tool\n- {{\"answer\": \"your final response\"}} when you have the answer",
                system_prompt,
                conversation_context,
                user_query
            );

            log::debug!("Agent: Iteration {} - calling LLM", iteration + 1);

            let response = self.call_llm(&full_prompt).await?;

            log::debug!("Agent: LLM response: {}", &response[..response.len().min(200)]);

            // Parse the response
            let action = self.parse_agent_response(&response)?;

            match action {
                AgentAction::Answer(answer) => {
                    log::info!(
                        "Agent: Completed after {} iterations with {} tool calls",
                        iteration + 1,
                        tool_calls.len()
                    );
                    return Ok((answer, tool_calls));
                }
                AgentAction::ToolCall { tool, params } => {
                    log::info!("Agent: Calling tool '{}' with params: {}", tool, params);

                    // Execute the tool, capturing errors as results so the agent can retry
                    let result = match self.execute_tool(&tool, &params, node).await {
                        Ok(val) => val,
                        Err(e) => {
                            log::warn!("Agent: Tool '{}' failed: {}", tool, e);
                            serde_json::json!({ "error": e })
                        }
                    };

                    log::debug!("Agent: Tool '{}' returned: {}", tool, &result.to_string()[..result.to_string().len().min(200)]);

                    // Record the tool call
                    tool_calls.push(ToolCallRecord {
                        tool: tool.clone(),
                        params: params.clone(),
                        result: result.clone(),
                    });

                    // Add to conversation context
                    conversation_context.push_str(&format!(
                        "\n\nTool call: {}\nParameters: {}\nResult: {}\n",
                        tool,
                        serde_json::to_string_pretty(&params).unwrap_or_default(),
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    ));
                }
            }
        }

        Err(format!(
            "Agent reached maximum iterations ({}) without providing a final answer",
            max_iterations
        ))
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
        prompt.push_str("IMPORTANT: Always respond with valid JSON. Either:\n");
        prompt.push_str("- {\"tool\": \"tool_name\", \"params\": {...}} to call a tool\n");
        prompt.push_str("- {\"answer\": \"your response\"} to provide the final answer\n");

        prompt
    }

    /// Parse an LLM response into an AgentAction
    fn parse_agent_response(&self, response: &str) -> Result<AgentAction, String> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            // No JSON object found - treat entire response as a plain-text answer
            return Ok(AgentAction::Answer(response.trim().to_string()));
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

    /// Execute a tool call and return the result
    async fn execute_tool(
        &self,
        tool: &str,
        params: &Value,
        node: &FoldNode,
    ) -> Result<Value, String> {
        let processor = OperationProcessor::new(node.clone());

        match tool {
            "query" => {
                let schema_name = params
                    .get("schema_name")
                    .and_then(|s| s.as_str())
                    .ok_or("query tool requires 'schema_name' parameter")?;

                let fields: Vec<String> = params
                    .get("fields")
                    .and_then(|f| f.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let filter = params.get("filter").cloned();

                let query = Query {
                    schema_name: schema_name.to_string(),
                    fields,
                    filter: filter.and_then(|f| serde_json::from_value(f).ok()),
                    as_of: None,
                };

                let results = processor
                    .execute_query_json(query)
                    .await
                    .map_err(|e| format!("Query execution failed: {}", e))?;

                Ok(Value::Array(results))
            }

            "list_schemas" => {
                let schemas = processor
                    .list_schemas()
                    .await
                    .map_err(|e| format!("Failed to list schemas: {}", e))?;

                serde_json::to_value(&schemas)
                    .map_err(|e| format!("Failed to serialize schemas: {}", e))
            }

            "get_schema" => {
                let name = params
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or("get_schema tool requires 'name' parameter")?;

                let schema = processor
                    .get_schema(name)
                    .await
                    .map_err(|e| format!("Failed to get schema: {}", e))?;

                match schema {
                    Some(s) => serde_json::to_value(&s)
                        .map_err(|e| format!("Failed to serialize schema: {}", e)),
                    None => Ok(Value::Null),
                }
            }

            "search" => {
                let terms = params
                    .get("terms")
                    .and_then(|t| t.as_str())
                    .ok_or("search tool requires 'terms' parameter")?;

                let results = processor
                    .native_index_search(terms)
                    .await
                    .map_err(|e| format!("Search failed: {}", e))?;

                serde_json::to_value(&results)
                    .map_err(|e| format!("Failed to serialize search results: {}", e))
            }

            _ => Err(format!("Unknown tool: {}", tool)),
        }
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

    /// Generate query terms for native index search based on a natural language query
    pub async fn generate_native_index_query_terms(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> Result<Vec<String>, String> {
        let prompt = self.build_native_index_query_terms_prompt(user_query, schemas);
        let response = self.call_llm(&prompt).await?;
        self.parse_query_terms_response(&response)
    }

    /// Execute a complete AI-native index query workflow
    pub async fn execute_ai_native_index_query(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<String, String> {
        // Step 1: Generate native index search terms using AI
        let search_terms = self
            .generate_native_index_search_terms(user_query, schemas)
            .await?;

        // Step 2: Execute native index searches for each term
        let mut all_results = Vec::new();
        if let Some(native_index_mgr) = db_ops.native_index_manager() {
            for term in &search_terms {
                match native_index_mgr
                    .search_all_classifications(term)
                    .await
                {
                    Ok(mut results) => {
                        log::debug!(
                            "LLM Query: Term '{}' returned {} results",
                            term,
                            results.len()
                        );
                        all_results.append(&mut results);
                    }
                    Err(e) => {
                        log::warn!("Native index search failed for term '{}': {}", term, e);
                    }
                }
            }
        }

        log::info!(
            "LLM Query: Collected {} total results for AI interpretation",
            all_results.len()
        );

        // Step 3: Send results to AI for interpretation
        self.interpret_native_index_results(user_query, &all_results)
            .await
    }

    /// Search the native index and return deduplicated results (without AI interpretation)
    ///
    /// This is the first step of the AI-native index query workflow.
    /// Call `interpret_native_index_results` separately to get AI interpretation.
    pub async fn search_native_index(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<Vec<crate::db_operations::IndexResult>, String> {
        // Step 1: Generate native index search terms using AI
        let search_terms = self
            .generate_native_index_search_terms(user_query, schemas)
            .await?;

        // Step 2: Execute native index searches for each term
        let mut all_results = Vec::new();
        if let Some(native_index_mgr) = db_ops.native_index_manager() {
            for term in &search_terms {
                match native_index_mgr
                    .search_all_classifications(term)
                    .await
                {
                    Ok(mut results) => {
                        log::debug!(
                            "LLM Query: Term '{}' returned {} results",
                            term,
                            results.len()
                        );
                        all_results.append(&mut results);
                    }
                    Err(e) => {
                        log::warn!("Native index search failed for term '{}': {}", term, e);
                    }
                }
            }
        }

        log::debug!(
            "LLM Query: Total results before deduplication: {}",
            all_results.len()
        );

        // Step 2.5: Deduplicate results based on schema_name + key_value + field
        let deduplicated_results = self.deduplicate_results(all_results);

        log::info!(
            "LLM Query: Found {} deduplicated results from native index",
            deduplicated_results.len()
        );

        Ok(deduplicated_results)
    }

    /// Execute a complete AI-native index query workflow and return both AI interpretation and raw results
    pub async fn execute_ai_native_index_query_with_results(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<(String, Vec<crate::db_operations::IndexResult>), String> {
        // Search the native index
        let deduplicated_results = self.search_native_index(user_query, schemas, db_ops).await?;

        log::info!(
            "LLM Query: Sending {} results to AI for interpretation",
            deduplicated_results.len()
        );

        // Send results to AI for interpretation
        let ai_interpretation = self
            .interpret_native_index_results(user_query, &deduplicated_results)
            .await?;

        Ok((ai_interpretation, deduplicated_results))
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
        prompt.push_str(&format!(
            "Existing Results ({} total): {}\n\n",
            results.len(),
            results_str
        ));
        prompt.push_str(&format!("Follow-up Question: {}\n\n", question));

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?})\n",
                schema.schema.name, schema.schema.schema_type
            ));

            // Include key configuration for Range and HashRange schemas
            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nDetermine if:\n");
        prompt.push_str("1. The question can be FULLY answered from the existing results (needs_query: false)\n");
        prompt.push_str(
            "2. The question needs NEW data that requires a query (needs_query: true)\n\n",
        );

        prompt.push_str("If a new query is needed, provide:\n");
        prompt.push_str("- query: The Query object to execute (same format as before)\n");
        prompt.push_str("- reasoning: Why a new query is needed\n\n");

        prompt.push_str(
            "FILTER TYPES AVAILABLE:\n\n\
            Filters for HashRange schemas (have both Hash Key and Range Key):\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match on BOTH hash key field AND range key field\n\
            - HashKey: {\"HashKey\": \"value\"} - filter on hash key field only\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}} - filter on hash key field + range key field prefix\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - glob pattern on hash key field\n\n\
            Filters for Range schemas (have Range Key only):\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - filter on range key field\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern on range key field\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - filter on range key field\n\n\
            Universal filters (work on any schema type):\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records\n\
            - null - no filter (return all records)\n\n\
            IMPORTANT JSON FORMATTING:\n\
            - All filter string values must use proper JSON format\n\
            - Special characters like @ # $ are valid in JSON strings without escaping\n\
            - Example: {\"HashKey\": \"@techinfluencer\"} is correct\n\n\
            CRITICAL: Always use key-based filters (HashKey, RangePrefix, etc.).\n\
            Check each schema's Hash Key and Range Key fields to determine which filter to use.\n\
            Example: If searching for author \"Jennifer Liu\" and schema has hash_field=author, use {\"HashKey\": \"Jennifer Liu\"}.\n\n"
        );

        prompt.push_str(
            "Respond in JSON format:\n\
            {\n\
              \"needs_query\": true/false,\n\
              \"query\": null or {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ...},\n\
              \"reasoning\": \"explanation\"\n\
            }\n\n\
            IMPORTANT: Return ONLY the JSON object, no additional text.",
        );

        prompt
    }

    /// Build prompt to generate native index query terms
    fn build_native_index_query_terms_prompt(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let mut prompt = String::from(
            "You are generating search terms for a native word index. Based on the user's natural language query, \
            generate relevant search terms that would help find matching records.\n\n"
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            // Include key configuration for Range and HashRange schemas
            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!(
                        "  Hash Key: {} (indexed for fast lookup)\n",
                        hash_field
                    ));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!(
                        "  Range Key: {} (indexed for fast lookup)\n",
                        range_field
                    ));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Generate 3-8 relevant search terms that would help find records matching this query.\n\n\
            Guidelines:\n\
            - Extract key words and phrases from the query\n\
            - Include synonyms and related terms\n\
            - Consider different ways the same concept might be expressed\n\
            - Include specific names, places, or entities mentioned\n\
            - Generate terms that would be found in indexed fields\n\
            - Avoid very common words (stopwords)\n\
            - Keep terms concise but meaningful\n\n\
            Examples:\n\
            - Query: \"Find posts about artificial intelligence\"\n\
              Terms: [\"artificial\", \"intelligence\", \"AI\", \"machine learning\", \"neural network\"]\n\
            - Query: \"Show me articles by Jennifer Liu\"\n\
              Terms: [\"Jennifer\", \"Liu\", \"Jennifer Liu\", \"author\"]\n\
            - Query: \"Products with electronics tag\"\n\
              Terms: [\"electronics\", \"electronic\", \"tech\", \"gadgets\", \"devices\"]\n\n\
            Respond with a JSON array of strings:\n\
            [\"term1\", \"term2\", \"term3\", ...]\n\n\
            IMPORTANT: Return ONLY the JSON array, no additional text."
        );

        prompt
    }

    /// Parse the query terms response
    fn parse_query_terms_response(&self, response: &str) -> Result<Vec<String>, String> {
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

    /// Generate native index search terms specifically for search execution
    async fn generate_native_index_search_terms(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> Result<Vec<String>, String> {
        let prompt = self.build_native_index_search_prompt(user_query, schemas);
        let response = self.call_llm(&prompt).await?;
        self.parse_query_terms_response(&response)
    }

    /// Deduplicate results based on schema_name + key_value + field combination
    fn deduplicate_results(
        &self,
        mut results: Vec<crate::db_operations::IndexResult>,
    ) -> Vec<crate::db_operations::IndexResult> {
        let _original_count = results.len();
        let mut seen = HashSet::new();

        results.retain(|result| {
            // Create a unique key based on schema_name + key_value + field
            let key = format!(
                "{}:{}:{}",
                result.schema_name,
                serde_json::to_string(&result.key_value).unwrap_or_default(),
                result.field
            );

            seen.insert(key)
        });

        results
    }

    /// Interpret native index search results using AI
    ///
    /// This method takes search results (potentially hydrated with actual values)
    /// and sends them to the AI for interpretation and summarization.
    pub async fn interpret_native_index_results(
        &self,
        original_query: &str,
        results: &[crate::db_operations::IndexResult],
    ) -> Result<String, String> {
        log::info!(
            "LLM Query: Sending {} results to AI for interpretation",
            results.len()
        );
        if results.is_empty() {
            log::warn!("LLM Query: No results to send to AI");
        } else {
            log::debug!(
                "LLM Query: Sample result - schema={}, field={}, key_value={:?}",
                results[0].schema_name,
                results[0].field,
                results[0].key_value
            );
        }
        let prompt = self.build_native_index_interpretation_prompt(original_query, results);
        self.call_llm(&prompt).await
    }

    /// Build prompt for native index search term generation
    fn build_native_index_search_prompt(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let mut prompt = String::from(
            "You are generating search terms for a native word index system. Based on the user's natural language query, \
            generate 3-6 specific search terms that will be used to search the native index.\n\n"
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!(
                        "  Hash Key: {} (indexed for fast lookup)\n",
                        hash_field
                    ));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!(
                        "  Range Key: {} (indexed for fast lookup)\n",
                        range_field
                    ));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Generate 3-6 specific search terms that will be used to search the native word index.\n\n\
            Guidelines:\n\
            - Extract the most important keywords from the query\n\
            - Include specific names, places, or entities mentioned\n\
            - Generate terms that would be found in indexed text fields\n\
            - Avoid very common words (stopwords)\n\
            - Keep terms concise but meaningful\n\
            - Focus on terms that are likely to appear in the data\n\n\
            Examples:\n\
            - Query: \"Find posts about artificial intelligence\"\n\
              Terms: [\"artificial\", \"intelligence\", \"AI\", \"machine learning\"]\n\
            - Query: \"Show me articles by Jennifer Liu\"\n\
              Terms: [\"Jennifer\", \"Liu\", \"Jennifer Liu\"]\n\
            - Query: \"Products with electronics tag\"\n\
              Terms: [\"electronics\", \"electronic\", \"tech\"]\n\n\
            Respond with a JSON array of strings:\n\
            [\"term1\", \"term2\", \"term3\", ...]\n\n\
            IMPORTANT: Return ONLY the JSON array, no additional text."
        );

        prompt
    }

    /// Build prompt for interpreting native index results
    fn build_native_index_interpretation_prompt(
        &self,
        original_query: &str,
        results: &[crate::db_operations::IndexResult],
    ) -> String {
        let results_preview = if results.len() > 50 {
            &results[..50]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        format!(
            "You are interpreting native index search results for a user. Analyze the search results and provide a helpful response.\n\n\
            Original User Query: {}\n\
            Search Results ({} total, showing first {}):\n{}\n\n\
            Provide:\n\
            1. A summary of what was found\n\
            2. Key insights from the results\n\
            3. Notable patterns or interesting findings\n\
            4. If no results were found, suggest alternative search terms\n\n\
            Keep the response concise, informative, and helpful to the user.",
            original_query,
            results.len(),
            results_preview.len(),
            results_str
        )
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

        prompt.push_str(&format!(
            "User's Original Question: {}\n\n",
            original_user_query
        ));

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

            // Include key configuration for Range and HashRange schemas
            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nSuggest ONE alternative approach:\n");
        prompt.push_str("1. Try a different schema that might have the data\n");
        prompt.push_str(
            "2. Broaden the filter (e.g., remove date constraints, use pattern matching)\n",
        );
        prompt.push_str("3. Try a different filter type (e.g., null filter for all records)\n");
        prompt.push_str("4. Search in related/index schemas\n\n");

        prompt
            .push_str("If you believe there are NO reasonable alternatives left, respond with:\n");
        prompt.push_str(
            "{\"has_alternative\": false, \"query\": null, \"reasoning\": \"explanation\"}\n\n",
        );

        prompt.push_str("Otherwise, respond with:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"has_alternative\": true,\n");
        prompt.push_str(
            "  \"query\": {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ...},\n",
        );
        prompt.push_str("  \"reasoning\": \"why this approach might work\"\n");
        prompt.push_str("}\n\n");

        prompt.push_str(
            "FILTER TYPES:\n\
            For HashRange schemas (check Hash Key field):\n\
            - HashRangeKey, HashKey, HashRangePrefix, HashPattern\n\
            For Range schemas (check Range Key field):\n\
            - RangePrefix, RangePattern, RangeRange\n\
            Universal filters:\n\
            - Value (LAST RESORT ONLY), SampleN, null (all records)\n\n\
            JSON FORMATTING:\n\
            - Use proper JSON format for all filter values\n\
            - Special characters like @ # $ are valid in JSON strings\n\
            - Example: {\"Value\": \"@username\"}, {\"HashKey\": \"@mention\"}\n\n\
            CRITICAL: Prefer key-based filters over Value filter.\n\
            Check Hash Key and Range Key fields to determine correct filter.\n\
            If search matches a key field, use key filter (HashKey/RangePrefix), NOT Value filter.\n\n\
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

    /// Build the analysis prompt
    fn build_analysis_prompt(&self, user_query: &str, schemas: &[SchemaWithState]) -> String {
        let mut prompt = String::from(
            "You are a database query optimizer. Analyze the following natural language query \
            and available schemas to create an execution plan.\n\n",
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            // Include key configuration for Range and HashRange schemas
            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

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
            FILTER TYPES AVAILABLE:\n\n\
            Filters for HashRange schemas (have both Hash Key and Range Key):\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match on BOTH hash key field AND range key field\n\
            - HashKey: {\"HashKey\": \"value\"} - filter on hash key field only, returns all records with this hash\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}} - filter on hash key field + range key field prefix\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - glob pattern matching on hash key field\n\n\
            Filters for Range schemas (have Range Key only):\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - filter on range key field, returns records with range starting with prefix\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern matching on range key field\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - filter on range key field for values within range\n\n\
            Universal filters (work on any schema type):\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records (NOT sorted)\n\
            - null - no filter (return all records)\n\n\
            IMPORTANT JSON FORMATTING:\n\
            - All string values in filters MUST be properly JSON-escaped\n\
            - Special characters like @ # $ etc. do NOT need escaping in JSON strings\n\
            - Example: {\"HashKey\": \"user@domain.com\"} is valid JSON\n\n\
            CRITICAL FILTER SELECTION RULES:\n\
            1. ALWAYS check the schema's Hash Key and Range Key fields to determine the correct filter\n\
            2. If the search term matches a Hash Key field value, use HashKey or HashPattern filter\n\
            3. If the search term matches a Range Key field value, use RangePrefix, RangePattern, or RangeRange filter\n\
            4. Examples of when to use each:\n\
               - Searching for author \"Jennifer Liu\" on a schema with hash_field=author → use {\"HashKey\": \"Jennifer Liu\"}\n\
               - Searching for date \"2025-09\" on a schema with range_field=publish_date → use {\"RangePrefix\": \"2025-09\"}\n\n\
            IMPORTANT NOTES:\n\
            - For HashRange schemas, HashKey filters operate on the hash_field, Range filters operate on the range_field\n\
            - For Range schemas, Range filters operate on the range_field\n\
            - SampleN returns RANDOM records, NOT sorted or ordered\n\
            - For \"most recent\" or \"latest\" queries, use null filter to get all records (backend will handle sorting)\n\
            - Range keys are stored as strings and compared lexicographically\n\n\
            EXAMPLES:\n\
            - Search for word \"ai\" in BlogPostWordIndex (hash_field=word): {\"HashKey\": \"ai\"} ✓ CORRECT\n\
            - Search for author \"Jennifer Liu\" in schema with hash_field=author: {\"HashKey\": \"Jennifer Liu\"} ✓ CORRECT\n\
            - Get blog post by ID in BlogPost (range_field=post_id): {\"RangePrefix\": \"post-123\"} ✓ CORRECT\n\
            - Get most recent posts: null (returns all, sorted by backend) ✓ CORRECT\n\
            - Get posts in date range (range_field=publish_date): {\"RangeRange\": {\"start\": \"2025-09-01\", \"end\": \"2025-09-30\"}} ✓ CORRECT\n\n\
            Respond in JSON format with:\n\
            {\n\
              \"query\": {\n\
                \"schema_name\": \"string\",\n\
                \"fields\": [\"field1\", \"field2\"],\n\
                \"filter\": null or one of the filter types above\n\
              },\n\
              \"index_schema\": null or index schema definition (see below),\n\
              \"reasoning\": \"your analysis\"\n\
            }\n\n\
            INDEX SCHEMA CREATION:\n\
            If no efficient schema exists for the query, recommend an index schema.\n\
            Index schemas enable fast lookups by creating a HashRange index on specific fields.\n\n\
            When to recommend an index:\n\
            - Word search queries (e.g., \"find posts containing 'technology'\")\n\
            - Array field searches (e.g., \"products with tag 'electronics'\")\n\
            - Author/user lookup queries (e.g., \"posts by Alice Johnson\")\n\
            - Any query that would benefit from hash-based lookup\n\n\
            Index schema format:\n\
            {\n\
              \"name\": \"SourceSchemaFieldIndex\",\n\
              \"descriptive_name\": \"Human Readable Name\",\n\
              \"key\": {\n\
                \"hash_field\": \"field_to_index_on\",\n\
                \"range_field\": \"timestamp_or_id_field\"\n\
              },\n\
              \"transform_fields\": {\n\
                \"indexed_field\": \"SourceSchema.field.transform()\",\n\
                \"other_field\": \"SourceSchema.map().other_field\"\n\
              },\n\
              \"field_topologies\": {\n\
                \"indexed_field\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"other_field\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}}\n\
              }\n\
            }\n\n\
            CRITICAL TOPOLOGY FORMAT:\n\
            - Every field in field_topologies MUST have format: {\"root\": {\"type\": \"Primitive\", \"value\": \"TYPE\"}}\n\
            - The \"value\" field is REQUIRED for Primitive types\n\
            - Valid values: \"String\", \"Number\", \"Boolean\", \"Null\"\n\
            - Arrays: {\"root\": {\"type\": \"Array\", \"value\": {\"type\": \"Primitive\", \"value\": \"String\"}}}\n\
            - Objects: {\"root\": {\"type\": \"Object\", \"value\": {\"field1\": {\"type\": \"Primitive\", \"value\": \"String\"}}}}\n\n\
            Transform functions available:\n\
            - split_by_word() - splits text into individual words\n\
            - split_array() - splits array into individual elements\n\
            - count() - counts items (returns Number)\n\
            - map() - applies transformation to each item\n\n\
            Example index schemas:\n\
            1. Word search index:\n\
            {\n\
              \"name\": \"BlogPostWordIndex\",\n\
              \"descriptive_name\": \"Blog Post Word Index\",\n\
              \"key\": {\"hash_field\": \"word\", \"range_field\": \"publish_date\"},\n\
              \"transform_fields\": {\n\
                \"word\": \"BlogPost.map().content.split_by_word().map()\",\n\
                \"title\": \"BlogPost.map().title\",\n\
                \"author\": \"BlogPost.map().author\",\n\
                \"publish_date\": \"BlogPost.map().publish_date\"\n\
              },\n\
              \"field_topologies\": {\n\
                \"word\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"title\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"author\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"publish_date\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}}\n\
              }\n\
            }\n\n\
            2. Author lookup index:\n\
            {\n\
              \"name\": \"BlogPostAuthorIndex\",\n\
              \"descriptive_name\": \"Blog Post Author Index\",\n\
              \"key\": {\"hash_field\": \"author\", \"range_field\": \"publish_date\"},\n\
              \"transform_fields\": {\n\
                \"author\": \"BlogPost.map().author\",\n\
                \"title\": \"BlogPost.map().title\",\n\
                \"content\": \"BlogPost.map().content\",\n\
                \"publish_date\": \"BlogPost.map().publish_date\"\n\
              },\n\
              \"field_topologies\": {\n\
                \"author\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"title\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"content\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"publish_date\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}}\n\
              }\n\
            }\n\n\
            3. Tag search index (array splitting):\n\
            {\n\
              \"name\": \"ProductTagIndex\",\n\
              \"descriptive_name\": \"Product Tag Index\",\n\
              \"key\": {\"hash_field\": \"tag\", \"range_field\": \"created_at\"},\n\
              \"transform_fields\": {\n\
                \"tag\": \"Product.map().tags.split_array().map()\",\n\
                \"product_id\": \"Product.map().product_id\",\n\
                \"name\": \"Product.map().name\",\n\
                \"price\": \"Product.map().price\",\n\
                \"created_at\": \"Product.map().created_at\"\n\
              },\n\
              \"field_topologies\": {\n\
                \"tag\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"product_id\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"name\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}},\n\
                \"price\": {\"root\": {\"type\": \"Primitive\", \"value\": \"Number\"}},\n\
                \"created_at\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\"}}\n\
              }\n\
            }\n\n\
            IMPORTANT: \n\
            - Return ONLY the JSON object, no additional text\n\
            - Use the EXACT filter format shown above\n\
            - For \"most recent\", \"latest\", or \"newest\" queries, use null filter (NOT SampleN)\n\
            - Prefer existing approved schemas; only recommend index_schema if no efficient schema exists\n\
            - Index schemas must always have schema_type \"HashRange\" (implicit)\n\
            - Always include field_topologies for all fields in transform_fields\n\
            - Choose hash_field based on what will be queried (word, author, tag, etc.)\n\
            - Choose range_field as a timestamp or ID for natural ordering"
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
            the context provided.\n\n",
        );

        prompt.push_str(&format!("Original Query: {}\n", original_query));
        prompt.push_str(&format!(
            "Results ({} total): {}\n\n",
            results.len(),
            results_str
        ));

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
