//! Query operations for Lambda context
//!
//! Optimized for direct node access without OperationProcessor wrapper.

use crate::fold_db_core::orchestration::IndexingStatus;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::IngestionError;
use crate::lambda::types::{
    AIQueryResponse, CompleteQueryResponse, ConversationMessage, FollowupRequest, FollowupResponse,
    QueryContext, QueryPlanInfo,
};
use crate::schema::types::operations::Mutation;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use super::context::LambdaContext;
use crate::datafold_node::OperationProcessor;
use crate::error::FoldDbError;

impl LambdaContext {
    /// Execute an AI-native index query using semantic search
    pub async fn ai_query(query: &str, user_id: String) -> Result<AIQueryResponse, IngestionError> {
        let ctx = Self::get()?;

        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            let processor = OperationProcessor::new(node_guard.clone());
            processor.list_schemas().await.map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to get schemas: {}", e))
            })?
        };

        // Execute AI-native index query workflow
        use crate::lambda::logging::run_with_user;
        let (ai_interpretation, raw_results) = run_with_user(&user_id, async {
            let node_mutex = Self::get_node(&user_id).await?;
            let node = node_mutex.lock().await;
            let db_ops = node
                .get_fold_db()
                .await
                .map_err(|e| {
                    IngestionError::InvalidInput(format!("Failed to access database: {}", e))
                })?
                .get_db_ops();
            drop(node); // Drop lock before await

            service
                .execute_ai_native_index_query_with_results(query, &schemas, &db_ops)
                .await
                .map_err(|e| IngestionError::InvalidInput(format!("AI query failed: {}", e)))
        })
        .await?;

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
    pub async fn run_ai_query(
        query: &str,
        user_id: String,
    ) -> Result<CompleteQueryResponse, IngestionError> {
        let ctx = Self::get()?;

        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            let processor = OperationProcessor::new(node_guard.clone());
            processor.list_schemas().await.map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to get schemas: {}", e))
            })?
        };

        use crate::lambda::logging::run_with_user;
        run_with_user(&user_id, async {
            // Analyze query with LLM
            let query_plan = service.analyze_query(query, &schemas).await.map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to analyze query: {}", e))
            })?;

            // Execute the query
            let processor = {
                let node_mutex = Self::get_node(&user_id).await?;
                let node_guard = node_mutex.lock().await;
                OperationProcessor::new(node_guard.clone())
            };

            let results = match processor.execute_query_json(query_plan.query.clone()).await {
                Ok(results) => results,
                Err(e) => {
                    return Err(IngestionError::InvalidInput(format!(
                        "Failed to execute query: {}",
                        e
                    )));
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
            let mut conversation_history = vec![ConversationMessage {
                role: "user".to_string(),
                content: query.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }];

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
        })
        .await
    }

    /// Ask a follow-up question about previous query results
    pub async fn ask_followup(
        request: FollowupRequest,
        user_id: String,
    ) -> Result<FollowupResponse, IngestionError> {
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
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            let processor = OperationProcessor::new(node_guard.clone());
            processor.list_schemas().await.map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to get schemas: {}", e))
            })?
        };

        use crate::lambda::logging::run_with_user;
        run_with_user(&user_id, async {
            // Convert conversation history
            let conversation_history: Vec<crate::datafold_node::llm_query::types::Message> =
                context
                    .conversation_history
                    .iter()
                    .map(|msg| crate::datafold_node::llm_query::types::Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        timestamp: SystemTime::UNIX_EPOCH
                            + std::time::Duration::from_secs(msg.timestamp),
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
                .map_err(|e| {
                    IngestionError::InvalidInput(format!("Failed to analyze followup: {}", e))
                })?;

            let mut combined_results = context.query_results.clone();
            let mut executed_new_query = false;

            // If a new query is needed, execute it
            if analysis.needs_query {
                if let Some(new_query) = analysis.query {
                    executed_new_query = true;

                    let processor = {
                        let node_mutex = Self::get_node(&user_id).await?;
                        let node_guard = node_mutex.lock().await;
                        OperationProcessor::new(node_guard.clone())
                    };

                    match processor.execute_query_json(new_query).await {
                        Ok(results) => {
                            combined_results = results;
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
                .map_err(|e| {
                    IngestionError::InvalidInput(format!("Failed to get answer: {}", e))
                })?;

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
        })
        .await
    }

    /// Execute a query and return results
    pub async fn query(
        query: crate::schema::types::Query,
        user_id: String,
    ) -> Result<Vec<Value>, IngestionError> {
        use crate::lambda::logging::run_with_user;
        run_with_user(&user_id, async {
            let processor = {
                let node_mutex = Self::get_node(&user_id).await?;
                let node_guard = node_mutex.lock().await;
                OperationProcessor::new(node_guard.clone())
            };

            match processor.execute_query_json(query).await {
                Ok(results) => Ok(results),
                Err(e) => Err(IngestionError::InvalidInput(format!("Query failed: {}", e))),
            }
        })
        .await
    }

    /// Search the native word index
    pub async fn native_index_search(
        term: &str,
        user_id: String,
    ) -> Result<Vec<Value>, IngestionError> {
        use crate::lambda::logging::run_with_user;
        run_with_user(&user_id, async {
            let processor = {
                let node_mutex = Self::get_node(&user_id).await?;
                let node_guard = node_mutex.lock().await;
                OperationProcessor::new(node_guard.clone())
            };

            let results = processor.native_index_search(term).await.map_err(|e| {
                IngestionError::InvalidInput(format!("Native index search failed: {}", e))
            })?;

            Ok(results
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!({})))
                .collect())
        })
        .await
    }

    /// Execute a mutation
    pub async fn execute_mutation(
        mutation: Mutation,
        user_id: String,
    ) -> Result<String, IngestionError> {
        use crate::lambda::logging::run_with_user;
        // OperationProcessor expects key_value separate from fields.
        // But the input Mutation struct already has key_value inside.
        // We can reconstruct arguments or add a direct mutation method to OperationProcessor?
        // OperationProcessor has execute_mutation which takes fields, key_value etc.
        // It also has execute_mutations_batch which takes Vec<Value>!
        // Or we can add `execute_mutation_object` to OperationProcessor?
        // Actually, execute_mutations_batch logic inside OperationProcessor converts Value -> Mutation.
        // But here we have Mutation object.
        // DataFoldNode::mutate_batch takes Vec<Mutation>.
        // OperationProcessor::execute_mutation creates a Mutation.

        // Let's use DataFoldNode wrapper in OperationProcessor if possible?
        // No, best to use OperationProcessor::execute_mutations_batch but that takes Value.

        // Let's serialize the mutation to Value and pass it to execute_mutations_batch?
        // That seems inefficient.

        // Or better: Let's use OperationProcessor::new(node).node.mutate_batch?
        // But that defeats the purpose of encapsulation if we just bypass it.

        // Wait, OperationProcessor is supposed to UNIFY implementation.
        // Function `execute_mutations_batch` in OperationProcessor takes `Vec<Value>`.
        // This is primarily for the HTTP API which receives JSON.
        // The Lambda might receive JSON too? Here input is `Mutation` struct.

        // I should probably add `execute_mutations_direct` to OperationProcessor that takes `Vec<Mutation>`?
        // Or just use the node inside processor? Use processor.execute_mutation logic?

        // Let's look at OperationProcessor again.
        // It has `execute_mutation` which builds a Mutation object.
        // It SHOULD have a method to execute prepared mutations.
        // Currently `DataFoldNode` has `mutate_batch`.

        // If I strictly want to use OperationProcessor, I should use `execute_mutations_batch` passing JSON values.
        // `serde_json::to_value(mutation)` works.

        run_with_user(&user_id, async {
            let processor = {
                let node_mutex = Self::get_node(&user_id).await?;
                let node_guard = node_mutex.lock().await;
                OperationProcessor::new(node_guard.clone())
            };

            processor
                .execute_mutation_op(mutation)
                .await
                .map_err(|e| IngestionError::InvalidInput(format!("Mutation failed: {}", e)))
        })
        .await
    }

    /// Execute multiple mutations in a batch
    pub async fn execute_mutations_batch(
        mutations: Vec<Mutation>,
        user_id: String,
    ) -> Result<Vec<String>, IngestionError> {
        use crate::lambda::logging::run_with_user;
        run_with_user(&user_id, async {
            let processor = {
                let node_mutex = Self::get_node(&user_id).await?;
                let node_guard = node_mutex.lock().await;
                OperationProcessor::new(node_guard.clone())
            };

            // Serialize mutations
            processor
                .execute_mutations_batch_ops(mutations)
                .await
                .map_err(|e| IngestionError::InvalidInput(format!("Batch mutation failed: {}", e)))
        })
        .await
    }

    /// List all transforms
    pub async fn list_transforms(
    ) -> Result<std::collections::HashMap<String, crate::schema::types::Transform>, IngestionError>
    {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor
            .list_transforms()
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("detect: {}", e)))
    }

    /// Get transform queue info
    pub async fn get_transform_queue() -> Result<Value, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        let (len, queued) = processor.get_transform_queue().await.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to get transform queue info: {}", e))
        })?;

        Ok(serde_json::json!({
            "length": len,
            "queued_transforms": queued
        }))
    }

    /// Add transform to queue
    pub async fn add_to_transform_queue(id: &str) -> Result<(), IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor
            .add_to_transform_queue(id, "manual_lambda_trigger")
            .await
            .map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to add transform to queue: {}", e))
            })
    }

    /// Get all backfills
    pub async fn get_all_backfills() -> Result<
        Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>,
        IngestionError,
    > {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor
            .get_all_backfills()
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfills: {}", e)))
    }

    /// Get active backfills
    pub async fn get_active_backfills() -> Result<
        Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>,
        IngestionError,
    > {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor.get_active_backfills().await.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to get active backfills: {}", e))
        })
    }

    /// Get backfill by ID
    pub async fn get_backfill(
        id: &str,
    ) -> Result<
        Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>,
        IngestionError,
    > {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor
            .get_backfill(id)
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfill: {}", e)))
    }

    /// Get backfill statistics
    pub async fn get_backfill_statistics() -> Result<
        crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatistics,
        IngestionError,
    > {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor.get_backfill_statistics().await.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to get backfill statistics: {}", e))
        })
    }

    /// Get transform statistics
    pub async fn get_transform_statistics() -> Result<Value, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        let stats = processor.get_transform_statistics().await.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to get transform statistics: {}", e))
        })?;
        Ok(serde_json::to_value(stats).unwrap_or(serde_json::json!({})))
    }

    /// Get indexing status
    pub async fn get_indexing_status() -> Result<IndexingStatus, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());

        processor.get_indexing_status().await.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to get indexing status: {}", e))
        })
    }
}
