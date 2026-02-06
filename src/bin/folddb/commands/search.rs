use crate::commands::CommandOutput;
use crate::error::CliError;
use fold_db::datafold_node::OperationProcessor;

pub async fn run(term: &str, processor: &OperationProcessor) -> Result<CommandOutput, CliError> {
    let results = processor.native_index_search(term).await?;
    Ok(CommandOutput::SearchResults(results))
}
