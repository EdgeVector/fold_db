pub mod ask;
pub mod completions;
pub mod ingest;
pub mod mutate;
pub mod query;
pub mod schema;
pub mod search;
pub mod system;
pub mod transform;

use crate::cli::{Command, ConfigCommand};
use crate::error::CliError;
use crate::output::OutputMode;
use fold_db::fold_node::OperationProcessor;
use fold_db::db_operations::native_index::IndexResult;
use fold_db::fold_db_core::infrastructure::backfill_tracker::BackfillStatistics;
use fold_db::fold_db_core::infrastructure::event_statistics::EventStatistics;
use fold_db::fold_db_core::orchestration::index_status::IndexingStatus;
use fold_db::ingestion::smart_folder::SmartFolderScanResponse;
use fold_db::schema::schema_types::SchemaWithState;
use fold_db::schema::types::transform::Transform;
use fold_db::storage::DatabaseConfig;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub enum CommandOutput {
    SchemaList(Vec<SchemaWithState>),
    SchemaGet(Box<SchemaWithState>),
    SchemaApproved {
        name: String,
        backfill_hash: Option<String>,
    },
    SchemaBlocked {
        name: String,
    },
    SchemaLoaded {
        available: usize,
        loaded: usize,
        failed: Vec<String>,
    },
    QueryResults(Vec<Value>),
    SearchResults(Vec<IndexResult>),
    MutationSuccess {
        id: String,
    },
    MutationBatch {
        ids: Vec<String>,
    },
    IngestSuccess {
        count: usize,
        ids: Vec<String>,
    },
    SmartScan(SmartFolderScanResponse),
    SmartIngestResults {
        total: usize,
        succeeded: usize,
        failed: usize,
        results: Vec<Value>,
    },
    AskAnswer {
        answer: String,
        tool_calls: Vec<fold_db::fold_node::llm_query::types::ToolCallRecord>,
    },
    Status {
        pub_key: String,
        user_hash: String,
        db_config: DatabaseConfig,
        indexing_status: IndexingStatus,
    },
    Config(DatabaseConfig),
    ConfigPath(String),
    ResetComplete,
    TransformList(HashMap<String, Transform>),
    TransformQueue {
        length: usize,
        queued: Vec<String>,
    },
    TransformStats(EventStatistics),
    BackfillStats(BackfillStatistics),
    Completions(String),
}

pub async fn dispatch(
    command: &Command,
    processor: &OperationProcessor,
    user_hash: &str,
    mode: OutputMode,
    config_path: Option<&str>,
    verbose: bool,
) -> Result<CommandOutput, CliError> {
    match command {
        Command::Schema { action } => schema::run(action, processor, mode).await,
        Command::Query {
            schema,
            fields,
            hash,
            range,
        } => query::run(schema, fields, hash.as_deref(), range.as_deref(), processor).await,
        Command::Search { term } => search::run(term, processor).await,
        Command::Mutate { action } => mutate::run(action, processor).await,
        Command::Ingest { action } => ingest::run(action, processor, mode).await,
        Command::Ask {
            query,
            max_iterations,
        } => ask::run(query, user_hash, *max_iterations, processor, mode).await,
        Command::Status => system::status(processor, user_hash).await,
        Command::Config { action } => {
            system::config(action.as_ref().unwrap_or(&ConfigCommand::Show), processor, config_path).await
        }
        Command::Reset { confirm } => system::reset(*confirm, processor, user_hash, mode).await,
        Command::Transform { action } => transform::run(action, processor).await,
        Command::Backfill { action } => transform::run_backfill(action, processor).await,
        Command::Completions { shell } => completions::run(*shell, verbose),
    }
}
