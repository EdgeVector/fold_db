use crate::cli::{BackfillCommand, TransformCommand};
use crate::commands::CommandOutput;
use crate::error::CliError;
use fold_db::datafold_node::OperationProcessor;

pub async fn run(
    action: &TransformCommand,
    processor: &OperationProcessor,
) -> Result<CommandOutput, CliError> {
    match action {
        TransformCommand::List => {
            let transforms = processor.list_transforms().await?;
            Ok(CommandOutput::TransformList(transforms))
        }
        TransformCommand::Queue => {
            let (length, queued) = processor.get_transform_queue().await?;
            Ok(CommandOutput::TransformQueue { length, queued })
        }
        TransformCommand::Stats => {
            let stats = processor.get_transform_statistics().await?;
            Ok(CommandOutput::TransformStats(stats))
        }
    }
}

pub async fn run_backfill(
    action: &BackfillCommand,
    processor: &OperationProcessor,
) -> Result<CommandOutput, CliError> {
    match action {
        BackfillCommand::Stats => {
            let stats = processor.get_backfill_statistics().await?;
            Ok(CommandOutput::BackfillStats(stats))
        }
    }
}
