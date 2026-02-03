# DataFold

[![Crates.io](https://img.shields.io/crates/v/datafold.svg)](https://crates.io/crates/datafold)
[![Documentation](https://docs.rs/datafold/badge.svg)](https://docs.rs/datafold)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/yourusername/datafold)

A Rust-based distributed data platform with schema-based storage, AI-powered ingestion, and real-time data processing capabilities. DataFold provides a complete solution for distributed data management with automatic schema generation, field mapping, and extensible ingestion pipelines.

## ✨ Features

- **🤖 AI-Powered Data Ingestion** - Automatic schema creation and field mapping using AI [Initial prototype]
- **💬 AI Natural Language Query** - Ask questions in plain English, get AI-interpreted results [working]
- **🔄 Real-Time Processing** - Event-driven architecture with automatic transform execution [working]
- **🌐 Distributed Architecture** - P2P networking with automatic peer discovery [untested]
- **📊 Flexible Schema System** - Dynamic schema management with validation [working]
- **🔐 Permission Management** - Fine-grained access control and trust-based permissions [working]
- **⚡ High Performance** - Rust-based core with optimized storage and query execution [maybe]
- **☁️ Serverless Ready** - S3-backed storage for AWS Lambda and serverless deployments [working]
- **🔌 Extensible Ingestion** - Plugin system for social media and external data sources [not yet begun]

## 🚀 Quick Start

### Installation

#### Option 1: Download Pre-built Binary (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/shiba4life/fold_db/releases):

```bash
# macOS (Intel)
curl -LO https://github.com/shiba4life/fold_db/releases/latest/download/datafold_http_server-macos-x86_64-VERSION
chmod +x datafold_http_server-macos-x86_64-VERSION
sudo mv datafold_http_server-macos-x86_64-VERSION /usr/local/bin/datafold_http_server

# macOS (Apple Silicon)
curl -LO https://github.com/shiba4life/fold_db/releases/latest/download/datafold_http_server-macos-aarch64-VERSION
chmod +x datafold_http_server-macos-aarch64-VERSION
sudo mv datafold_http_server-macos-aarch64-VERSION /usr/local/bin/datafold_http_server

# Linux
curl -LO https://github.com/shiba4life/fold_db/releases/latest/download/datafold_http_server-linux-x86_64-VERSION
chmod +x datafold_http_server-linux-x86_64-VERSION
sudo mv datafold_http_server-linux-x86_64-VERSION /usr/local/bin/datafold_http_server
```

Replace `VERSION` with the actual version number (e.g., `0.1.5`).

#### Option 2: Install from Crates.io

Add DataFold to your `Cargo.toml`:

```toml
[dependencies]
datafold = "0.1.0"
```

Or install the CLI tools:

```bash
cargo install datafold
```

This provides three binaries:

- `datafold_cli` - Command-line interface
- `datafold_http_server` - HTTP server with web UI
- `datafold_node` - P2P node server

### Optional TypeScript Bindings

The crate ships without generating TypeScript artifacts by default so it can
compile cleanly in any environment. If you need the auto-generated bindings for
the web UI, enable the `ts-bindings` feature when building or testing:

```bash
cargo build --features ts-bindings
```

The feature keeps the `ts-rs` dependency optional and writes the generated
definitions to the existing `bindings/` directory just like the repository
version.

### Basic Usage

```rust
use fold_db::{DataFoldNode, IngestionCore, Schema};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a DataFold node
    let node = DataFoldNode::new_with_defaults().await?;

    // Create an ingestion pipeline
    let config = fold_db::IngestionConfig::from_env_allow_empty();
    let ingestion = IngestionCore::new(config)?;

    // Process JSON data with automatic schema generation
    let data = json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30,
        "preferences": {
            "theme": "dark",
            "notifications": true
        }
    });

    let response = ingestion.process_json_ingestion(
        fold_db::IngestionRequest { data }
    ).await?;

    println!("Ingestion result: {:?}", response);
    Ok(())
}
```

### Running the HTTP Server

```bash
# Start the HTTP server with web UI
datafold_http_server --port 9001
```

Then visit `http://localhost:9001` for the web interface.

## 🌐 Global Schema Service

DataFold provides a **Global Schema Service** at [schema.folddb.com](https://schema.folddb.com) for sharing and discovering schemas across the network.

### How It Works

The global schema service is managed **automatically** through two processes:

1. **AI-Powered Ingestion** - When ingesting data, the LLM analyzes your data structure and automatically checks the global schema registry for matching schemas. If a compatible schema exists, it's used; otherwise, a new schema may be registered.

2. **AI Query Processing** - When executing natural language queries, the system consults the global schema service to understand available data structures and optimize query execution across nodes.

### Features

- **Automatic Schema Discovery** - Schemas are matched and reused automatically during ingestion
- **Schema Registry** - Published schemas are available for all DataFold nodes to discover
- **Interoperability** - Shared schemas enable seamless data exchange between nodes
- **Version Tracking** - Schema versions are tracked over time

### Configuration

Set the schema service URL via environment variable:

```bash
export DATAFOLD_SCHEMA_SERVICE_URL=https://schema.folddb.com
```

## 📖 Core Concepts

### Schemas

DataFold uses dynamic schemas that define data structure and operations:

```rust
use fold_db::{Schema, Operation};

// Load a schema
let schema_json = std::fs::read_to_string("my_schema.json")?;
let schema: Schema = serde_json::from_str(&schema_json)?;

// Execute operations
let operation = Operation::Query(query_data);
let result = node.execute_operation(operation).await?;
```

### AI-Powered Ingestion

Automatically analyze and ingest data from any source:

```rust
use fold_db::{IngestionConfig, IngestionCore};

// Configure with OpenRouter API
let config = IngestionConfig {
    openrouter_api_key: Some("your-api-key".to_string()),
    openrouter_model: "anthropic/claude-3.5-sonnet".to_string(),
    ..Default::default()
};

let ingestion = IngestionCore::new(config)?;

// Process any JSON data
let result = ingestion.process_json_ingestion(request).await?;
```

### Distributed Networking

Connect nodes in a P2P network:

```rust
use fold_db::{NetworkConfig, NetworkCore};

let network_config = NetworkConfig::default();
let network = NetworkCore::new(network_config).await?;

// Start networking
network.start().await?;

// Discover peers
let peers = network.discover_peers().await?;
```

## 🌐 Frontend Development

DataFold includes a comprehensive React frontend with a unified API client architecture that provides type-safe, standardized access to all backend operations.

### Frontend API Clients

The frontend uses specialized API clients that eliminate boilerplate code and provide consistent error handling, caching, and authentication:

```typescript
import { schemaClient, securityClient, systemClient } from "../api/clients";

// Schema operations with automatic caching
const response = await schemaClient.getSchemas();
if (response.success) {
  const schemas = response.data; // Fully typed SchemaData[]
}

// System monitoring with intelligent caching
const status = await systemClient.getSystemStatus(); // 30-second cache

// Security operations with built-in validation
const verification = await securityClient.verifyMessage(signedMessage);
```

### Key Features

- **🔒 Type Safety** - Full TypeScript support with comprehensive interfaces
- **⚡ Intelligent Caching** - Operation-specific caching (30s for status, 5m for schemas, 1h for keys)
- **🔄 Automatic Retries** - Configurable retry logic with exponential backoff
- **🛡️ Error Handling** - Standardized error types with user-friendly messages
- **🔐 Built-in Authentication** - Automatic auth header management
- **📊 Request Deduplication** - Prevents duplicate concurrent requests
- **🎯 Batch Operations** - Efficient multi-request processing

### Available Clients

- **SchemaClient** - Schema management and SCHEMA-002 compliance
- **SecurityClient** - Authentication, key management, cryptographic operations
- **SystemClient** - System operations, logging, database management
- **TransformClient** - Data transformation and queue management
- **IngestionClient** - AI-powered data ingestion (60s timeout for AI processing)
- **MutationClient** - Data mutation operations and query execution

### Error Handling

```typescript
import {
  isNetworkError,
  isAuthenticationError,
  isSchemaStateError,
} from "../api/core/errors";

try {
  const response = await schemaClient.approveSchema("users");
} catch (error) {
  if (isAuthenticationError(error)) {
    redirectToLogin();
  } else if (isSchemaStateError(error)) {
    showMessage(`Schema "${error.schemaName}" is ${error.currentState}`);
  } else {
    showMessage(error.toUserMessage());
  }
}
```

### Frontend Development Setup

```bash
# Start the backend server
cargo run --bin datafold_http_server -- --port 9001

# In another terminal, start the React frontend
cd src/datafold_node/static-react
npm install
npm run dev
```

The frontend will be available at `http://localhost:5173` with hot-reload.

### Frontend Documentation

- **[Architecture Guide](docs/delivery/API-STD-1/api-client-architecture.md)** - Technical architecture and design patterns
- **[Developer Guide](docs/delivery/API-STD-1/developer-guide.md)** - Usage examples and best practices
- **[Migration Reference](docs/delivery/API-STD-1/migration-reference.md)** - Migration from direct fetch() usage

## 🔌 Extensible Ingestion

DataFold supports ingesting data from various sources with the new adapter-based architecture:

- **Social Media APIs** - Twitter, Facebook, Reddit, TikTok
- **Real-time Streams** - WebSockets, Server-Sent Events
- **File Uploads** - JSON, CSV, JSONL with AI-powered conversion
- **S3 File Paths** - Process files already in S3 without re-uploading
- **Webhooks** - Real-time event processing
- **Custom Adapters** - Extensible plugin system

See [`SOCIAL_MEDIA_INGESTION_PROPOSAL.md`](SOCIAL_MEDIA_INGESTION_PROPOSAL.md) for the complete ingestion architecture.

### File Ingestion

DataFold provides two ways to ingest files:

**1. Traditional File Upload**

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "file=@/path/to/local/file.json" \
  -F "autoExecute=true"
```

**2. S3 File Path (No Re-upload Required)**

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/path/to/file.json" \
  -F "autoExecute=true"
```

**3. Programmatic API (for Lambda/Rust code)**

```rust
use fold_db::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

// Async ingestion (returns immediately with progress_id)
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());
let response = ingest_from_s3_path_async(&request, &state).await?;
println!("Started: {}", response.progress_id.unwrap());

// Or sync ingestion (waits for completion)
use fold_db::ingestion::ingest_from_s3_path_sync;
let response = ingest_from_s3_path_sync(&request, &state).await?;
println!("Complete: {} mutations", response.mutations_executed);
```

The S3 file path option allows you to process files already stored in S3 without uploading them again, saving bandwidth and time. This is particularly useful for:

- **Lambda Functions** - Process S3 events programmatically
- **ETL Pipelines** - Ingest pipeline outputs already in S3
- **Batch Processing** - Process existing S3 files at scale
- **Data Lakes** - Integration with S3-based data lakes

**Requirements for S3 file paths:**

- S3 storage mode must be configured (`DATAFOLD_UPLOAD_STORAGE_MODE=s3`)
- AWS credentials with `s3:GetObject` permissions

See [S3 File Path Ingestion Guide](docs/S3_FILE_PATH_INGESTION.md) for complete documentation and [Lambda example](examples/lambda_s3_ingestion.rs) for AWS Lambda integration.

## 🛠️ Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 16+ (for web UI development)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/datafold.git
cd datafold

# Install dependencies
sudo apt install rustup
rustup default stable        # Installs cargo
sudo apt install openssl libssl-dev pkg-config

# Build all components
cargo build --release --workspace

# Run tests
cargo test --workspace
```

### Running the Web UI

For development with hot-reload:

```bash
# Start the Rust backend
cargo run --bin datafold_http_server -- --port 9001

# In another terminal, start the React frontend
cd src/datafold_node/static-react
npm install
npm run dev
```

The UI will be available at `http://localhost:5173`.

## ☁️ Serverless Deployment (S3 Storage)

DataFold can run in serverless environments like AWS Lambda using S3-backed storage:

```rust
use fold_db::{FoldDB, S3Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure S3 storage
    let config = S3Config::new(
        "my-folddb-bucket".to_string(),
        "us-west-2".to_string(),
        "production".to_string(),
    );

    // Database automatically downloads from S3 on startup
    let db = FoldDB::new_with_s3(config).await?;

    // Use normally - all operations are local
    // ... queries, mutations, transforms ...

    // Sync back to S3
    db.flush_to_s3().await?;

    Ok(())
}
```

**Environment variable configuration:**

```bash
# Database storage (Sled with S3 sync)
export DATAFOLD_STORAGE_MODE=s3
export DATAFOLD_S3_BUCKET=my-folddb-bucket
export DATAFOLD_S3_REGION=us-west-2

# Upload storage (for file ingestion)
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-uploads-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

See [S3 Configuration Guide](docs/S3_CONFIGURATION.md) for complete setup instructions, AWS Lambda deployment, and cost optimization.

## ⚡ AWS Lambda & DynamoDB

DataFold provides first-class support for AWS Lambda with a multi-tenant DynamoDB backend. This allows you to build serverless, user-isolated applications without managing servers.

### Setup

Add the `lambda` feature to your `Cargo.toml`:

```toml
[dependencies]
datafold = { version = "0.1.0", features = ["lambda"] }
```

### Configuration

Initialize the `LambdaContext` with `LambdaStorage::DynamoDb`:

```rust
use fold_db::lambda::{LambdaConfig, LambdaContext, LambdaStorage, LambdaLogging};
use fold_db::storage::{DynamoDbConfig, ExplicitTables};

// Using ExplicitTables::from_prefix for convenience
let config = LambdaConfig::new(
    LambdaStorage::DynamoDb(DynamoDbConfig {
        region: "us-east-1".to_string(),
        tables: ExplicitTables::from_prefix("MyApp"), // Creates: MyApp-main, MyApp-schemas, etc.
        auto_create: true,
        user_id: None,
    }),
    LambdaLogging::Stdout,
);

LambdaContext::init(config).await?;
```

### DynamoDB Tables

The system requires and automatically manages **11 tables** per deployment. Using `ExplicitTables::from_prefix("MyApp")`, they are:

- `MyApp-main` (Data)
- `MyApp-metadata`
- `MyApp-node_id_schema_permissions`
- `MyApp-transforms`
- `MyApp-orchestrator_state`
- `MyApp-schema_states`
- `MyApp-schemas`
- `MyApp-public_keys`
- `MyApp-transform_queue_tree`
- `MyApp-native_index`
- `MyApp-process` (Process Tracking)

### Multi-Tenancy

DataFold automatically handles multi-tenancy. When you pass a `user_id` to ingestion or node retrieval methods, operations are scoped to that user within the DynamoDB tables.

## 📊 Examples

### Loading Sample Data

```bash
# Use the CLI to load a schema
datafold_cli load-schema examples/user_schema.json

# Query data
datafold_cli query examples/user_query.json

# Execute mutations
datafold_cli mutate examples/user_mutation.json
```

### Rust Code Examples

See [`examples/`](examples/) directory for:

- **[simple_s3_ingestion.rs](examples/simple_s3_ingestion.rs)** - Basic S3 file ingestion from Rust code
- **[lambda_s3_ingestion.rs](examples/lambda_s3_ingestion.rs)** - AWS Lambda integration with S3 events

```rust
// Quick example: Ingest S3 file in Lambda
use fold_db::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/data.json".to_string());
let response = ingest_from_s3_path_async(&request, &state).await?;
```

### Python Integration

See [`datafold_api_examples/`](datafold_api_examples/) for Python scripts demonstrating:

- Schema management
- Data querying
- Mutations and updates
- User management

## 🔧 Configuration

DataFold uses JSON configuration files. Default config:

```json
{
  "storage_path": "data/db",
  "default_trust_distance": 1,
  "network": {
    "port": 9000,
    "enable_mdns": true
  },
  "ingestion": {
    "enabled": true,
    "openrouter_model": "anthropic/claude-3.5-sonnet"
  }
}
```

Environment variables:

- `OPENROUTER_API_KEY` - API key for AI-powered ingestion
- `DATAFOLD_CONFIG` - Path to configuration file

## 🔐 Public Key Persistence

DataFold stores registered Ed25519 public keys in the sled database. When the node
starts it loads all saved keys, and new keys are persisted as soon as they are
registered. This keeps authentication intact across restarts. See
[PBI SEC-8 documentation](docs/delivery/SEC-8/prd.md) for implementation details.

- `DATAFOLD_LOG_LEVEL` - Logging level (trace, debug, info, warn, error)

## 📚 Documentation

- **[API Documentation](https://docs.rs/datafold)** - Complete API reference
- **[CLI Guide](README_CLI.md)** - Command-line interface usage
- **[Ingestion Guide](INGESTION_README.md)** - AI-powered data ingestion
- **[S3 File Path Ingestion](docs/S3_FILE_PATH_INGESTION.md)** - Process S3 files without re-uploading
- **[AI Query Guide](docs/AI_QUERY_USAGE_GUIDE.md)** - Natural language query with AI interpretation
- **[AI Query Quick Reference](docs/AI_QUERY_QUICK_REFERENCE.md)** - Quick start for AI queries
- **[S3 Storage Guide](docs/S3_CONFIGURATION.md)** - Serverless deployment with S3
- **[Upload Storage Guide](docs/UPLOAD_STORAGE.md)** - Configure local or S3 storage for uploads
- **[Architecture](docs/Unified_Architecture.md)** - System design and patterns

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo test --workspace`
5. Submit a pull request

## 📄 License

This project is licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

## 🌟 Community

- **Issues** - Report bugs and request features on [GitHub Issues](https://github.com/yourusername/datafold/issues)
- **Discussions** - Join discussions on [GitHub Discussions](https://github.com/yourusername/datafold/discussions)

---

**DataFold** - Distributed data platform for the modern world 🚀
