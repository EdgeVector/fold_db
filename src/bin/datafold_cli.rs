use clap::{Parser, Subcommand};
use datafold::schema::SchemaHasher;
use datafold::{load_node_config, DataFoldNode, MutationType, Operation, OperationProcessor, SchemaState};
use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to the node configuration file
    #[arg(short, long, default_value = "config/node_config.json")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load a schema from a JSON file
    LoadSchema {
        /// Path to the schema JSON file
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Add a new schema to the available_schemas directory
    AddSchema {
        /// Path to the schema JSON file to add
        #[arg(required = true)]
        path: PathBuf,
        /// Optional custom name for the schema (defaults to filename)
        #[arg(long, short)]
        name: Option<String>,
    },
    /// Hash all schemas in the available_schemas directory
    HashSchemas {
        /// Verify existing hashes instead of updating them
        #[arg(long, short)]
        verify: bool,
    },
    /// List all loaded schemas
    ListSchemas {},
    /// List all schemas available on disk
    ListAvailableSchemas {},
    /// Unload a schema
    UnloadSchema {
        /// Schema name to unload
        #[arg(long, short, required = true)]
        name: String,
    },
    /// Allow operations on a schema (loads it if unloaded)
    AllowSchema {
        /// Schema name to allow
        #[arg(long, short, required = true)]
        name: String,
    },
    /// Approve a schema for queries and mutations
    ApproveSchema {
        /// Schema name to approve
        #[arg(long, short, required = true)]
        name: String,
    },
    /// Block a schema from queries and mutations
    BlockSchema {
        /// Schema name to block
        #[arg(long, short, required = true)]
        name: String,
    },
    /// Get the current state of a schema
    GetSchemaState {
        /// Schema name to check
        #[arg(long, short, required = true)]
        name: String,
    },
    /// List schemas by state
    ListSchemasByState {
        /// State to filter by (available, approved, blocked)
        #[arg(long, short, required = true)]
        state: String,
    },
    /// Execute a query operation
    Query {
        /// Schema name to query
        #[arg(short, long, required = true)]
        schema: String,

        /// Fields to retrieve (comma-separated)
        #[arg(short, long, required = true, value_delimiter = ',')]
        fields: Vec<String>,

        /// Optional filter in JSON format
        #[arg(short = 'i', long)]
        filter: Option<String>,

        /// Output format (json or pretty)
        #[arg(short, long, default_value = "pretty")]
        output: String,
    },
    /// Execute a mutation operation
    Mutate {
        /// Schema name to mutate
        #[arg(short, long, required = true)]
        schema: String,

        /// Mutation type
        #[arg(short, long, required = true, value_enum)]
        mutation_type: MutationType,

        /// Data in JSON format
        #[arg(short, long, required = true)]
        fields_and_values: HashMap<String, Value>,

        /// Keys and values in JSON format
        #[arg(short, long, required = true)]
        keys_and_values: HashMap<String, String>,
    },
    /// Load an operation from a JSON file
    Execute {
        /// Path to the operation JSON file
        #[arg(required = true)]
        path: PathBuf,
    },
}

fn handle_load_schema(
    path: PathBuf,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading schema from: {}", path.display());
    let path_str = path.to_str().ok_or("Invalid file path")?;
    node.load_schema_from_file(path_str)?;
    info!("Schema loaded successfully");
    Ok(())
}

fn handle_add_schema(
    path: PathBuf,
    name: Option<String>,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Adding schema from: {}", path.display());

    // Read the schema file
    let schema_content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read schema file: {}", e))?;

    // Determine schema name from parameter or filename
    let custom_name = name.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });

    info!("Using database-level validation (always enabled)");

    // Use the database-level method which includes full validation
    let final_schema_name = node
        .add_schema_to_available_directory(&schema_content, custom_name)
        .map_err(|e| format!("Schema validation failed: {}", e))?;

    // Reload available schemas
    info!("Reloading available schemas...");
    node.refresh_schemas()
        .map_err(|e| format!("Failed to reload schemas: {}", e))?;

    info!(
        "Schema '{}' is now available for approval and use",
        final_schema_name
    );
    Ok(())
}

fn handle_hash_schemas(verify: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verify {
        info!("Verifying schema hashes in available_schemas directory...");

        match SchemaHasher::verify_available_schemas_directory() {
            Ok(results) => {
                let mut all_valid = true;
                info!("Hash verification results:");

                for (filename, is_valid) in results {
                    if is_valid {
                        info!("  ✅ {}: Valid hash", filename);
                    } else {
                        info!("  ❌ {}: Invalid or missing hash", filename);
                        all_valid = false;
                    }
                }

                if all_valid {
                    info!("All schemas have valid hashes!");
                } else {
                    info!("Some schemas have invalid or missing hashes. Run without --verify to update them.");
                }
            }
            Err(e) => {
                return Err(format!("Failed to verify schema hashes: {}", e).into());
            }
        }
    } else {
        info!("Adding/updating hashes for all schemas in available_schemas directory...");

        match SchemaHasher::hash_available_schemas_directory() {
            Ok(results) => {
                info!("Successfully processed {} schema files:", results.len());

                for (filename, hash) in results {
                    info!("  ✅ {}: {}", filename, hash);
                }

                info!("All schemas have been updated with hashes!");
            }
            Err(e) => {
                return Err(format!("Failed to hash schemas: {}", e).into());
            }
        }
    }

    Ok(())
}

fn handle_list_schemas(node: &mut DataFoldNode) -> Result<(), Box<dyn std::error::Error>> {
    let schemas = node.list_schemas()?;
    info!("Loaded schemas:");
    for schema in schemas {
        info!("  - {}", schema);
    }
    Ok(())
}

fn handle_list_available_schemas(
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let names = node.list_available_schemas()?;
    info!("Available schemas:");
    for name in names {
        info!("  - {}", name);
    }
    Ok(())
}

fn handle_unload_schema(
    name: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    node.unload_schema(&name)?;
    info!("Schema '{}' unloaded", name);
    Ok(())
}

fn handle_allow_schema(
    name: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    node.allow_schema(&name)?;
    info!("Schema '{}' allowed", name);
    Ok(())
}

fn handle_approve_schema(
    name: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    node.approve_schema(&name)?;
    info!("Schema '{}' approved successfully", name);
    Ok(())
}

fn handle_block_schema(
    name: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    node.block_schema(&name)?;
    info!("Schema '{}' blocked successfully", name);
    Ok(())
}

fn handle_get_schema_state(
    name: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = node.get_schema_state(&name)?;
    let state_str = match state {
        SchemaState::Available => "available",
        SchemaState::Approved => "approved",
        SchemaState::Blocked => "blocked",
    };
    info!("Schema '{}' state: {}", name, state_str);
    Ok(())
}

fn handle_list_schemas_by_state(
    state: String,
    node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema_state = match state.as_str() {
        "available" => SchemaState::Available,
        "approved" => SchemaState::Approved,
        "blocked" => SchemaState::Blocked,
        _ => {
            return Err(format!(
                "Invalid state: {}. Use: available, approved, or blocked",
                state
            )
            .into())
        }
    };

    let schemas = node.list_schemas_by_state(schema_state)?;
    info!("Schemas with state '{}':", state);
    for schema in schemas {
        info!("  - {}", schema);
    }
    Ok(())
}

fn handle_query(
    node: Arc<Mutex<DataFoldNode>>,
    schema: String,
    fields: Vec<String>,
    filter: Option<String>,
    output: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing query on schema: {}", schema);

    let filter_value = if let Some(filter_str) = filter {
        Some(serde_json::from_str(&filter_str)?)
    } else {
        None
    };

    let operation = Operation::Query {
        schema,
        fields,
        filter: filter_value,
    };

    let processor = OperationProcessor::new(node);
    let result = processor.execute_sync(operation)?;

    if output == "json" {
        info!("{}", result);
    } else {
        info!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

fn handle_mutate(
    node: Arc<Mutex<DataFoldNode>>,
    schema: String,
    mutation_type: MutationType,
    fields_and_values: HashMap<String, Value>,
    keys_and_values: HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing mutation on schema: {}", schema);

    let operation = Operation::Mutation {
        schema,
        fields_and_values,
        keys_and_values,
        mutation_type,
    };

    let processor = OperationProcessor::new(node);
    processor.execute_sync(operation)?;
    info!("Mutation executed successfully");

    Ok(())
}

fn handle_execute(
    path: PathBuf,
    node: Arc<Mutex<DataFoldNode>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing operation from file: {}", path.display());
    let operation_str = fs::read_to_string(path)?;
    let operation: Operation = serde_json::from_str(&operation_str)?;

    let processor = OperationProcessor::new(node);
    let result = processor.execute_sync(operation)?;

    if !result.is_null() {
        info!("Result:");
        info!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        info!("Operation executed successfully");
    }

    Ok(())
}

/// Main entry point for the DataFold CLI.
///
/// This function parses command-line arguments, initializes a DataFold node,
/// and executes the requested command. It supports various operations such as
/// loading schemas, listing schemas, executing queries and mutations, and more.
///
/// # Command-Line Arguments
///
/// * `-c, --config <PATH>` - Path to the node configuration file (default: config/node_config.json)
/// * Subcommands:
///   * `load-schema <PATH>` - Load a schema from a JSON file
///   * `list-schemas` - List all loaded schemas
///   * `list-available-schemas` - List schemas stored on disk
///   * `unload-schema --name <NAME>` - Unload a schema
///   * `query` - Execute a query operation
///   * `mutate` - Execute a mutation operation
///   * `execute <PATH>` - Load an operation from a JSON file
///
/// # Returns
///
/// A `Result` indicating success or failure.
///
/// # Errors
///
/// Returns an error if:
/// * The configuration file cannot be read or parsed
/// * The node cannot be initialized
/// * There is an error executing the requested command
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    datafold::web_logger::init().ok();
    let cli = Cli::parse();

    // Handle commands that don't need the node first
    if let Commands::HashSchemas { verify } = cli.command {
        return handle_hash_schemas(verify);
    }

    // Load node configuration
    info!("Loading config from: {}", cli.config);
    let config = load_node_config(Some(&cli.config), None)?;

    // Initialize node
    info!("Initializing DataFold Node...");
    let mut node = DataFoldNode::load(config).await?;
    info!("Node initialized with ID: {}", node.get_node_id());

    // Convert to Arc<Mutex<DataFoldNode>> for OperationProcessor
    let node_arc = Arc::new(Mutex::new(node));

    // Process command
    match cli.command {
        Commands::LoadSchema { path } => {
            let mut node_guard = node_arc.lock().await;
            handle_load_schema(path, &mut node_guard)?;
        }
        Commands::AddSchema { path, name } => {
            let mut node_guard = node_arc.lock().await;
            handle_add_schema(path, name, &mut node_guard)?;
        }
        Commands::HashSchemas { .. } => unreachable!(), // Already handled above
        Commands::ListSchemas {} => {
            let mut node_guard = node_arc.lock().await;
            handle_list_schemas(&mut node_guard)?;
        }
        Commands::ListAvailableSchemas {} => {
            let mut node_guard = node_arc.lock().await;
            handle_list_available_schemas(&mut node_guard)?;
        }
        Commands::AllowSchema { name } => {
            let mut node_guard = node_arc.lock().await;
            handle_allow_schema(name, &mut node_guard)?;
        }
        Commands::Query {
            schema,
            fields,
            filter,
            output,
        } => handle_query(node_arc.clone(), schema, fields, filter, output)?,
        Commands::Mutate {
            schema,
            mutation_type,
            fields_and_values,
            keys_and_values,
        } => handle_mutate(node_arc.clone(), schema, mutation_type, fields_and_values, keys_and_values)?,
        Commands::UnloadSchema { name } => {
            let mut node_guard = node_arc.lock().await;
            handle_unload_schema(name, &mut node_guard)?;
        }
        Commands::ApproveSchema { name } => {
            let mut node_guard = node_arc.lock().await;
            handle_approve_schema(name, &mut node_guard)?;
        }
        Commands::BlockSchema { name } => {
            let mut node_guard = node_arc.lock().await;
            handle_block_schema(name, &mut node_guard)?;
        }
        Commands::GetSchemaState { name } => {
            let mut node_guard = node_arc.lock().await;
            handle_get_schema_state(name, &mut node_guard)?;
        }
        Commands::ListSchemasByState { state } => {
            let mut node_guard = node_arc.lock().await;
            handle_list_schemas_by_state(state, &mut node_guard)?;
        }
        Commands::Execute { path } => handle_execute(path, node_arc.clone())?,
    }

    Ok(())
}
