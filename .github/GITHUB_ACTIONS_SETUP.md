# GitHub Actions Setup Guide

This guide explains how to configure GitHub Actions for the FoldDB project, including how to set up environment variables for AI query testing.

## Overview

The project has multiple GitHub Actions workflows:
1. **`ci-tests.yml`** - All CI tests (Rust + Frontend + AI Query Test)
2. **`coverage.yml`** - Code coverage analysis

The AI query workflow test (`test_ai_query_workflow`) runs as part of the main CI test suite.

## Setting Up Environment Variables

### For AI Query Testing

The AI query workflow test (`test_ai_query_workflow`) **requires** AI configuration. Without it, this specific test will fail but other tests in the suite will still pass.

### Step 1: Add Secrets to GitHub Repository

1. Go to your GitHub repository
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add the following secrets (choose one AI provider):

#### Option A: OpenRouter API (Recommended)

```
Secret name: AI_PROVIDER
Secret value: openrouter

Secret name: FOLD_OPENROUTER_API_KEY
Secret value: your-openrouter-api-key
```

#### Option B: Ollama (Self-hosted)

```
Secret name: AI_PROVIDER
Secret value: ollama

Secret name: OLLAMA_BASE_URL
Secret value: http://your-ollama-server:11434
```

### Step 2: Verify Configuration

The workflow will automatically:
- **Verify** secrets are configured before running tests
- **Fail early** if AI_PROVIDER is missing or set to 'none'
- **Fail early** if required API keys are missing for the selected provider
- **Pass** only if AI configuration is valid and tests succeed

## Environment Variables Reference

### AI_PROVIDER

- **Description**: Which AI provider to use
- **Values**: `openrouter` or `ollama`
- **Required**: **YES** - test will fail if not set or set to `none`
- **Example**: `openrouter`

### FOLD_OPENROUTER_API_KEY

- **Description**: API key for OpenRouter service
- **Required**: **YES** if `AI_PROVIDER=openrouter`
- **How to get**: Sign up at https://openrouter.ai
- **Example**: `sk-or-v1-...`
- **Note**: Test will fail if provider is openrouter but key is missing

### OLLAMA_BASE_URL

- **Description**: Base URL for Ollama server
- **Required**: **YES** if `AI_PROVIDER=ollama`
- **Example**: `http://localhost:11434` or `http://your-ollama-server:11434`
- **Note**: Test will fail if provider is ollama but URL is missing

## Workflow Configuration in YAML

The main CI workflow (`ci-tests.yml`) includes AI environment variables:

```yaml
jobs:
  rust-tests:
    env:
      AI_PROVIDER: ${{ secrets.AI_PROVIDER }}
      FOLD_OPENROUTER_API_KEY: ${{ secrets.FOLD_OPENROUTER_API_KEY }}
      OLLAMA_BASE_URL: ${{ secrets.OLLAMA_BASE_URL }}
    steps:
      - run: cargo test --workspace  # Includes AI query test
```

**Key Points:**
- All tests run with `cargo test --workspace`
- AI secrets are optional for the workflow
- Only `test_ai_query_workflow` requires AI configuration
- Other tests pass regardless of AI configuration

## Testing Locally

**Note:** AI environment variables are REQUIRED for this test to pass.

### Run the Test

```bash
# AI credentials already configured locally
cargo test --test ai_query_workflow_integration_test -- --nocapture
```

### If Not Configured (Manual Setup)

```bash
export AI_PROVIDER=ollama
export OLLAMA_BASE_URL=http://localhost:11434
cargo test --test ai_query_workflow_integration_test -- --nocapture
```

## What Gets Tested

### Base Tests (Always Run)

- ✅ Server startup and initialization
- ✅ Base schema approval
- ✅ Sample data creation
- ✅ Declarative schema approval
- ✅ Transform backfill execution
- ✅ Declarative schema queries:
  - `BlogPostAuthorIndex`
  - `ProductTagIndex`
  - `MessageSenderIndex`
  - `UserActivityTypeIndex`

### AI Tests (REQUIRED - Will Fail if Not Configured)

- 🤖 AI configuration validation
- 🤖 AI query analysis
- 🤖 AI schema selection
- 🤖 AI query execution

**Important:** The test will **fail** if AI environment variables are not properly configured. This ensures AI query integration is always tested.

## Workflow Triggers

The CI test suite (including AI query test) runs on:

1. **Push** to any branch
2. **Pull Requests**

The AI query test runs automatically as part of `cargo test --workspace`.

## Reading Test Results

### Successful Run

```
✅ All Tests Passed

The declarative schemas integration test completed successfully.

Tests Verified:
- ✅ BlogPostAuthorIndex
- ✅ ProductTagIndex
- ✅ MessageSenderIndex
- ✅ UserActivityTypeIndex

ℹ️  AI query testing was skipped (no API keys configured)
```

### With AI Testing (Required)

```
✅ All Tests Passed

Tests Verified:
- ✅ BlogPostAuthorIndex
- ✅ ProductTagIndex
- ✅ MessageSenderIndex
- ✅ UserActivityTypeIndex
- ✅ AI Query Integration

💡 AI provider used: openrouter
```

**Note:** AI testing is REQUIRED. If not configured, the workflow will fail at the "Verify AI configuration" step before running tests.

### Failed Run

```
❌ Tests Failed

Possible Issues:
- Server startup failure
- Schema approval issues
- Transform backfill problems
- Query execution errors
```

Check the detailed logs in the Actions tab for specific error messages.

## Troubleshooting

### Workflow Fails at "Verify AI configuration"

**Problem**: Workflow fails before running tests  
**Cause**: AI environment variables not configured  
**Solution**: 
1. Go to GitHub repository Settings → Secrets and variables → Actions
2. Add required secrets:
   - `AI_PROVIDER` (set to `openrouter` or `ollama`)
   - `FOLD_OPENROUTER_API_KEY` (if using OpenRouter)
   - `OLLAMA_BASE_URL` (if using Ollama)
3. Re-run the workflow

### Test Fails with "AI_PROVIDER not set"

**Problem**: Test shows environment variable errors  
**Cause**: Secrets not properly configured or workflow not updated  
**Solution**: Verify secrets are named exactly:
- `AI_PROVIDER` (not `ai_provider` or `AI_Provider`)
- `FOLD_OPENROUTER_API_KEY` (not `OPENROUTER_API_KEY`)
- `OLLAMA_BASE_URL` (not `OLLAMA_URL`)

### API Key Invalid

**Problem**: AI query test fails with authentication error  
**Solution**: 
1. Verify API key is correct
2. Check API key has not expired
3. Ensure API key has sufficient credits (OpenRouter)

### Ollama Connection Failed

**Problem**: Cannot connect to Ollama server  
**Solution**:
1. Verify Ollama is running
2. Check URL is accessible from GitHub Actions runner
3. Consider using a public URL or OpenRouter instead

### Secrets Not Available in Forks

**Problem**: Pull requests from forks don't have access to secrets  
**Solution**: This is intentional security behavior. Secrets are only available:
- On the main repository
- For pushes (not PR from forks)
- After PR is merged

**Impact**: The declarative schemas test will **fail** on PRs from forks due to missing secrets. This is expected and acceptable - maintainers can run the test after merging.

## Security Best Practices

### ✅ Do

- ✅ Use GitHub Secrets for all sensitive data
- ✅ Rotate API keys regularly
- ✅ Use read-only or limited-scope API keys
- ✅ Monitor API usage and costs

### ❌ Don't

- ❌ Commit API keys in code
- ❌ Share API keys in PR comments
- ❌ Use production API keys for CI/CD
- ❌ Log API keys in test output

## Adding More Environment Variables

To add new environment variables to the workflow:

1. Add the secret in GitHub (Settings → Secrets)
2. Add to workflow YAML:
   ```yaml
   env:
     YOUR_VAR: ${{ secrets.YOUR_VAR || 'default' }}
   ```
3. Use in test code:
   ```rust
   let value = std::env::var("YOUR_VAR").unwrap_or_default();
   ```

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [GitHub Secrets Documentation](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [OpenRouter API Docs](https://openrouter.ai/docs)
- [Ollama Documentation](https://ollama.ai/docs)

## Support

For issues with:
- **GitHub Actions setup**: Check this guide or GitHub docs
- **API keys**: Contact your API provider
- **Test failures**: Check test logs and server.log artifact

---

**Last Updated**: October 11, 2025  
**Workflow Version**: 1.0

