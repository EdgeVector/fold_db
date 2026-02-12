use crate::cli::ConfigCommand;
use crate::commands::CommandOutput;
use crate::error::CliError;
use crate::output::OutputMode;
use fold_db::fold_node::OperationProcessor;

pub async fn status(
    processor: &OperationProcessor,
    user_hash: &str,
) -> Result<CommandOutput, CliError> {
    let pub_key = processor.get_node_public_key();
    let db_config = processor.get_database_config();
    let indexing_status = processor.get_indexing_status().await?;
    Ok(CommandOutput::Status {
        pub_key,
        user_hash: user_hash.to_string(),
        db_config,
        indexing_status,
    })
}

pub async fn config(
    action: &ConfigCommand,
    processor: &OperationProcessor,
    config_path: Option<&str>,
) -> Result<CommandOutput, CliError> {
    match action {
        ConfigCommand::Show => {
            let db_config = processor.get_database_config();
            Ok(CommandOutput::Config(db_config))
        }
        ConfigCommand::Path => {
            let path = config_path
                .map(|p| p.to_string())
                .or_else(|| std::env::var("NODE_CONFIG").ok())
                .unwrap_or_else(|| "node_config.toml".to_string());
            Ok(CommandOutput::ConfigPath(path))
        }
    }
}

pub async fn reset(
    confirm: bool,
    processor: &OperationProcessor,
    user_hash: &str,
    mode: OutputMode,
) -> Result<CommandOutput, CliError> {
    if !confirm {
        if mode == OutputMode::Json {
            return Err(CliError::new("Database reset requires --confirm flag"));
        }
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("This will permanently delete all data. Are you sure?")
            .default(false)
            .interact()
            .map_err(|e| CliError::new(format!("Prompt failed: {}", e)))?;
        if !confirmed {
            return Err(CliError::new("Reset cancelled"));
        }
    }
    processor.perform_database_reset(Some(user_hash)).await?;
    Ok(CommandOutput::ResetComplete)
}
