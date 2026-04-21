# FoldDB

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/EdgeVector/fold_db)

**Your personal database with AI that organizes everything for you.**

Drop in files, JSON, or social media exports — FoldDB detects schemas, extracts searchable keywords, and lets you query with natural language. Runs locally on your machine. Your data stays yours.

**Try it now** — install with one command and have a working database in under a minute:

```bash
brew install edgevector/folddb/folddb
folddb daemon start
# Open http://localhost:9101 — drag in a JSON file, ask a question
```

## Quick Start

### Option A: Desktop App (macOS)

Download the latest `.dmg` from [GitHub Releases](https://github.com/EdgeVector/fold_db_node/releases), open it, and drag **FoldDB.app** to Applications. Double-click to launch — no terminal needed.

### Option B: Install via Homebrew (macOS + Linux x86_64)

```bash
brew install edgevector/folddb/folddb
```

Ships `folddb` (CLI) and `folddb_server` (daemon). Handles upgrades via `brew upgrade folddb`. The schema service runs in the cloud at `schema.folddb.com` — no local binary is installed.

### Option C: Install from Source

Fallback for platforms not covered by the tap (Linux arm64, air-gapped). Downloads the latest release tarball from the `EdgeVector/fold_db` mirror and verifies its sha256:

```bash
curl -fsSL https://raw.githubusercontent.com/EdgeVector/fold_db_node/main/install.sh | sh
```

Or build from source:

```bash
cargo install --git https://github.com/EdgeVector/fold_db_node folddb folddb_server
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
- **Native Face Detection** — On-device face detection (SCRFD model) for photo fingerprinting; no cloud calls
- **Fingerprints, Personas, Identities** — Observe identity signals during ingestion; cluster into Personas; anchor to verified Identities via signed Identity Cards exchanged over E2E messaging
- **Cross-User Sharing** — Share schema records with other nodes via signed share rules; query `from:{sender}` namespaces; sync engine auto-refreshes on rule changes
- **AccessTier access control** — Field-level tiers (0=Public … 4=Owner) plus capability tokens and payment gates; every molecule write is signed with Ed25519 for cryptographic provenance
- **Web UI** — Browse schemas, ingest data, query, manage personas/fingerprints/identities, and configure sharing from your browser
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
- People tab: Personas, Identities, fingerprints, sharing
- System status and AI configuration (vision backend picker)

The main UI has tabs for:
- **Schemas** — browse, approve, and inspect schemas
- **Ingestion** — upload files, configure smart folders, import Twitter/Apple/social data
- **Query** — natural language questions, structured field queries, full-text search
- **Native Index** — keyword-based search across all ingested data
- **Word Graph** — visual exploration of how your data connects
- **People** — Personas (filter, sort, undo-delete), Identity Cards (issue/receive/import), Fingerprints
- **Settings** — AI provider config, vision backend picker, system status, database management

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
- Node.js 18+ (for web UI)
- Git LFS (for test fixtures — run `git lfs install && git lfs pull`)

### Building from Source

```bash
git clone https://github.com/EdgeVector/fold_db.git
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
cargo install --git https://github.com/EdgeVector/fold_db.git --bin folddb
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

## Troubleshooting

<details>
<summary><strong>Port 9001 or 5173 already in use</strong></summary>

Another process is occupying the port. Kill it or pick a different one:

```bash
# Find what's using port 9001
lsof -i :9001
# Kill it
kill -9 <PID>
# Or use a different port
folddb_server --port 9002
```

`./run.sh --local` automatically kills existing FoldDB processes before starting.

</details>

<details>
<summary><strong>Ollama not detected</strong></summary>

FoldDB auto-detects Ollama at `http://127.0.0.1:11434`. If it's not found:

1. Make sure Ollama is installed and running: `ollama serve`
2. Verify it's reachable: `curl http://127.0.0.1:11434/api/tags`
3. Pull a model if you haven't: `ollama pull llama3.1`
4. If Ollama is on a different host/port, set it in Settings > AI Provider

</details>

<details>
<summary><strong>OpenRouter API key errors</strong></summary>

- **"API key invalid or expired"** — Check your key at [openrouter.ai/keys](https://openrouter.ai/keys)
- **"insufficient credits"** — Add funds at [openrouter.ai/credits](https://openrouter.ai/credits)
- **"model not found"** — The configured model may have been removed; switch to a different one in Settings

Set your key: `export FOLD_OPENROUTER_API_KEY="sk-or-v1-..."` or configure in the Settings tab.

</details>

<details>
<summary><strong>macOS: "FoldDB.app is damaged" or Gatekeeper blocks</strong></summary>

The app is not code-signed. To open it:

1. Right-click `FoldDB.app` > **Open** > click **Open** in the dialog
2. Or: **System Settings > Privacy & Security**, scroll down, click **Open Anyway**
3. If that fails: `xattr -cr /Applications/FoldDB.app`

</details>

<details>
<summary><strong>Build errors</strong></summary>

- **"can't find crate"** — Run `cargo clean && cargo build`
- **Frontend embed error** — The React frontend must be built first: `cd src/server/static-react && npm ci && npm run build`
- **Git LFS pointer files** — Run `git lfs install && git lfs pull` to download test fixtures

</details>

## FAQ

<details>
<summary><strong>How much disk space does FoldDB use?</strong></summary>

Data is stored in `~/.datafold/data`. Space depends on what you ingest. A typical Twitter archive with 10k tweets uses ~50 MB including indexes. Uploaded files are encrypted and stored separately.

</details>

<details>
<summary><strong>How do I back up my data?</strong></summary>

Copy the `~/.datafold/data` directory. It contains the Sled database, encrypted uploads, and configuration. To restore, copy it back.

</details>

<details>
<summary><strong>Can I use FoldDB fully offline?</strong></summary>

Yes. Use `./run.sh --local --local-schema` with Ollama as your AI provider. No internet connection is needed.

</details>

<details>
<summary><strong>How does deduplication work?</strong></summary>

FoldDB deduplicates at two levels:
- **Schema-level**: Schemas are content-addressed by their field structure (identity hash). Ingesting data with the same shape reuses the existing schema.
- **File-level**: Each file is SHA256-hashed. Re-ingesting the same file for the same user is skipped automatically.

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

- **[GitHub Issues](https://github.com/EdgeVector/fold_db/issues)** — Report bugs and request features
- **[GitHub Discussions](https://github.com/EdgeVector/fold_db/discussions)** — Questions and community discussion
