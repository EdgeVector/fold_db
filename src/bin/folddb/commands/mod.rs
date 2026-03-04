pub mod ask;
pub mod completions;
pub mod ingest;
pub mod mutate;
pub mod query;
pub mod schema;
pub mod search;
pub mod setup;
pub mod system;

use crate::cli::{Command, ConfigCommand};
use crate::error::CliError;
use crate::output::OutputMode;
use fold_db::db_operations::native_index::IndexResult;
use fold_db::fold_db_core::orchestration::index_status::IndexingStatus;
use fold_db::fold_node::OperationProcessor;
use fold_db::ingestion::smart_folder::SmartFolderScanResponse;
use fold_db::schema::schema_types::SchemaWithState;
use fold_db::storage::DatabaseConfig;
use serde_json::Value;

#[derive(Debug)]
pub enum CommandOutput {
    SchemaList(Vec<SchemaWithState>),
    SchemaGet(Box<SchemaWithState>),
    SchemaApproved {
        name: String,
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
    MigrateComplete,
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
            system::config(
                action.as_ref().unwrap_or(&ConfigCommand::Show),
                processor,
                config_path,
            )
            .await
        }
        Command::Reset { confirm } => system::reset(*confirm, processor, user_hash, mode).await,
        Command::MigrateToCloud { api_url, api_key } => {
            system::migrate_to_cloud(api_url, api_key, processor).await
        }
        Command::Completions { shell } => completions::run(*shell, verbose),
    }
}
