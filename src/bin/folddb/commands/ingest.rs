use crate::cli::IngestCommand;
use crate::commands::CommandOutput;
use crate::error::CliError;
use crate::output::spinner;
use crate::output::OutputMode;
use fold_db::datafold_node::OperationProcessor;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub async fn run(
    action: &IngestCommand,
    processor: &OperationProcessor,
    mode: OutputMode,
) -> Result<CommandOutput, CliError> {
    match action {
        IngestCommand::File { path } => {
            let data = read_json_from_file_or_stdin(path.as_ref())?;
            let mutations: Vec<Value> = match data {
                Value::Array(arr) => arr,
                other => vec![other],
            };
            let ids = processor.execute_mutations_batch(mutations).await?;
            Ok(CommandOutput::IngestSuccess {
                count: ids.len(),
                ids,
            })
        }
        IngestCommand::SmartScan {
            path,
            max_depth,
            max_files,
        } => {
            let sp = if mode == OutputMode::Human {
                Some(spinner::new_spinner("Scanning folder..."))
            } else {
                None
            };
            let result = processor.smart_folder_scan(path, *max_depth, *max_files).await;
            if let Some(ref pb) = sp {
                spinner::finish_spinner(pb, "Scan complete");
            }
            let response = result?;
            Ok(CommandOutput::SmartScan(response))
        }
        IngestCommand::Smart {
            path,
            all,
            files,
            no_execute,
        } => {
            let auto_execute = !no_execute;
            let files_to_ingest = resolve_files(path, *all, files.as_ref(), processor, mode).await?;

            if files_to_ingest.is_empty() {
                return Err(CliError::new("No files to ingest"));
            }

            let total = files_to_ingest.len();
            let pb = if mode == OutputMode::Human {
                Some(spinner::new_progress_bar(total as u64, "Ingesting"))
            } else {
                None
            };

            let mut results = Vec::new();
            for (i, relative_path) in files_to_ingest.iter().enumerate() {
                let full_path = path.join(relative_path);
                if let Some(ref pb) = pb {
                    pb.set_message(relative_path.clone());
                    pb.set_position((i + 1) as u64);
                }

                match processor.ingest_single_file(&full_path, auto_execute).await {
                    Ok(response) => {
                        results.push(serde_json::json!({
                            "file": relative_path,
                            "success": response.success,
                            "schema_used": response.schema_used,
                            "new_schema_created": response.new_schema_created,
                            "mutations_generated": response.mutations_generated,
                            "mutations_executed": response.mutations_executed,
                            "errors": response.errors,
                        }));
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "file": relative_path,
                            "success": false,
                            "error": e.to_string(),
                        }));
                    }
                }
            }

            if let Some(ref pb) = pb {
                pb.finish_and_clear();
            }

            let succeeded = results.iter().filter(|r| r["success"] == true).count();
            Ok(CommandOutput::SmartIngestResults {
                total,
                succeeded,
                failed: total - succeeded,
                results,
            })
        }
    }
}

async fn resolve_files(
    folder_path: &Path,
    all: bool,
    files: Option<&Vec<String>>,
    processor: &OperationProcessor,
    mode: OutputMode,
) -> Result<Vec<String>, CliError> {
    if all {
        let sp = if mode == OutputMode::Human {
            Some(spinner::new_spinner("Scanning folder..."))
        } else {
            None
        };
        let scan = processor.smart_folder_scan(folder_path, 5, 500).await?;
        if let Some(ref pb) = sp {
            spinner::finish_spinner(
                pb,
                &format!(
                    "Found {} recommended files out of {} total",
                    scan.recommended_files.len(),
                    scan.total_files
                ),
            );
        }
        Ok(scan.recommended_files.into_iter().map(|r| r.path).collect())
    } else if let Some(file_list) = files {
        Ok(file_list.clone())
    } else {
        Err(CliError::new("Specify --files or --all"))
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
