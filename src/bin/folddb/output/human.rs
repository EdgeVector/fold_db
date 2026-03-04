use crate::commands::CommandOutput;
use comfy_table::{ContentArrangement, Table};
use console::style;

pub fn render(output: &CommandOutput) {
    match output {
        CommandOutput::SchemaList(schemas) => {
            if schemas.is_empty() {
                println!("No schemas found.");
                return;
            }
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["Name", "State", "Fields"]);
            for s in schemas {
                let field_count = s.schema.fields.as_ref().map_or(0, |f| f.len());
                table.add_row(vec![
                    s.name().to_string(),
                    format!("{:?}", s.state),
                    field_count.to_string(),
                ]);
            }
            println!("{table}");
        }

        CommandOutput::SchemaGet(schema) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema).unwrap_or_else(|_| format!("{:?}", schema))
            );
        }

        CommandOutput::SchemaApproved {
            name,
        } => {
            println!(
                "{} Schema '{}' approved",
                style("\u{2713}").green().bold(),
                style(name).bold()
            );
        }

        CommandOutput::SchemaBlocked { name } => {
            println!(
                "{} Schema '{}' blocked",
                style("\u{2713}").green().bold(),
                style(name).bold()
            );
        }

        CommandOutput::SchemaLoaded {
            available,
            loaded,
            failed,
        } => {
            println!(
                "{} Loaded schemas from service",
                style("\u{2713}").green().bold()
            );
            println!("  Available: {}", available);
            println!("  Loaded:    {}", loaded);
            if !failed.is_empty() {
                println!("  Failed:    {} ({})", failed.len(), failed.join(", "));
            }
        }

        CommandOutput::QueryResults(results) => {
            if results.is_empty() {
                println!("No results.");
                return;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&results).unwrap_or_else(|_| format!("{:?}", results))
            );
        }

        CommandOutput::SearchResults(results) => {
            if results.is_empty() {
                println!("No results.");
                return;
            }
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["Schema", "Field", "Value"]);
            for r in results {
                let value_str = match &r.value {
                    serde_json::Value::String(s) => truncate(s, 60),
                    other => truncate(&other.to_string(), 60),
                };
                table.add_row(vec![r.schema_name.clone(), r.field.clone(), value_str]);
            }
            println!("{table}");
        }

        CommandOutput::MutationSuccess { id } => {
            println!(
                "{} Mutation applied (id: {})",
                style("\u{2713}").green().bold(),
                id
            );
        }

        CommandOutput::MutationBatch { ids } => {
            println!(
                "{} {} mutations applied",
                style("\u{2713}").green().bold(),
                ids.len()
            );
            for id in ids {
                println!("  {}", id);
            }
        }

        CommandOutput::IngestSuccess { count, ids } => {
            println!(
                "{} Ingested {} record{}",
                style("\u{2713}").green().bold(),
                count,
                if *count == 1 { "" } else { "s" }
            );
            for id in ids {
                println!("  {}", id);
            }
        }

        CommandOutput::SmartScan(response) => {
            println!(
                "{} Scan complete ({} files found)",
                style("\u{2713}").green().bold(),
                response.total_files
            );
            if !response.recommended_files.is_empty() {
                println!(
                    "\n{} recommended for ingestion:",
                    response.recommended_files.len()
                );
                let mut table = Table::new();
                table.set_content_arrangement(ContentArrangement::Dynamic);
                table.set_header(vec!["File", "Category", "Reason"]);
                for f in &response.recommended_files {
                    table.add_row(vec![
                        f.path.clone(),
                        f.category.clone(),
                        truncate(&f.reason, 50),
                    ]);
                }
                println!("{table}");
            }
            if !response.skipped_files.is_empty() {
                println!("\n{} files skipped", response.skipped_files.len());
            }
        }

        CommandOutput::SmartIngestResults {
            total,
            succeeded,
            failed,
            results,
        } => {
            println!(
                "{} Ingestion complete: {}/{} succeeded",
                if *failed == 0 {
                    style("\u{2713}").green().bold()
                } else {
                    style("!").yellow().bold()
                },
                succeeded,
                total,
            );
            if *failed > 0 {
                println!("  {} failed", failed);
            }
            for r in results {
                let status = if r["success"] == true {
                    "\u{2713}"
                } else {
                    "\u{2717}"
                };
                let file = r["file"].as_str().unwrap_or("?");
                println!("  {} {}", status, file);
                if let Some(err) = r["error"].as_str() {
                    println!("    {}", style(err).red());
                }
            }
        }

        CommandOutput::AskAnswer { answer, tool_calls } => {
            if !tool_calls.is_empty() {
                println!(
                    "{} Done ({} tool call{})\n",
                    style("\u{2713}").green().bold(),
                    tool_calls.len(),
                    if tool_calls.len() == 1 { "" } else { "s" }
                );
            }
            println!("{}", answer);
        }

        CommandOutput::Status {
            pub_key,
            user_hash,
            db_config,
            indexing_status,
        } => {
            println!("{}  {}", style("Node Public Key:").bold(), pub_key);
            println!("{}        {}", style("User Hash:").bold(), user_hash);
            let db_str = match db_config {
                fold_db::DatabaseConfig::Local { path } => {
                    format!("Local ({})", path.display())
                }
                #[cfg(feature = "aws-backend")]
                fold_db::DatabaseConfig::Cloud(_) => "Cloud".to_string(),
                fold_db::DatabaseConfig::Exemem { api_url, .. } => {
                    format!("Exemem ({})", api_url)
                }
            };
            println!("{}         {}", style("Database:").bold(), db_str);
            let idx_str = format!(
                "{:?} ({} documents indexed)",
                indexing_status.state, indexing_status.total_operations_processed
            );
            println!("{}         {}", style("Indexing:").bold(), idx_str);
        }

        CommandOutput::Config(config) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&config).unwrap_or_else(|_| format!("{:?}", config))
            );
        }

        CommandOutput::ConfigPath(path) => {
            println!("{}", path);
        }

        CommandOutput::ResetComplete => {
            println!(
                "{} Database reset complete",
                style("\u{2713}").green().bold()
            );
        }

        CommandOutput::MigrateComplete => {
            println!(
                "{} Database migration to cloud complete",
                style("\u{2713}").green().bold()
            );
        }

        CommandOutput::Completions(script) => {
            print!("{}", script);
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        let long = "a".repeat(100);
        let result = truncate(&long, 20);
        assert_eq!(result.len(), 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn human_schema_list_empty() {
        // Just verify it doesn't panic
        render(&CommandOutput::SchemaList(vec![]));
    }

    #[test]
    fn human_reset_complete() {
        render(&CommandOutput::ResetComplete);
    }

    #[test]
    fn human_migrate_complete() {
        render(&CommandOutput::MigrateComplete);
    }

    #[test]
    fn human_config_path() {
        render(&CommandOutput::ConfigPath("/tmp/config.toml".into()));
    }

    #[test]
    fn human_completions() {
        render(&CommandOutput::Completions("# completions".into()));
    }

    #[test]
    fn human_query_results_empty() {
        render(&CommandOutput::QueryResults(vec![]));
    }

    #[test]
    fn human_search_results_empty() {
        render(&CommandOutput::SearchResults(vec![]));
    }

    #[test]
    fn human_mutation_success() {
        render(&CommandOutput::MutationSuccess {
            id: "test-id".into(),
        });
    }

    #[test]
    fn human_mutation_batch() {
        render(&CommandOutput::MutationBatch {
            ids: vec!["a".into(), "b".into()],
        });
    }

    #[test]
    fn human_ingest_success_singular() {
        render(&CommandOutput::IngestSuccess {
            count: 1,
            ids: vec!["a".into()],
        });
    }

    #[test]
    fn human_ingest_success_plural() {
        render(&CommandOutput::IngestSuccess {
            count: 3,
            ids: vec!["a".into(), "b".into(), "c".into()],
        });
    }

    #[test]
    fn human_schema_approved() {
        render(&CommandOutput::SchemaApproved {
            name: "test".into(),
        });
    }

    #[test]
    fn human_schema_blocked() {
        render(&CommandOutput::SchemaBlocked {
            name: "test".into(),
        });
    }

    #[test]
    fn human_schema_loaded() {
        render(&CommandOutput::SchemaLoaded {
            available: 5,
            loaded: 3,
            failed: vec!["x".into()],
        });
    }

    #[test]
    fn human_ask_answer() {
        render(&CommandOutput::AskAnswer {
            answer: "The answer is 42".into(),
            tool_calls: vec![],
        });
    }
}
