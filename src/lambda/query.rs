//! Query operations for Lambda context

use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::IngestionError;
use crate::lambda::types::{
    AIQueryResponse, CompleteQueryResponse, ConversationMessage, FollowupRequest, 
    FollowupResponse, QueryContext, QueryPlanInfo,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::context::LambdaContext;

impl LambdaContext {
    /// Execute an AI-native index query using semantic search
    ///
    /// This is the simplest method - provide a natural language query and get
    /// AI-interpreted results. Fully stateless.
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let response = LambdaContext::ai_query("Find all electronics products").await?;
    ///     println!("AI says: {}", response.ai_interpretation);
    ///     println!("Found {} results", response.raw_results.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn ai_query(query: &str) -> Result<AIQueryResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Execute AI-native index query workflow
        let (ai_interpretation, raw_results) = {
            let node = ctx.node.lock().await;
            let db_ops = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?
                .get_db_ops();
            drop(node); // Drop lock before await
            
            service.execute_ai_native_index_query_with_results(query, &schemas, &db_ops).await
                .map_err(|e| IngestionError::InvalidInput(format!("AI query failed: {}", e)))?
        };

        // Convert results to JSON
        let results_as_json: Vec<Value> = raw_results
            .into_iter()
            .map(|result| serde_json::to_value(result).unwrap_or(serde_json::json!({})))
            .collect();

        // Build context for potential follow-ups
        let context = QueryContext {
            original_query: query.to_string(),
            query_results: results_as_json.clone(),
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: query.to_string(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
                ConversationMessage {
                    role: "assistant".to_string(),
                    content: ai_interpretation.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            ],
            query_plan: None,
        };

        Ok(AIQueryResponse {
            ai_interpretation,
            raw_results: results_as_json,
            context,
        })
    }

    /// Run complete AI query workflow: analyze + execute + summarize
    ///
    /// This method handles the full workflow internally and waits for any
    /// necessary backfills to complete before returning results. Fully stateless.
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let response = LambdaContext::run_ai_query("Show recent blog posts").await?;
    ///     println!("Found {} records", response.results.len());
    ///     if let Some(summary) = response.summary {
    ///         println!("Summary: {}", summary);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_ai_query(query: &str) -> Result<CompleteQueryResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Analyze query with LLM
        let query_plan = service.analyze_query(query, &schemas).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to analyze query: {}", e)))?;

        // Execute the query
        let node_arc = Arc::clone(&ctx.node);
        let processor = OperationProcessor::new(node_arc);
        let results = match processor.execute_query_map(query_plan.query.clone()).await {
            Ok(result_map) => {
                let records_map = records_from_field_map(&result_map);
                records_map
                    .into_iter()
                    .map(|(key, record)| serde_json::json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                    .collect::<Vec<Value>>()
            }
            Err(e) => {
                return Err(IngestionError::InvalidInput(format!("Failed to execute query: {}", e)));
            }
        };

        // Summarize results with LLM
        let summary = service.summarize_results(query, &results).await.ok();

        // Build query plan info
        let filter_type = query_plan.query.filter.as_ref().map(|f| format!("{:?}", f));
        let query_plan_info = QueryPlanInfo {
            schema_name: query_plan.query.schema_name.clone(),
            fields: query_plan.query.fields.clone(),
            filter_type,
            reasoning: query_plan.reasoning.clone(),
        };

        // Build context for follow-ups
        let mut conversation_history = vec![
            ConversationMessage {
                role: "user".to_string(),
                content: query.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        ];

        if let Some(ref s) = summary {
            conversation_history.push(ConversationMessage {
                role: "assistant".to_string(),
                content: s.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        let context = QueryContext {
            original_query: query.to_string(),
            query_results: results.clone(),
            conversation_history,
            query_plan: Some(query_plan_info.clone()),
        };

        Ok(CompleteQueryResponse {
            query_plan: query_plan_info,
            results,
            summary,
            context,
        })
    }

    /// Ask a follow-up question about previous query results
    ///
    /// Completely stateless - client provides full context from previous query.
    ///
    /// # Arguments
    ///
    /// * `request` - Follow-up request with context and question
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaContext, FollowupRequest};
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     // First query
    ///     let response1 = LambdaContext::run_ai_query("Show all products").await?;
    ///     
    ///     // Follow-up question
    ///     let followup = LambdaContext::ask_followup(FollowupRequest {
    ///         context: response1.context,
    ///         question: "Which are electronics?".to_string(),
    ///     }).await?;
    ///     
    ///     println!("Answer: {}", followup.answer);
    ///     Ok(())
    /// }
    /// ```
    pub async fn ask_followup(request: FollowupRequest) -> Result<FollowupResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        let context = request.context;
        let question = request.question;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Convert conversation history to Message format
        let conversation_history: Vec<crate::datafold_node::llm_query::types::Message> = context
            .conversation_history
            .iter()
            .map(|msg| crate::datafold_node::llm_query::types::Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
                timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(msg.timestamp),
            })
            .collect();

        // Analyze if follow-up needs a new query
        let analysis = service
            .analyze_followup_question(
                &context.original_query,
                &context.query_results,
                &question,
                &schemas,
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to analyze followup: {}", e)))?;

        let mut combined_results = context.query_results.clone();
        let mut executed_new_query = false;

        // If a new query is needed, execute it
        if analysis.needs_query {
            if let Some(new_query) = analysis.query {
                executed_new_query = true;
                let node_arc = Arc::clone(&ctx.node);
                let processor = OperationProcessor::new(node_arc);
                match processor.execute_query_map(new_query).await {
                    Ok(result_map) => {
                        let records_map = records_from_field_map(&result_map);
                        combined_results = records_map
                            .into_iter()
                            .map(|(key, record)| serde_json::json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                            .collect();
                    }
                    Err(e) => {
                        log::warn!("Failed to execute followup query: {}", e);
                    }
                }
            }
        }

        // Get answer from AI
        let answer = service
            .answer_question(
                &context.original_query,
                &combined_results,
                &conversation_history,
                &question,
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get answer: {}", e)))?;

        // Build updated context
        let mut updated_conversation = context.conversation_history.clone();
        updated_conversation.push(ConversationMessage {
            role: "user".to_string(),
            content: question.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        updated_conversation.push(ConversationMessage {
            role: "assistant".to_string(),
            content: answer.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });

        let updated_context = QueryContext {
            original_query: context.original_query,
            query_results: combined_results,
            conversation_history: updated_conversation,
            query_plan: context.query_plan,
        };

        Ok(FollowupResponse {
            answer,
            executed_new_query,
            context: updated_context,
        })
    }

    /// Execute a query and return results
    ///
    /// This is for regular (non-AI) queries where you know the schema and fields.
    ///
    /// # Arguments
    ///
    /// * `query` - Query specification with schema name, fields, and optional filter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Query;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let query = Query {
    ///         schema_name: "users".to_string(),
    ///         fields: vec!["name".to_string(), "email".to_string()],
    ///         filter: None,
    ///     };
    ///     
    ///     let results = LambdaContext::query(query).await?;
    ///     println!("Found {} records", results.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn query(query: crate::schema::types::Query) -> Result<Vec<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node_arc = Arc::clone(&ctx.node);
        let processor = OperationProcessor::new(node_arc);
        
        match processor.execute_query_map(query).await {
            Ok(result_map) => {
                let records_map = records_from_field_map(&result_map);
                let results: Vec<Value> = records_map
                    .into_iter()
                    .map(|(key, record)| serde_json::json!({
                        "key": key,
                        "fields": record.fields,
                        "metadata": record.metadata
                    }))
                    .collect();
                Ok(results)
            }
            Err(e) => Err(IngestionError::InvalidInput(format!("Query failed: {}", e))),
        }
    }

    /// Search the native word index
    ///
    /// Search across all classifications in the native word index.
    ///
    /// # Arguments
    ///
    /// * `term` - Search term
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let results = LambdaContext::native_index_search("electronics").await?;
    ///     println!("Found {} results", results.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn native_index_search(term: &str) -> Result<Vec<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
          let results = db_guard.native_search_all_classifications(term).await
            .map_err(|e| IngestionError::InvalidInput(format!("Native index search failed: {}", e)))?;
        
        Ok(results.into_iter()
            .map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!({})))
            .collect())
    }
}
