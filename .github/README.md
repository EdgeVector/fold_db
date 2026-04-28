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

| Workflow | Purpose | Trigger | AI Required |
|----------|---------|---------|-------------|
| `ci-tests.yml` | Run all tests (Rust + Frontend) | Push/PR | Optional* |
| `coverage.yml` | Generate code coverage | Push/PR | No |
| `release.yml` | Build release binaries | Version tags | No |

*AI secrets are optional - if not set, the AI query test will fail but other tests will still pass.

### Release Workflow

The `release.yml` workflow automatically builds binaries when you push a version tag:

**Platforms:**
- macOS (Intel x86_64)
- macOS (Apple Silicon aarch64)
- Linux (x86_64)

**How to trigger a release:**
```bash
# 1. Update version in Cargo.toml
# 2. Commit changes
git add .
git commit -m "Release v0.1.6"

# 3. Create and push version tag
git tag v0.1.6
git push origin main --tags

# 4. GitHub Actions will automatically:
#    - Build binaries for all platforms
#    - Create a GitHub release
#    - Attach binaries as release assets
```

**Release binaries are named:**
- `folddb_server-macos-x86_64-{version}`
- `folddb_server-macos-aarch64-{version}`
- `folddb_server-linux-x86_64-{version}`

For detailed workflow validation and troubleshooting, see [RELEASE_WORKFLOW_VALIDATION.md](./RELEASE_WORKFLOW_VALIDATION.md).

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

