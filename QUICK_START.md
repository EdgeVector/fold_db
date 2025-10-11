# Quick Start - Declarative Schemas & AI Testing

## 🚀 Get Started in 3 Steps

### 1. Setup Sample Data
```bash
./run_http_server.sh
python3 scripts/setup_sample_schemas.py
```

### 2. Test AI Queries
```bash
export AI_PROVIDER=openrouter
export FOLD_OPENROUTER_API_KEY=your-key
cargo test --test ai_query_workflow_integration_test -- --nocapture
```

### 3. Configure GitHub Actions
```
Repository Settings → Secrets → Actions → New secret
  1. AI_PROVIDER = openrouter
  2. FOLD_OPENROUTER_API_KEY = your-key
```

## 📚 Documentation

- **Overview**: `DECLARATIVE_SCHEMAS_README.md`
- **Schema Details**: `docs/README_DECLARATIVE_SCHEMAS.md`
- **Schema Reference**: `docs/README_SCHEMAS_REFERENCE.md`
- **AI Examples**: `docs/AI_QUERY_EXAMPLES.md`
- **GitHub Actions**: `.github/GITHUB_ACTIONS_SETUP.md`

## ✅ What You Get

- **25 schemas** (8 base + 17 declarative)
- **Sample data** (~88 records)
- **AI query testing** (26 assertions)
- **CI/CD ready** (GitHub Actions configured)

**Status**: ✅ Production Ready
