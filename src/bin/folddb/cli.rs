use clap::{Parser, Subcommand};
use clap_complete::Shell;
use fold_db::schema::types::operations::MutationType;
use std::path::PathBuf;

/// FoldDB CLI - human-first command-line access to FoldDB
#[derive(Parser, Debug)]
#[command(name = "folddb", author, version, about)]
pub struct Cli {
    /// Output all results as JSON (for scripting)
    #[arg(long, global = true)]
    pub json: bool,

    /// Show verbose/debug output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to node config file (also reads NODE_CONFIG env var)
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// User hash for data isolation (also reads FOLD_USER_HASH env var)
    #[arg(long, global = true)]
    pub user_hash: Option<String>,

    /// Override: local Sled database directory
    #[arg(long, global = true)]
    pub data_path: Option<PathBuf>,

    /// Override: schema service URL
    #[arg(long, global = true)]
    pub schema_service_url: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage schemas
    Schema {
        #[command(subcommand)]
        action: SchemaCommand,
    },

    /// Execute a query against a schema
    Query {
        /// Schema name
        schema: String,

        /// Fields to query (comma-separated)
        #[arg(long)]
        fields: String,

        /// Filter hash key
        #[arg(long)]
        hash: Option<String>,

        /// Filter range key
        #[arg(long)]
        range: Option<String>,
    },

    /// Search the native word index
    Search {
        /// Search term
        term: String,
    },

    /// Execute mutations
    Mutate {
        #[command(subcommand)]
        action: MutateCommand,
    },

    /// Ingest data into FoldDB
    Ingest {
        #[command(subcommand)]
        action: IngestCommand,
    },

    /// Ask a natural-language question (LLM agent)
    Ask {
        /// The natural language query
        query: String,

        /// Maximum agent iterations (default: 10)
        #[arg(long, default_value = "10")]
        max_iterations: usize,
    },

    /// Show node status (key, user hash, config, indexing)
    Status,

    /// Show or inspect configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigCommand>,
    },

    /// Reset the database (destructive)
    Reset {
        /// Skip interactive confirmation
        #[arg(long)]
        confirm: bool,
    },

    /// Migrate the local database to the cloud
    MigrateToCloud {
        /// Target cloud API URL
        #[arg(long)]
        api_url: String,

        /// Target cloud API Key
        #[arg(long)]
        api_key: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum SchemaCommand {
    /// List all schemas with their states
    List,
    /// Get a specific schema by name
    Get {
        /// Schema name
        name: String,
    },
    /// Approve a schema
    Approve {
        /// Schema name
        name: String,
    },
    /// Block a schema
    Block {
        /// Schema name
        name: String,
    },
    /// Load schemas from schema service
    Load,
}

#[derive(Subcommand, Debug)]
pub enum MutateCommand {
    /// Execute a single mutation
    Run {
        /// Schema name
        schema: String,

        /// Mutation type
        #[arg(long, value_enum)]
        r#type: MutationType,

        /// Fields as JSON object (e.g. '{"name":"value"}')
        #[arg(long)]
        fields: String,

        /// Hash key
        #[arg(long)]
        hash: Option<String>,

        /// Range key
        #[arg(long)]
        range: Option<String>,
    },
    /// Execute batch mutations from JSON file or stdin
    Batch {
        /// Path to JSON file (reads stdin if omitted)
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum IngestCommand {
    /// Ingest JSON data from a file or stdin
    File {
        /// Path to JSON file (reads stdin if omitted)
        path: Option<PathBuf>,
    },
    /// Scan a folder using AI to classify files for ingestion
    SmartScan {
        /// Path to the folder to scan
        path: PathBuf,

        /// Maximum directory depth to scan (default: 5)
        #[arg(long, default_value = "5")]
        max_depth: usize,

        /// Maximum number of files to analyze (default: 500)
        #[arg(long, default_value = "500")]
        max_files: usize,
    },
    /// Ingest files from a folder using AI recommendations
    Smart {
        /// Base folder path
        path: PathBuf,

        /// First scan, then ingest all recommended files
        #[arg(long)]
        all: bool,

        /// Specific files to ingest (comma-separated, relative to folder)
        #[arg(long, value_delimiter = ',')]
        files: Option<Vec<String>>,

        /// Disable auto-execution of mutations
        #[arg(long)]
        no_execute: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Print the config file path
    Path,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_schema_list() {
        let cli = Cli::parse_from(["folddb", "schema", "list"]);
        match cli.command {
            Command::Schema {
                action: SchemaCommand::List,
            } => {}
            _ => panic!("Expected Schema List"),
        }
    }

    #[test]
    fn parse_schema_get() {
        let cli = Cli::parse_from(["folddb", "schema", "get", "my_schema"]);
        match cli.command {
            Command::Schema {
                action: SchemaCommand::Get { name },
            } => assert_eq!(name, "my_schema"),
            _ => panic!("Expected Schema Get"),
        }
    }

    #[test]
    fn parse_schema_approve() {
        let cli = Cli::parse_from(["folddb", "schema", "approve", "tweets"]);
        match cli.command {
            Command::Schema {
                action: SchemaCommand::Approve { name },
            } => assert_eq!(name, "tweets"),
            _ => panic!("Expected Schema Approve"),
        }
    }

    #[test]
    fn parse_schema_block() {
        let cli = Cli::parse_from(["folddb", "schema", "block", "bad_schema"]);
        match cli.command {
            Command::Schema {
                action: SchemaCommand::Block { name },
            } => assert_eq!(name, "bad_schema"),
            _ => panic!("Expected Schema Block"),
        }
    }

    #[test]
    fn parse_schema_load() {
        let cli = Cli::parse_from(["folddb", "schema", "load"]);
        match cli.command {
            Command::Schema {
                action: SchemaCommand::Load,
            } => {}
            _ => panic!("Expected Schema Load"),
        }
    }

    #[test]
    fn parse_query() {
        let cli = Cli::parse_from([
            "folddb",
            "query",
            "tweets",
            "--fields",
            "text,author",
            "--hash",
            "abc",
        ]);
        match cli.command {
            Command::Query {
                schema,
                fields,
                hash,
                range,
            } => {
                assert_eq!(schema, "tweets");
                assert_eq!(fields, "text,author");
                assert_eq!(hash, Some("abc".to_string()));
                assert!(range.is_none());
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn parse_search() {
        let cli = Cli::parse_from(["folddb", "search", "hello world"]);
        match cli.command {
            Command::Search { term } => assert_eq!(term, "hello world"),
            _ => panic!("Expected Search"),
        }
    }

    #[test]
    fn parse_mutate_run() {
        let cli = Cli::parse_from([
            "folddb",
            "mutate",
            "run",
            "tweets",
            "--type",
            "create",
            "--fields",
            r#"{"text":"hello"}"#,
        ]);
        match cli.command {
            Command::Mutate {
                action:
                    MutateCommand::Run {
                        schema,
                        r#type,
                        fields,
                        ..
                    },
            } => {
                assert_eq!(schema, "tweets");
                assert_eq!(r#type, MutationType::Create);
                assert!(fields.contains("hello"));
            }
            _ => panic!("Expected Mutate Run"),
        }
    }

    #[test]
    fn parse_mutate_batch_with_file() {
        let cli = Cli::parse_from(["folddb", "mutate", "batch", "data.json"]);
        match cli.command {
            Command::Mutate {
                action: MutateCommand::Batch { file },
            } => assert_eq!(file, Some(PathBuf::from("data.json"))),
            _ => panic!("Expected Mutate Batch"),
        }
    }

    #[test]
    fn parse_mutate_batch_no_file() {
        let cli = Cli::parse_from(["folddb", "mutate", "batch"]);
        match cli.command {
            Command::Mutate {
                action: MutateCommand::Batch { file },
            } => assert!(file.is_none()),
            _ => panic!("Expected Mutate Batch"),
        }
    }

    #[test]
    fn parse_ingest_file() {
        let cli = Cli::parse_from(["folddb", "ingest", "file", "input.json"]);
        match cli.command {
            Command::Ingest {
                action: IngestCommand::File { path },
            } => assert_eq!(path, Some(PathBuf::from("input.json"))),
            _ => panic!("Expected Ingest File"),
        }
    }

    #[test]
    fn parse_ingest_file_no_path() {
        let cli = Cli::parse_from(["folddb", "ingest", "file"]);
        match cli.command {
            Command::Ingest {
                action: IngestCommand::File { path },
            } => assert!(path.is_none()),
            _ => panic!("Expected Ingest File"),
        }
    }

    #[test]
    fn parse_ingest_smart_scan() {
        let cli = Cli::parse_from(["folddb", "ingest", "smart-scan", "/tmp/data"]);
        match cli.command {
            Command::Ingest {
                action:
                    IngestCommand::SmartScan {
                        path,
                        max_depth,
                        max_files,
                    },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/data"));
                assert_eq!(max_depth, 5);
                assert_eq!(max_files, 500);
            }
            _ => panic!("Expected Ingest SmartScan"),
        }
    }

    #[test]
    fn parse_ingest_smart_scan_with_options() {
        let cli = Cli::parse_from([
            "folddb",
            "ingest",
            "smart-scan",
            "/tmp/data",
            "--max-depth",
            "3",
            "--max-files",
            "100",
        ]);
        match cli.command {
            Command::Ingest {
                action:
                    IngestCommand::SmartScan {
                        max_depth,
                        max_files,
                        ..
                    },
            } => {
                assert_eq!(max_depth, 3);
                assert_eq!(max_files, 100);
            }
            _ => panic!("Expected Ingest SmartScan"),
        }
    }

    #[test]
    fn parse_ingest_smart_all() {
        let cli = Cli::parse_from(["folddb", "ingest", "smart", "/tmp/data", "--all"]);
        match cli.command {
            Command::Ingest {
                action:
                    IngestCommand::Smart {
                        path,
                        all,
                        no_execute,
                        files,
                    },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/data"));
                assert!(all);
                assert!(!no_execute);
                assert!(files.is_none());
            }
            _ => panic!("Expected Ingest Smart"),
        }
    }

    #[test]
    fn parse_ingest_smart_with_files() {
        let cli = Cli::parse_from([
            "folddb",
            "ingest",
            "smart",
            "/tmp/data",
            "--files",
            "a.json,b.csv",
            "--no-execute",
        ]);
        match cli.command {
            Command::Ingest {
                action:
                    IngestCommand::Smart {
                        files,
                        no_execute,
                        all,
                        ..
                    },
            } => {
                assert_eq!(files, Some(vec!["a.json".to_string(), "b.csv".to_string()]));
                assert!(no_execute);
                assert!(!all);
            }
            _ => panic!("Expected Ingest Smart"),
        }
    }

    #[test]
    fn parse_ask() {
        let cli = Cli::parse_from(["folddb", "ask", "What is my data about?"]);
        match cli.command {
            Command::Ask {
                query,
                max_iterations,
            } => {
                assert_eq!(query, "What is my data about?");
                assert_eq!(max_iterations, 10);
            }
            _ => panic!("Expected Ask"),
        }
    }

    #[test]
    fn parse_ask_with_max_iterations() {
        let cli = Cli::parse_from(["folddb", "ask", "test", "--max-iterations", "5"]);
        match cli.command {
            Command::Ask { max_iterations, .. } => assert_eq!(max_iterations, 5),
            _ => panic!("Expected Ask"),
        }
    }

    #[test]
    fn parse_status() {
        let cli = Cli::parse_from(["folddb", "status"]);
        assert!(matches!(cli.command, Command::Status));
    }

    #[test]
    fn parse_config_show() {
        let cli = Cli::parse_from(["folddb", "config", "show"]);
        match cli.command {
            Command::Config {
                action: Some(ConfigCommand::Show),
            } => {}
            _ => panic!("Expected Config Show"),
        }
    }

    #[test]
    fn parse_config_path() {
        let cli = Cli::parse_from(["folddb", "config", "path"]);
        match cli.command {
            Command::Config {
                action: Some(ConfigCommand::Path),
            } => {}
            _ => panic!("Expected Config Path"),
        }
    }

    #[test]
    fn parse_config_bare() {
        let cli = Cli::parse_from(["folddb", "config"]);
        match cli.command {
            Command::Config { action: None } => {}
            _ => panic!("Expected Config with no subcommand"),
        }
    }

    #[test]
    fn parse_reset_with_confirm() {
        let cli = Cli::parse_from(["folddb", "reset", "--confirm"]);
        match cli.command {
            Command::Reset { confirm } => assert!(confirm),
            _ => panic!("Expected Reset"),
        }
    }

    #[test]
    fn parse_reset_without_confirm() {
        let cli = Cli::parse_from(["folddb", "reset"]);
        match cli.command {
            Command::Reset { confirm } => assert!(!confirm),
            _ => panic!("Expected Reset"),
        }
    }

    #[test]
    fn parse_completions() {
        let cli = Cli::parse_from(["folddb", "completions", "bash"]);
        match cli.command {
            Command::Completions { shell } => assert_eq!(shell, Shell::Bash),
            _ => panic!("Expected Completions"),
        }
    }

    #[test]
    fn parse_json_flag() {
        let cli = Cli::parse_from(["folddb", "--json", "status"]);
        assert!(cli.json);
        assert!(matches!(cli.command, Command::Status));
    }

    #[test]
    fn parse_verbose_flag() {
        let cli = Cli::parse_from(["folddb", "-v", "status"]);
        assert!(cli.verbose);
    }

    #[test]
    fn parse_data_path() {
        let cli = Cli::parse_from(["folddb", "--data-path", "/tmp/mydb", "status"]);
        assert_eq!(cli.data_path, Some(PathBuf::from("/tmp/mydb")));
    }

    #[test]
    fn parse_user_hash() {
        let cli = Cli::parse_from(["folddb", "--user-hash", "abc123", "status"]);
        assert_eq!(cli.user_hash, Some("abc123".to_string()));
    }

    #[test]
    fn parse_schema_service_url() {
        let cli = Cli::parse_from([
            "folddb",
            "--schema-service-url",
            "http://localhost:9002",
            "schema",
            "load",
        ]);
        assert_eq!(
            cli.schema_service_url,
            Some("http://localhost:9002".to_string())
        );
    }

    #[test]
    fn parse_all_subcommands_exist() {
        let commands: Vec<Vec<&str>> = vec![
            vec!["folddb", "schema", "list"],
            vec!["folddb", "schema", "get", "x"],
            vec!["folddb", "schema", "approve", "x"],
            vec!["folddb", "schema", "block", "x"],
            vec!["folddb", "schema", "load"],
            vec!["folddb", "query", "s", "--fields", "f"],
            vec!["folddb", "search", "t"],
            vec![
                "folddb", "mutate", "run", "s", "--type", "create", "--fields", "{}",
            ],
            vec!["folddb", "mutate", "batch"],
            vec!["folddb", "ingest", "file"],
            vec!["folddb", "ingest", "smart-scan", "/tmp"],
            vec!["folddb", "ingest", "smart", "/tmp", "--all"],
            vec!["folddb", "ask", "test question"],
            vec!["folddb", "status"],
            vec!["folddb", "config"],
            vec!["folddb", "config", "show"],
            vec!["folddb", "config", "path"],
            vec!["folddb", "reset"],
            vec!["folddb", "completions", "bash"],
        ];

        for args in &commands {
            let result = Cli::try_parse_from(args.iter());
            assert!(
                result.is_ok(),
                "Failed to parse: {:?}: {}",
                args,
                result.unwrap_err()
            );
        }
    }
}
