# FoldDB

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/shiba4life/fold_db)

**Your personal database with AI that organizes everything for you.**

Drop in files, JSON, or social media exports — FoldDB detects schemas, extracts searchable keywords, and lets you query with natural language. Runs locally on your machine. Your data stays yours.

<!-- TODO: Add demo GIF showing: drag-and-drop file ingestion → automatic schema detection → natural language query → results -->

## Quick Start

### Option A: Desktop App (macOS)

Download the latest `.dmg` from [GitHub Releases](https://github.com/shiba4life/fold_db/releases), open it, and drag **FoldDB.app** to Applications. Double-click to launch — no terminal needed.

> **First launch:** macOS will block the unsigned app. Right-click the app → **Open** → click **Open** in the dialog, or go to **System Settings → Privacy & Security → Open Anyway**.

### Option B: Install via Homebrew

```bash
brew tap shiba4life/fold_db
brew install folddb
```

### Option C: Install from Source

```bash
curl -fsSL https://raw.githubusercontent.com/shiba4life/fold_db/master/install.sh | sh
```

### Run

```bash
./run.sh --local
```

### Open your browser

Visit [http://localhost:5173](http://localhost:5173) — that's it. No API key required if you use [Ollama](https://ollama.com) (free, runs locally).

## What You Can Do

- **Import your Twitter archive** — Drop in your Twitter data export, then ask "what were my most liked tweets?"
- **Organize a folder of documents** — Point FoldDB at `~/Documents` and let AI classify, schema, and ingest everything automatically
- **Search across all your data** — Ask "what taxes did I pay last year?" and get answers pulled from ingested PDFs, JSON, and notes
- **Upload a CSV or PDF** — Get automatic schema detection, keyword extraction, and full-text search with zero configuration
- **Explore connections** — Visualize how your data relates using the built-in word graph

## Features

- **AI-Powered Ingestion** — Drop any file and AI generates schemas, maps fields, and stores your data
- **Smart Folder Ingestion** — Point at a directory and let AI filter, classify, and batch-ingest files
- **Natural Language Queries** — Ask questions in plain English, get structured results
- **Encryption at Rest** — AES-256-GCM encryption with local key management
- **Dynamic Schemas** — Schemas evolve with your data, no migrations needed
- **Keyword Indexing** — AI extracts and normalizes searchable terms (dates, names, topics)
- **Web UI** — Browse schemas, ingest data, query, and manage everything from your browser
- **CLI** — Full command-line interface for scripting and power users
- **Runs 100% Local** — No cloud account required. Sled embedded storage, Ollama for AI

## How It Works

```
  Your Files (JSON, CSV, PDF, images, exports)
                    |
                    v
          +------------------+
          |   AI Ingestion   |  ← Ollama (local) or OpenRouter (cloud)
          +------------------+
                    |
        +-----------+-----------+
        |                       |
        v                       v
  Schema Service          Keyword Indexing
  (detect/create)         (extract & normalize)
        |                       |
        +-----------+-----------+
                    |
                    v
          +------------------+
          |   Sled Storage   |  ← Encrypted at rest
          +------------------+
                    |
                    v
     Query with natural language or fields
```

## Supported Formats

| Format | What Happens |
|--------|-------------|
| **JSON** | Schema auto-detected, fields mapped, data stored |
| **CSV** | Rows parsed, columns become schema fields |
| **PDF** | Text extracted, AI identifies structure and keywords |
| **Images** | EXIF metadata extracted (dates, locations, camera info) |
| **Twitter JS exports** | Tweets, likes, and metadata parsed into searchable records |
| **Plain text** | Content indexed with AI-generated keywords |

## AI Providers

FoldDB works with two AI backends. Pick one (or both):

| Provider | Cost | Runs Where | Setup |
|----------|------|-----------|-------|
| **[Ollama](https://ollama.com)** | Free | Your machine | Install Ollama, pull a model, done |
| **[OpenRouter](https://openrouter.ai)** | Pay-per-use | Cloud API | `export FOLD_OPENROUTER_API_KEY="sk-..."` |

Ollama is the default — no API key needed. FoldDB will use it automatically if it's running.

## Web UI

Start with `./run.sh --local` and visit `http://localhost:5173`. The UI provides:

- Schema browsing and approval
- File upload, smart folder ingestion, and social media imports
- Natural language and structured queries
- Native index search
- Word graph visualization
- System status and configuration

<!-- TODO: Add screenshot of main UI -->

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

### Install from Source (alternative)

```bash
cargo install --git https://github.com/shiba4life/fold_db.git --bin folddb
```

## Configuration

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `FOLD_OPENROUTER_API_KEY` | API key for OpenRouter AI (or `OPENROUTER_API_KEY`). Not needed with Ollama. |
| `FOLD_SCHEMA_SERVICE_URL` | Schema service URL (default: `https://schema.folddb.com`) |
| `FOLD_CONFIG` | Path to configuration file |
| `FOLD_LOG_LEVEL` | Logging level (`trace`, `debug`, `info`, `warn`, `error`) |

<details>
<summary><h2>Advanced: AWS Deployment</h2></summary>

FoldDB can scale to AWS with DynamoDB + S3 storage and Lambda for serverless multi-tenant deployments.

### S3 Storage

```bash
export FOLD_STORAGE_MODE=s3
export FOLD_S3_BUCKET=my-folddb-bucket
export FOLD_S3_REGION=us-west-2
```

### Lambda + DynamoDB

Enable with the `lambda` feature flag:

```toml
[dependencies]
fold_db = { version = "0.1.0", features = ["lambda"] }
```

This creates 11 DynamoDB tables with automatic multi-tenant isolation via `user_id`.

### AWS Environment Variables

| Variable | Purpose |
|----------|---------|
| `FOLD_STORAGE_MODE` | Set to `s3` for cloud storage |
| `FOLD_S3_BUCKET` | S3 bucket for database storage |
| `FOLD_S3_REGION` | AWS region for S3 |
| `FOLD_UPLOAD_STORAGE_MODE` | Upload storage backend (`s3` for cloud) |

See [Lambda Multitenancy](docs/LAMBDA_MULTITENANCY.md) for architecture details.

</details>

## Documentation

- **[Vision & Goals](docs/GOAL.md)** — What FoldDB is building toward
- **[Launch Plan](docs/LAUNCH_PLAN.md)** — Roadmap and milestones
- **[Strategy](docs/STRATEGY.md)** — Product strategy

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, frontend development setup, and API client documentation.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.

## Community

- **[GitHub Issues](https://github.com/shiba4life/fold_db/issues)** — Report bugs and request features
- **[GitHub Discussions](https://github.com/shiba4life/fold_db/discussions)** — Questions and community discussion
