use crate::cli::SchemaCommand;
use crate::commands::CommandOutput;
use crate::error::CliError;
use crate::output::spinner;
use crate::output::OutputMode;
use fold_db::fold_node::OperationProcessor;

pub async fn run(
    action: &SchemaCommand,
    processor: &OperationProcessor,
    mode: OutputMode,
) -> Result<CommandOutput, CliError> {
    match action {
        SchemaCommand::List => {
            let schemas = processor.list_schemas().await?;
            Ok(CommandOutput::SchemaList(schemas))
        }
        SchemaCommand::Get { name } => {
            let schema = processor.get_schema(name).await?;
            match schema {
                Some(s) => Ok(CommandOutput::SchemaGet(Box::new(s))),
                None => Err(CliError::new(format!("Schema '{}' not found", name))
                    .with_hint("Run 'folddb schema list' to see all schemas.")),
            }
        }
        SchemaCommand::Approve { name } => {
            let backfill_hash = processor.approve_schema(name).await?;
            Ok(CommandOutput::SchemaApproved {
                name: name.clone(),
                backfill_hash,
            })
        }
        SchemaCommand::Block { name } => {
            processor.block_schema(name).await?;
            Ok(CommandOutput::SchemaBlocked { name: name.clone() })
        }
        SchemaCommand::Load => {
            let sp = if mode == OutputMode::Human {
                Some(spinner::new_spinner("Loading schemas from service..."))
            } else {
                None
            };
            let result = processor.load_schemas().await;
            if let Some(ref pb) = sp {
                spinner::finish_spinner(pb, "Schemas loaded");
            }
            let (available, loaded, failed) = result?;
            Ok(CommandOutput::SchemaLoaded {
                available,
                loaded,
                failed,
            })
        }
    }
}
