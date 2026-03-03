# FoldDB: Product Overview & Launch Plan

## 1. What is FoldDB?

FoldDB is a **personal database that uses AI to automatically organize your data**. You drop in files, JSON, or social media exports -- FoldDB detects schemas, extracts searchable keywords, and lets you query everything with natural language.

**The core idea:** Your data should live with you, not in corporate silos. FoldDB gives individuals a private, AI-powered data store that runs on their own machine.

### Value Proposition

| For whom | Problem | FoldDB solution |
|---|---|---|
| **Privacy-conscious individuals** | Personal data scattered across dozens of services | One local database that ingests and unifies everything |
| **Power users / developers** | No easy way to query across personal data exports | Natural language queries + structured query builder |
| **Data sovereignty advocates** | Cloud platforms own and monetize user data | Local-first, encrypted, open-source |

### How It Works (User Experience)

1. **Install** -- One-line install for macOS/Linux, or download the desktop app
2. **Configure AI** -- Pick OpenRouter (cloud) or Ollama (local/private) for the AI engine
3. **Ingest data** -- Point at a folder, upload files, or paste JSON. AI automatically detects structure, creates schemas, and indexes everything
4. **Ask questions** -- "What taxes did I pay last year?" / "Show my travel plans" / "What data do I have?"

### Key Features

- **Smart Folder Ingestion** -- Point at any directory. AI scans, classifies files (personal data, media, config, etc.), shows cost estimates, and batch-ingests with progress tracking and spend limits
- **AI-Powered Natural Language Queries** -- Conversational chat interface backed by an autonomous AI agent that translates questions into structured database queries
- **Automatic Schema Detection** -- No manual schema definition needed. AI analyzes data structure, checks a global schema registry for consistency, and creates schemas automatically
- **Full-Text Search** -- Native keyword index built at write time. Every field value is tokenized and searchable
- **Word Graph Visualization** -- Interactive force-directed graph showing connections between words and schemas across your data
- **Multiple Input Formats** -- JSON, CSV, PDF, images (EXIF metadata), social media exports (Twitter, Instagram, LinkedIn, TikTok)
- **Data Browser** -- Hierarchical explorer with version history and metadata for every record
- **Encryption at Rest** -- AES-256-GCM encryption with local keys
- **Runs Anywhere** -- Desktop app (Tauri/macOS), CLI, web UI, or cloud-hosted

### AI Provider Support

| Provider | Privacy | Cost | Models |
|---|---|---|---|
| **Ollama** (local) | Data never leaves your machine | Free (your hardware) | llama3.3, any Ollama model |
| **OpenRouter** (cloud) | Data sent to cloud LLM | Pay-per-use | Gemini 2.5 Flash, Claude Sonnet 4.6, GPT-4.1, DeepSeek V3 |

---

## 2. Current State

**Version:** 0.3.0

### What Works Today

| Capability | Status |
|---|---|
| Local Sled storage backend | Production-ready |
| AWS DynamoDB + S3 cloud backend | Production-ready |
| AI ingestion (JSON, file upload, smart folder) | Working (OpenRouter + Ollama) |
| Natural language AI queries (agent mode) | Working |
| Global schema service (schema.folddb.com) | Deployed |
| Native keyword index + full-text search | Working |
| Schema management UI | Working |
| Structured query builder + mutation editor | Working |
| Data browser with version history | Working |
| Word graph visualization | Working |
| Onboarding wizard (6-step) | Working |
| CLI (schema, ingest, query commands) | Working |
| One-line installer (macOS + Linux) | Working |
| Tauri desktop app (macOS) | Buildable, not shipping |
| Cloud migration (local -> Exemem) | Working |
| 450+ frontend tests, CI pipeline | Passing |

### What's Not Ready

| Gap | Impact on launch |
|---|---|
| No polished desktop app distribution (DMG/brew) | Major -- primary install path |
| No Windows support | Limits addressable market |
| E2E encryption not implemented (design complete) | Not a launch blocker, but differentiator |
| Error handling rough in some ingestion edge cases | User experience |
| No usage analytics or crash reporting | Can't measure adoption |
| No automated update mechanism | Manual updates only |
| No onboarding for non-technical users | Limits audience |
| No landing page or marketing site | No way to discover the product |

---

## 3. Target Audience for Launch

### Primary: Technical Early Adopters

- Developers, data engineers, security researchers
- Active on Hacker News, Reddit r/selfhosted, r/privacy, r/datahoarder
- Comfortable with CLI install, configuring Ollama
- Value: open-source, local-first, AI-powered

### Secondary: Privacy-Conscious Power Users

- People who request their data exports from Google, Twitter, Facebook
- Use password managers, VPNs, encrypted messaging
- Interested in data sovereignty but may not be developers
- Value: "my data stays on my machine" + natural language queries

### Not Yet (Post-Launch)

- Non-technical consumers (need polished desktop app + auto-updates)
- Enterprise/B2B (need multi-user, compliance features)
- P2P network users (Phase 3-4 from strategy doc)

---

## 4. Launch Strategy

### Launch Type: Open-Source Developer Preview

Position FoldDB as an **open-source project for technical early adopters**. The goal is to build a community of contributors and power users who validate the product before expanding to a broader audience.

### Launch Channels

| Channel | Action | Expected impact |
|---|---|---|
| **Hacker News** | "Show HN" post with demo video | Primary driver of technical early adopters |
| **Reddit** | Posts to r/selfhosted, r/privacy, r/datahoarder, r/rust | Community-aligned audiences |
| **GitHub** | Polish README, add screenshots, write CONTRIBUTING.md | Organic discovery + contributor funnel |
| **Twitter/X** | Thread demonstrating Twitter export -> AI query flow | Viral potential ("query your own tweets with AI") |
| **Dev blogs** | Write-up on architecture (Rust + AI + local-first) | SEO + developer credibility |
| **YouTube** | 3-5 minute demo video | Embed in HN post and README |

### Launch Narrative

> "What if you could drop all your personal data into one place and ask it questions?"
>
> FoldDB is an open-source personal database. Point it at your Twitter export, tax documents, or photo library. AI automatically organizes everything and lets you query with natural language -- and your data never leaves your machine.

---

## 5. Pre-Launch Tasks

### Track 1: Product Polish (Engineering)

| # | Task | Priority | Effort | Description |
|---|---|---|---|---|
| 1.1 | Ship macOS desktop app via DMG | P0 | 3 days | Build pipeline exists. Need: signed DMG, README install instructions, test on clean machine |
| 1.2 | Homebrew formula | P0 | 1 day | `brew install folddb` for the CLI. Cask for the desktop app |
| 1.3 | Fix ingestion error handling | P0 | 2 days | Audit error paths: LLM failures, malformed files, network timeouts. Show clear user-facing messages |
| 1.4 | Improve onboarding wizard | P1 | 2 days | Add "Import Twitter export" as a guided first-run option. Pre-populate sample data for demo |
| 1.5 | Privacy warning UX | P1 | 1 day | Make OpenRouter vs Ollama privacy tradeoff clearer during setup. Default to Ollama if installed |
| 1.6 | Performance pass | P1 | 2 days | Profile smart folder scan + ingestion on large directories (1000+ files). Fix bottlenecks |
| 1.7 | Linux arm64 support | P2 | 1 day | Verify/fix install script for Raspberry Pi / Linux ARM (self-hosting audience) |
| 1.8 | Windows support (WSL guide) | P2 | 1 day | Document WSL2 install path. Native Windows is post-launch |

### Track 2: Content & Marketing

| # | Task | Priority | Effort | Description |
|---|---|---|---|---|
| 2.1 | Landing page (folddb.com) | P0 | 3 days | Single page: hero, demo GIF, features, install command, GitHub link |
| 2.2 | Demo video (3-5 min) | P0 | 2 days | Install -> ingest Twitter export -> ask "what are my most liked tweets?" -> show results |
| 2.3 | README overhaul | P0 | 1 day | Screenshots, GIFs, clear install instructions, feature list, architecture diagram |
| 2.4 | Write Show HN post | P0 | 0.5 day | Concise, honest, focus on the "drop files, ask questions" narrative |
| 2.5 | Architecture blog post | P1 | 2 days | "Building a personal AI database in Rust" -- technical deep dive |
| 2.6 | Twitter thread | P1 | 0.5 day | "I built an AI that organizes all your personal data locally" -- with screenshots |
| 2.7 | CONTRIBUTING.md | P1 | 0.5 day | How to contribute, development setup, architecture overview for new contributors |

### Track 3: Infrastructure

| # | Task | Priority | Effort | Description |
|---|---|---|---|---|
| 3.1 | GitHub releases with binaries | P0 | 1 day | CI pipeline to build + publish macOS (Intel/ARM) and Linux x86_64 binaries |
| 3.2 | Crash reporting (opt-in) | P1 | 1 day | Sentry or similar, opt-in during setup, privacy-respecting |
| 3.3 | Usage analytics (opt-in) | P2 | 1 day | Anonymous install count + feature usage. PostHog or self-hosted Plausible |
| 3.4 | Auto-update mechanism | P2 | 2 days | Tauri built-in updater for desktop app. CLI: check latest GitHub release |

---

## 6. Timeline

### Week 1-2: Foundation

| Week | Tasks | Milestone |
|---|---|---|
| Week 1 | 1.1 (macOS DMG), 1.3 (error handling), 2.3 (README), 3.1 (GitHub releases) | Installable, polished binary available |
| Week 2 | 1.2 (Homebrew), 1.4 (onboarding), 2.1 (landing page), 2.7 (CONTRIBUTING) | Landing page live, brew install works |

### Week 3: Content

| Week | Tasks | Milestone |
|---|---|---|
| Week 3 | 2.2 (demo video), 2.5 (blog post), 2.6 (Twitter thread), 1.5 (privacy UX) | All launch content ready |

### Week 4: Launch

| Day | Action |
|---|---|
| Monday | Final QA: fresh install on macOS + Linux, full onboarding flow, AI query demo |
| Tuesday | Publish blog post |
| Wednesday | **Show HN post** (9am ET), Twitter thread, Reddit posts |
| Thursday | Monitor feedback, respond to comments, fix critical bugs |
| Friday | Publish "What we learned from launch" follow-up, triage GitHub issues |

### Post-Launch (Weeks 5-8)

| Week | Focus |
|---|---|
| Week 5-6 | Bug fixes from user feedback, performance improvements, contributor onboarding |
| Week 7-8 | Windows support (native or WSL), auto-update, E2E encryption prototype |

---

## 7. Success Metrics

### Launch Week

| Metric | Target |
|---|---|
| GitHub stars | 500+ |
| HN post rank | Front page (top 30) |
| Installs (unique) | 200+ |
| GitHub issues filed | 20+ (signal of real usage) |
| Contributors (PRs) | 3+ |

### First Month

| Metric | Target |
|---|---|
| GitHub stars | 2,000+ |
| Monthly active installs | 500+ |
| Community Discord/forum members | 100+ |
| External blog posts / mentions | 5+ |

### Three Months

| Metric | Target |
|---|---|
| GitHub stars | 5,000+ |
| Monthly active installs | 2,000+ |
| Regular contributors | 10+ |
| Schema service registered schemas | 500+ (signal of diverse data ingestion) |

---

## 8. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| AI ingestion fails on common file formats | Medium | High | Pre-launch testing with Twitter, Google Takeout, Apple Health, bank statements |
| Ollama too hard to install for non-devs | High | Medium | Clear docs, fallback to OpenRouter, detect Ollama on startup |
| OpenRouter costs surprise users | Medium | Medium | Show cost estimates before ingestion, spend limits already built |
| Schema service becomes bottleneck | Low | High | Local schema cache, offline mode already works |
| Security vulnerability discovered | Low | Critical | Security audit before launch, responsible disclosure policy, bug bounty |
| "Just another AI wrapper" perception | Medium | High | Lead with data sovereignty narrative, emphasize local-first + open-source |

---

## 9. Long-Term Vision (Post-Launch Roadmap)

The launch is **Phase 1** of the broader vision described in [STRATEGY.md](./STRATEGY.md):

| Phase | Timeline | Description |
|---|---|---|
| **Phase 1: Local-First PDN** | Launch + 3 months | What we're shipping. Local database, AI ingestion, natural language queries |
| **Phase 2: Policy Engine** | Months 4-9 | Fine-grained disclosure controls. "Share my income summary with this lender, but not raw transactions" |
| **Phase 3: P2P Connections** | Months 10-18 | Node-to-node connections. Query a friend's node (with their permission). Institutions become peers |
| **Phase 4: AI Mediator** | Months 18+ | AI computes insights without leaking raw data. Verified claims, inference attack detection |

Each phase delivers standalone value. The launch establishes FoldDB as the best local-first personal database. Future phases transform it into a decentralized data network.

---

## 10. Competitive Landscape

| Product | Overlap | FoldDB differentiator |
|---|---|---|
| **Apple Spotlight / Siri** | Local search + AI | FoldDB ingests structured data, not just files. Natural language queries return data, not links |
| **Obsidian** | Local-first knowledge base | FoldDB handles structured data (schemas, queries, mutations), not just markdown |
| **Solid (Inrupt)** | Data sovereignty pods | FoldDB is a working product today with AI. Solid is a protocol/spec |
| **Notion AI** | AI-powered data organization | FoldDB is local-first and open-source. Notion is cloud-only and proprietary |
| **Personal AI / Rewind** | Personal AI assistant | FoldDB is a database you own, not a surveillance tool. Open-source, no continuous recording |
| **SQLite + ChatGPT** | DIY local AI + DB | FoldDB automates the entire pipeline: ingestion, schema creation, indexing, querying |

---

*This document should be treated as a living plan. Update as priorities shift and user feedback comes in.*
