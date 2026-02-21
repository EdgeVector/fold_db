# FoldDB

[![Crates.io](https://img.shields.io/crates/v/fold_db.svg)](https://crates.io/crates/fold_db)
[![Documentation](https://docs.rs/fold_db/badge.svg)](https://docs.rs/fold_db)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/shiba4life/fold_db)

FoldDB is a personal database that uses AI to automatically organize your data. Drop in files, JSON, or social media exports — FoldDB detects schemas, extracts searchable keywords, and lets you query with natural language. Runs locally with minimal config, or scales to AWS with DynamoDB and Lambda.

## When to Use FoldDB

- **Organize personal data** — Tweets, notes, photos, documents. Drop them in and FoldDB schemas, indexes, and stores them automatically.
- **Build apps with AI-powered ingestion** — Skip writing schema definitions and field mappings. Send JSON, get structured storage.
- **Self-hosted alternative to cloud databases** — Everything runs on your machine with AES-256-GCM encryption at rest. Your data stays yours.
- **Serverless data backends** — Deploy to AWS Lambda with DynamoDB multi-tenant isolation out of the box.

## Quick Start

### 1. Install

```bash
curl -fsSL https://raw.githubusercontent.com/shiba4life/fold_db/master/install.sh | sh
```

Auto-detects macOS (Apple Silicon / Intel) and Linux x86_64. Or install from source:

```bash
cargo install --git https://github.com/shiba4life/fold_db.git --bin folddb
```

### 2. Set your API key (for AI features)

```bash
export FOLD_OPENROUTER_API_KEY="sk-..."
```

### 3. Run

```bash
./run.sh --local
```

Visit `http://localhost:5173` for the web UI. The backend runs on port 9001.

## Features

- **AI-Powered Ingestion** — Drop any JSON and AI generates schemas, maps fields, and stores your data
- **Smart Folder Ingestion** — Point at a directory and let AI filter, classify, and batch-ingest files
- **Natural Language Queries** — Ask questions in plain English, get structured results
- **Encryption at Rest** — AES-256-GCM encryption with local keys or AWS KMS
- **Fine-Grained Permissions** — Trust-based access control at the field level
- **Dynamic Schemas** — Schemas evolve with your data, no migrations needed
- **Serverless Ready** — DynamoDB + S3 storage backend, deploy to AWS Lambda with zero modifications
- **Distributed P2P** — Built-in peer discovery and networking for horizontal scaling

## How It Works

```
Files / JSON / APIs
        |
        v
   AI Ingestion ──> Schema Service (detects or creates schema)
        |
        v
   Mutation ──> Storage (Sled local or DynamoDB cloud)
        |
        v
   Keyword Indexing ──> AI extracts and normalizes searchable terms
        |
        v
   Query ──> Natural language or structured field queries
```

1. **Ingest** — Send data in any format. AI analyzes the structure and maps it to a schema.
2. **Schema** — The global schema service checks for existing compatible schemas or creates new ones.
3. **Store** — Data is written as mutations with encryption at rest.
4. **Index** — AI extracts keywords and normalizes terms (dates, names, etc.) for search.
5. **Query** — Search with natural language or structured field queries.

## Core Concepts

### Schemas

FoldDB uses dynamic schemas that define data structure and operations. Schemas are managed automatically during AI ingestion or can be loaded manually:

```bash
folddb schema load my_schema.json
folddb schema approve my_schema
folddb schema list -p
```

### Global Schema Service

A shared registry at [schema.folddb.com](https://schema.folddb.com) ensures schema consistency across FoldDB instances. During ingestion, the AI checks the registry for compatible schemas before creating new ones.

```bash
export FOLD_SCHEMA_SERVICE_URL=https://schema.folddb.com
```

### Ingestion

```rust
use fold_db::{IngestionConfig, IngestionCore, IngestionRequest};
use serde_json::json;

let config = IngestionConfig::from_env_allow_empty();
let ingestion = IngestionCore::new(config)?;

let data = json!({
    "name": "John Doe",
    "email": "john@example.com",
    "age": 30
});

let result = ingestion.process_json_ingestion(
    IngestionRequest { data }
).await?;
```

### Queries

```rust
use fold_db::FoldNode;

let node = FoldNode::new_with_defaults().await?;

// Natural language query
let response = node.ai_query("Show me all purchases over $50").await?;
```

## CLI Reference

```bash
# Status and exploration
folddb status -p                              # Check node health
folddb schema list -p                         # List all schemas

# Ingest data
folddb ingest run data.json                   # Ingest a JSON file
folddb ingest smart-folder ~/Documents --scan # Scan a directory
folddb ingest smart-folder ~/Documents --all-recommended  # Batch ingest

# Query
folddb query run tweets --fields text,author  # Structured query
folddb query search "machine learning"        # Full-text search
folddb query ai "recent purchases over $50"   # Natural language

# Schema management
folddb schema load schema.json                # Load a schema
folddb schema approve my_schema               # Approve a pending schema
folddb schema get my_schema -p                # Inspect a schema
```

Run `folddb --help` for the full command reference.

## Web UI

Start with `./run.sh --local` and visit `http://localhost:5173`. The UI provides:

- Schema browsing and approval
- Data ingestion (file upload, smart folders, social media imports)
- Natural language and structured queries
- Native index search
- System status and configuration

## Frontend Development

FoldDB includes a React frontend with type-safe API clients:

```typescript
import { schemaClient, securityClient, systemClient } from "../api/clients";

// Schema operations with automatic caching
const response = await schemaClient.getSchemas();
if (response.success) {
  const schemas = response.data; // Fully typed SchemaData[]
}

// System monitoring with intelligent caching
const status = await systemClient.getSystemStatus(); // 30-second cache
```

### Available Clients

- **SchemaClient** — Schema management and SCHEMA-002 compliance
- **SecurityClient** — Authentication, key management, cryptographic operations
- **SystemClient** — System operations, logging, database management
- **TransformClient** — Data transformation and queue management
- **IngestionClient** — AI-powered data ingestion (60s timeout for AI processing)
- **MutationClient** — Data mutation operations and query execution

### Frontend Setup

```bash
# Start both backend and frontend
./run.sh --local

# Frontend-only development
cd src/server/static-react
npm install
npm run dev
```

## Advanced: AWS Deployment

### S3 Storage

FoldDB can use S3-backed storage for serverless environments:

```bash
export FOLD_STORAGE_MODE=s3
export FOLD_S3_BUCKET=my-folddb-bucket
export FOLD_S3_REGION=us-west-2
```

### Lambda + DynamoDB

Add the `lambda` feature for multi-tenant serverless deployments:

```toml
[dependencies]
fold_db = { version = "0.1.0", features = ["lambda"] }
```

```rust
use fold_db::lambda::{LambdaConfig, LambdaContext, LambdaStorage, LambdaLogging};
use fold_db::storage::{DynamoDbConfig, ExplicitTables};

let config = LambdaConfig::new(
    LambdaStorage::DynamoDb(DynamoDbConfig {
        region: "us-east-1".to_string(),
        tables: ExplicitTables::from_prefix("MyApp"),
        auto_create: true,
        user_id: None,
    }),
    LambdaLogging::Stdout,
);

LambdaContext::init(config).await?;
```

This creates 11 DynamoDB tables (`MyApp-main`, `MyApp-schemas`, etc.) with automatic multi-tenant isolation via `user_id`.

### File Ingestion from S3

Process files already in S3 without re-uploading:

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/path/to/file.json" \
  -F "autoExecute=true"
```

See [S3 Configuration Guide](docs/S3_CONFIGURATION.md) for complete setup.

## Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 16+ (for web UI)

### Building from Source

```bash
git clone https://github.com/shiba4life/fold_db.git
cd fold_db
cargo build --release --workspace
cargo test --workspace
```

### Running Locally

```bash
./run.sh --local                    # Local Sled + prod schema service (recommended)
./run.sh --local --local-schema     # Fully offline
./run.sh --local --empty-db         # Start with fresh database
./run.sh --local --dev              # Local Sled + dev schema service
```

### TypeScript Bindings

```bash
cargo build --features ts-bindings
```

## Configuration

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `FOLD_OPENROUTER_API_KEY` | API key for AI-powered ingestion (or `OPENROUTER_API_KEY`) |
| `FOLD_SCHEMA_SERVICE_URL` | Schema service URL (default: `https://schema.folddb.com`) |
| `FOLD_CONFIG` | Path to configuration file |
| `FOLD_LOG_LEVEL` | Logging level (`trace`, `debug`, `info`, `warn`, `error`) |
| `FOLD_STORAGE_MODE` | Storage backend (`s3` for cloud) |
| `FOLD_S3_BUCKET` | S3 bucket for database storage |
| `FOLD_S3_REGION` | AWS region for S3 |
| `FOLD_UPLOAD_STORAGE_MODE` | Upload storage backend (`s3` for cloud) |

## Documentation

- **[API Reference](https://docs.rs/fold_db)** — Complete Rust API docs
- **[Ingestion Guide](INGESTION_README.md)** — AI-powered data ingestion
- **[AI Query Guide](docs/AI_QUERY_USAGE_GUIDE.md)** — Natural language queries
- **[S3 Storage Guide](docs/S3_CONFIGURATION.md)** — Serverless deployment with S3
- **[Architecture](docs/Unified_Architecture.md)** — System design and patterns

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo test --workspace`
5. Submit a pull request

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.

## Community

- **[GitHub Issues](https://github.com/shiba4life/fold_db/issues)** — Report bugs and request features
- **[GitHub Discussions](https://github.com/shiba4life/fold_db/discussions)** — Questions and community discussion
