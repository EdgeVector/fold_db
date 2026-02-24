//! Native index search, interpretation, and alternative query suggestion.

use super::super::types::{QueryPlan, ToolCallRecord};
use crate::schema::types::Query;
use serde_json::Value;

use super::LlmQueryService;

impl LlmQueryService {
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

    /// Search the native index and return results (without AI interpretation)
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

        log::info!(
            "LLM Query: Found {} results from native index",
            all_results.len()
        );

        Ok(all_results)
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

    /// Execute a tool call and return the result
    pub(super) async fn execute_tool(
        &self,
        tool: &str,
        params: &Value,
        node: &crate::fold_node::node::FoldNode,
    ) -> Result<Value, String> {
        let processor = crate::fold_node::OperationProcessor::new(node.clone());

        match tool {
            "query" => {
                let schema_name = params
                    .get("schema_name")
                    .and_then(|s| s.as_str())
                    .ok_or("query tool requires 'schema_name' parameter")?;

                let mut fields: Vec<String> = params
                    .get("fields")
                    .and_then(|f| f.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                // When the agent omits fields, default to all fields from the schema
                if fields.is_empty() {
                    if let Ok(Some(schema_with_state)) = processor.get_schema(schema_name).await {
                        fields = schema_with_state.schema.runtime_fields.keys().cloned().collect();
                    }
                }

                let filter = params.get("filter").cloned();

                let query = Query {
                    schema_name: schema_name.to_string(),
                    fields,
                    filter: filter.and_then(|f| serde_json::from_value(f).ok()),
                    as_of: None,
                    rehydrate_depth: Some(1),
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
        schemas: &[crate::schema::SchemaWithState],
        node: &crate::fold_node::node::FoldNode,
        _user_hash: &str,
        max_iterations: usize,
    ) -> Result<(String, Vec<ToolCallRecord>), String> {
        let mut tool_calls: Vec<ToolCallRecord> = Vec::new();
        let mut conversation_context = String::new();

        // Build the initial system prompt with tool definitions
        let system_prompt = self.build_agent_system_prompt(schemas);
        let today = chrono::Local::now().format("%A, %B %-d, %Y").to_string();

        log::info!(
            "Agent: Starting query with max {} iterations: {}",
            max_iterations,
            user_query
        );

        for iteration in 0..max_iterations {
            // Build the full prompt with conversation history
            // Repeat the current date at the end so it's fresh context when generating the answer
            let full_prompt = format!(
                "{}\n\n{}\n\nUser Query: {}\n\nReminder: Today is {}. Dates before today are in the past. Dates after today are in the future.\n\nRespond with a JSON object. Either:\n- {{\"tool\": \"tool_name\", \"params\": {{...}}}} to use a tool\n- {{\"answer\": \"your final response\"}} when you have the answer",
                system_prompt,
                conversation_context,
                user_query,
                today
            );

            log::debug!("Agent: Iteration {} - calling LLM", iteration + 1);

            let response = self.call_llm(&full_prompt).await?;

            log::debug!("Agent: LLM response: {}", &response[..response.len().min(200)]);

            // Parse the response
            let action = self.parse_agent_response(&response)?;

            match action {
                super::super::types::AgentAction::Answer(answer) => {
                    log::info!(
                        "Agent: Completed after {} iterations with {} tool calls",
                        iteration + 1,
                        tool_calls.len()
                    );
                    return Ok((answer, tool_calls));
                }
                super::super::types::AgentAction::ToolCall { tool, params } => {
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
}
