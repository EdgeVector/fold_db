# GitHub Actions Configuration

## Quick Setup

To enable AI query testing in the CI workflow, configure these secrets:

### Required Secrets

Go to: **Repository Settings** → **Secrets and variables** → **Actions** → **New repository secret**

#### For OpenRouter (Recommended):
```
Name: AI_PROVIDER
Value: openrouter

Name: FOLD_OPENROUTER_API_KEY
Value: <your-openrouter-api-key>
```

#### For Ollama (Self-hosted):
```
Name: AI_PROVIDER
Value: ollama

Name: OLLAMA_BASE_URL
Value: http://your-ollama-server:11434
```

## What Happens Without Secrets?

⚠️  The AI query workflow test will fail (but other tests still pass)  
✅ The main CI workflow runs all tests with `cargo test --workspace`  
ℹ️  Only the `test_ai_query_workflow` test requires AI configuration

## Detailed Documentation

See [GITHUB_ACTIONS_SETUP.md](./GITHUB_ACTIONS_SETUP.md) for:
- Complete setup instructions
- Environment variable reference
- Troubleshooting guide
- Testing locally
- Security best practices

## Workflows

| Workflow | Purpose | AI for Full Pass |
|----------|---------|------------------|
| `ci-tests.yml` | All tests including AI query test | Optional* |
| `coverage.yml` | Code coverage | No |

*AI secrets are optional - if not set, the AI query test will fail but other tests will still pass.

## Getting API Keys

### OpenRouter (Recommended)
1. Go to https://openrouter.ai
2. Sign up for an account
3. Generate an API key
4. Add credits to your account
5. Add the key as `FOLD_OPENROUTER_API_KEY` secret

### Ollama (Self-hosted)
1. Install Ollama: https://ollama.ai
2. Run Ollama server: `ollama serve`
3. Ensure it's accessible at a URL
4. Add the URL as `OLLAMA_BASE_URL` secret

## Support

For setup help, see the detailed guide: [GITHUB_ACTIONS_SETUP.md](./GITHUB_ACTIONS_SETUP.md)

