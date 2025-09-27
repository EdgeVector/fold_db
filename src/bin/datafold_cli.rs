use clap::{Parser, Subcommand};
use datafold::schema::SchemaHasher;
use datafold::{load_node_config, DataFoldNode, MutationType, Operation};
use datafold::datafold_node::OperationProcessor;
use datafold::fold_db_core::query::format_hash_range_fields;
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
        fields_and_values: String,

        /// Keys and values in JSON format
        #[arg(short, long, required = true)]
        keys_and_values: String,
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
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading schema from: {}", path.display());
    // TODO: Schema loading functionality needs to be implemented
    info!("Schema loading functionality is not yet implemented");
    Ok(())
}

fn handle_add_schema(
    path: PathBuf,
    name: Option<String>,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Adding schema from: {}", path.display());
    
    // Read the schema file to validate it exists
    let _schema_content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read schema file: {}", e))?;

    // Determine schema name from parameter or filename
    let _custom_name = name.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });

    // TODO: Schema management functionality needs to be implemented
    info!("Schema management functionality is not yet implemented");
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

fn handle_list_schemas(_node: &mut DataFoldNode) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema listing functionality needs to be implemented
    info!("Schema listing functionality is not yet implemented");
    Ok(())
}

fn handle_list_available_schemas(
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Available schema listing functionality needs to be implemented
    info!("Available schema listing functionality is not yet implemented");
    Ok(())
}

fn handle_unload_schema(
    name: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema unloading functionality needs to be implemented
    info!("Schema unloading functionality is not yet implemented for: {}", name);
    Ok(())
}

fn handle_allow_schema(
    name: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema allowing functionality needs to be implemented
    info!("Schema allowing functionality is not yet implemented for: {}", name);
    Ok(())
}

fn handle_approve_schema(
    name: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema approval functionality needs to be implemented
    info!("Schema approval functionality is not yet implemented for: {}", name);
    Ok(())
}

fn handle_block_schema(
    name: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema blocking functionality needs to be implemented
    info!("Schema blocking functionality is not yet implemented for: {}", name);
    Ok(())
}

fn handle_get_schema_state(
    name: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Schema state functionality needs to be implemented
    info!("Schema state functionality is not yet implemented for: {}", name);
    Ok(())
}

fn handle_list_schemas_by_state(
    state: String,
    _node: &mut DataFoldNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate the state parameter
    match state.as_str() {
        "available" | "approved" | "blocked" => {},
        _ => {
            return Err(format!(
                "Invalid state: {}. Use: available, approved, or blocked",
                state
            )
            .into())
        }
    }

    // TODO: Schema listing by state functionality needs to be implemented
    info!("Schema listing by state functionality is not yet implemented for state: {}", state);
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

    let processor = OperationProcessor::new(node);
    let (schema, fields, filter) = (schema, fields, filter_value);
    let rt = tokio::runtime::Handle::current();
    let result_map = rt.block_on(async move {
        processor.execute_query_map(schema, fields, filter).await
    })?;

    let formatted = format_hash_range_fields(&result_map);

    if output == "json" {
        info!("{}", serde_json::to_string(&formatted)?);
    } else {
        info!("{}", serde_json::to_string_pretty(&formatted)?);
    }

    Ok(())
}

fn handle_mutate(
    node: Arc<Mutex<DataFoldNode>>,
    schema: String,
    mutation_type: MutationType,
    fields_and_values: String,
    keys_and_values: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing mutation on schema: {}", schema);

    // Parse the JSON strings
    let fields_and_values_map: HashMap<String, Value> = serde_json::from_str(&fields_and_values)?;
    let keys_and_values_map: HashMap<String, String> = serde_json::from_str(&keys_and_values)?;

    // Create KeyValue from the keys_and_values
    let key_value = datafold::schema::types::key_value::KeyValue::new(
        keys_and_values_map.get("hash").cloned(),
        keys_and_values_map.get("range").cloned(),
    );

    let processor = OperationProcessor::new(node);
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async move {
        processor.execute_mutation(schema, fields_and_values_map, key_value, mutation_type).await
    })?;
    info!("Mutation executed successfully");

    Ok(())
}

fn handle_execute(
    path: PathBuf,
    node: Arc<Mutex<DataFoldNode>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing operation from file: {}", path.display());
    let operation_str = fs::read_to_string(path)?;
    let processor = OperationProcessor::new(node);
    // Determine operation type and dispatch explicitly
    let parsed: Operation = serde_json::from_str(&operation_str)?;
    let rt = tokio::runtime::Handle::current();
    let result = match parsed {
        Operation::Query { schema, fields, filter } => {
            let map = rt.block_on(async move {
                processor.execute_query_map(schema, fields, filter).await
            })?;
            format_hash_range_fields(&map)
        }
        Operation::Mutation { schema, fields_and_values, key_value, mutation_type } => {
            rt.block_on(async move {
                processor.execute_mutation(schema, fields_and_values, key_value, mutation_type).await
            })?
        }
    };

    if !result.is_null() {
        info!("Result:");
        info!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        info!("Operation executed successfully");
    }

    Ok(())
}

// formatting is handled by fold_db_core::query::formatter

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
    let node = DataFoldNode::load(config).await?;
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
