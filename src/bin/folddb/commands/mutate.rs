use crate::cli::MutateCommand;
use crate::commands::CommandOutput;
use crate::error::CliError;
use fold_db::datafold_node::OperationProcessor;
use fold_db::schema::types::key_value::KeyValue;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

pub async fn run(
    action: &MutateCommand,
    processor: &OperationProcessor,
) -> Result<CommandOutput, CliError> {
    match action {
        MutateCommand::Run {
            schema,
            r#type,
            fields,
            hash,
            range,
        } => {
            let fields_map: HashMap<String, Value> = serde_json::from_str(fields)
                .map_err(|e| CliError::new(format!("Invalid fields JSON: {}", e)))?;
            let key_value = KeyValue::new(hash.clone(), range.clone());
            let id = processor
                .execute_mutation(
                    schema.clone(),
                    fields_map,
                    key_value,
                    r#type.clone(),
                )
                .await?;
            Ok(CommandOutput::MutationSuccess { id })
        }
        MutateCommand::Batch { file } => {
            let data = read_json_from_file_or_stdin(file.as_ref())?;
            let mutations: Vec<Value> = match data {
                Value::Array(arr) => arr,
                other => vec![other],
            };
            let ids = processor.execute_mutations_batch(mutations).await?;
            Ok(CommandOutput::MutationBatch { ids })
        }
    }
}

fn read_json_from_file_or_stdin(file: Option<&PathBuf>) -> Result<Value, CliError> {
    let content = match file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| CliError::new(format!("Failed to read {}: {}", path.display(), e)))?,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| CliError::new(format!("Failed to read stdin: {}", e)))?;
            buf
        }
    };
    serde_json::from_str(&content)
        .map_err(|e| CliError::new(format!("Invalid JSON: {}", e)))
}
