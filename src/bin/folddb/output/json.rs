use crate::commands::CommandOutput;
use serde_json::{json, Value};

pub fn render(output: &CommandOutput) {
    let val = to_json(output);
    println!(
        "{}",
        serde_json::to_string(&val)
            .unwrap_or_else(|e| format!("{{\"ok\":false,\"error\":\"{}\"}}", e))
    );
}

fn to_json(output: &CommandOutput) -> Value {
    match output {
        CommandOutput::SchemaList(schemas) => {
            let val = serde_json::to_value(schemas).unwrap_or(Value::Null);
            json!({ "ok": true, "schemas": val })
        }
        CommandOutput::SchemaGet(schema) => {
            let val = serde_json::to_value(schema).unwrap_or(Value::Null);
            json!({ "ok": true, "schema": val })
        }
        CommandOutput::SchemaApproved {
            name,
        } => {
            json!({
                "ok": true,
                "schema": name,
            })
        }
        CommandOutput::SchemaBlocked { name } => {
            json!({ "ok": true, "schema": name })
        }
        CommandOutput::SchemaLoaded {
            available,
            loaded,
            failed,
        } => {
            json!({
                "ok": true,
                "available": available,
                "loaded": loaded,
                "failed": failed,
            })
        }
        CommandOutput::QueryResults(results) => {
            json!({ "ok": true, "results": results })
        }
        CommandOutput::SearchResults(results) => {
            let val = serde_json::to_value(results).unwrap_or(Value::Null);
            json!({ "ok": true, "results": val })
        }
        CommandOutput::MutationSuccess { id } => {
            json!({ "ok": true, "id": id })
        }
        CommandOutput::MutationBatch { ids } => {
            json!({ "ok": true, "ids": ids })
        }
        CommandOutput::IngestSuccess { count, ids } => {
            json!({ "ok": true, "ingested": count, "ids": ids })
        }
        CommandOutput::SmartScan(response) => {
            let val = serde_json::to_value(response).unwrap_or(Value::Null);
            json!({ "ok": true, "scan": val })
        }
        CommandOutput::SmartIngestResults {
            total,
            succeeded,
            failed,
            results,
        } => {
            json!({
                "ok": *succeeded > 0,
                "total": total,
                "succeeded": succeeded,
                "failed": failed,
                "results": results,
            })
        }
        CommandOutput::AskAnswer { answer, tool_calls } => {
            let tool_calls_json: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "tool": tc.tool,
                        "params": tc.params,
                        "result": tc.result,
                    })
                })
                .collect();
            json!({
                "ok": true,
                "answer": answer,
                "tool_calls": tool_calls_json,
            })
        }
        CommandOutput::Status {
            pub_key,
            user_hash,
            db_config,
            indexing_status,
        } => {
            json!({
                "ok": true,
                "node_public_key": pub_key,
                "user_hash": user_hash,
                "database_config": serde_json::to_value(db_config).unwrap_or(Value::Null),
                "indexing_status": serde_json::to_value(indexing_status).unwrap_or(Value::Null),
            })
        }
        CommandOutput::Config(config) => {
            let val = serde_json::to_value(config).unwrap_or(Value::Null);
            json!({ "ok": true, "config": val })
        }
        CommandOutput::ConfigPath(path) => {
            json!({ "ok": true, "path": path })
        }
        CommandOutput::ResetComplete => {
            json!({ "ok": true, "message": "Database reset complete" })
        }
        CommandOutput::MigrateComplete => {
            json!({ "ok": true, "message": "Database migration to cloud complete" })
        }
        CommandOutput::Completions(_) => {
            json!({ "ok": true, "message": "Completions written to stdout" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandOutput;

    fn assert_ok(output: &CommandOutput) {
        let val = to_json(output);
        assert_eq!(val["ok"], true, "Expected ok:true for {:?}", output);
    }

    #[test]
    fn json_schema_list() {
        assert_ok(&CommandOutput::SchemaList(vec![]));
    }

    #[test]
    fn json_schema_approved() {
        assert_ok(&CommandOutput::SchemaApproved {
            name: "test".into(),
        });
    }

    #[test]
    fn json_schema_blocked() {
        assert_ok(&CommandOutput::SchemaBlocked {
            name: "test".into(),
        });
    }

    #[test]
    fn json_schema_loaded() {
        assert_ok(&CommandOutput::SchemaLoaded {
            available: 5,
            loaded: 3,
            failed: vec!["x".into()],
        });
    }

    #[test]
    fn json_query_results() {
        assert_ok(&CommandOutput::QueryResults(vec![]));
    }

    #[test]
    fn json_search_results() {
        assert_ok(&CommandOutput::SearchResults(vec![]));
    }

    #[test]
    fn json_mutation_success() {
        assert_ok(&CommandOutput::MutationSuccess { id: "abc".into() });
    }

    #[test]
    fn json_mutation_batch() {
        assert_ok(&CommandOutput::MutationBatch {
            ids: vec!["a".into()],
        });
    }

    #[test]
    fn json_ingest_success() {
        assert_ok(&CommandOutput::IngestSuccess {
            count: 1,
            ids: vec!["a".into()],
        });
    }

    #[test]
    fn json_reset_complete() {
        assert_ok(&CommandOutput::ResetComplete);
    }

    #[test]
    fn json_migrate_complete() {
        assert_ok(&CommandOutput::MigrateComplete);
    }

    #[test]
    fn json_config_path() {
        assert_ok(&CommandOutput::ConfigPath("/tmp/config.toml".into()));
    }

    #[test]
    fn json_completions() {
        assert_ok(&CommandOutput::Completions("# bash completions".into()));
    }
}
