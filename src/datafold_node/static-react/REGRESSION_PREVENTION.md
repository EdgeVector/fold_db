# UI Regression Prevention Guide

This document outlines the measures implemented to prevent endpoint mismatches and similar UI regressions.

## The Problem We're Solving

**Endpoint Drift**: Frontend and backend endpoints become misaligned over time, causing:
- "Unexpected end of JSON input" errors (when hitting non-existent endpoints)
- Silent failures in production
- Time-consuming debugging sessions

## Prevention Strategy

### 1. 🎯 Centralized API Endpoints

**File**: `src/api/endpoints.ts`

All API endpoints are defined in a single location:

```typescript
export const API_ENDPOINTS = {
  MUTATION: '/api/mutation',
  QUERY: '/api/query',
  // ... more endpoints
} as const;
```

**✅ Benefits**:
- Single source of truth
- Type-safe endpoint access
- Easy to update all references

### 2. 🏗️ API Client Classes

**Files**: 
- `src/api/mutationClient.ts`
- `src/api/schemaClient.ts` 
- `src/api/securityClient.ts`

Centralized API clients handle all HTTP communication:

```typescript
// ❌ Before (prone to errors)
const response = await fetch('/api/data/mutate', {...})

// ✅ After (centralized & safe)
const response = await MutationClient.executeMutation(signedMessage)
```

**✅ Benefits**:
- Consistent error handling
- Type-safe responses
- Easier testing and mocking

### 3. 🛡️ ESLint Rules (Auto-Prevention)

**File**: `.eslintrc.cjs`

ESLint rules prevent hardcoded URLs at development time:

```javascript
rules: {
  'no-restricted-syntax': [
    'error',
    {
      selector: "Literal[value=/^\\/api\\//]",
      message: "🚫 Use API_ENDPOINTS instead of hardcoded '/api/' URLs"
    }
  ]
}
```

**✅ Benefits**:
- Catches issues during development
- IDE integration shows warnings immediately
- Prevents regressions before they're committed

### 4. 🧪 Automated Validation Tests

**File**: `src/test/validation/endpointValidation.test.js`

Tests that validate endpoint consistency:

```javascript
describe('Backend Route Compatibility', () => {
  it('should have matching endpoint for backend route: /api/mutation', () => {
    expect(API_ENDPOINTS.MUTATION).toBe('/api/mutation');
  });
});
```

**✅ Benefits**:
- Catches mismatches in CI/CD
- Documents expected backend routes
- Prevents deployment of broken endpoints

## Usage Guidelines

### ✅ DO: Use Centralized Clients

```typescript
// Mutations
import { MutationClient } from '../api/mutationClient';
const result = await MutationClient.executeMutation(data);

// Schema operations  
import { getAllSchemasWithState } from '../api/schemaClient';
const schemas = await getAllSchemasWithState();
```

### ❌ DON'T: Use Direct fetch() Calls

```typescript
// This will trigger ESLint errors
const response = await fetch('/api/mutation', {...});
const response = await fetch('/api/data/mutate', {...}); // Wrong endpoint!
```

## Running Validation

### Check for Regressions

```bash
# Run endpoint validation tests
npm run test src/test/validation/

# Run ESLint to catch hardcoded URLs
npm run lint

# Run all tests before committing
npm run test
```

### Integration with CI/CD

Add these checks to your CI pipeline:

```yaml
- name: Validate API Endpoints
  run: npm run test src/test/validation/endpointValidation.test.js

- name: Check for Hardcoded URLs
  run: npm run lint
```

## When Adding New Endpoints

1. **Add to `endpoints.ts`** first
2. **Update the appropriate client** (MutationClient, SchemaClient, etc.)
3. **Add validation tests** for the new endpoint
4. **Update components** to use the client
5. **Run tests** to ensure everything works

## Quick Reference

| Component Type | Use This Client | Example |
|---------------|-----------------|---------|
| Mutations | `MutationClient` | `MutationClient.executeMutation()` |
| Schema Ops | `schemaClient` | `getAllSchemasWithState()` |
| Security | `securityClient` | `verifyMessage()` |

## Troubleshooting

**Q: I'm getting "Unexpected end of JSON input"**
A: Check that your endpoint exists in `endpoints.ts` and matches the backend route

**Q: ESLint is complaining about my fetch() call**
A: Use the appropriate API client instead of direct fetch()

**Q: How do I add a new endpoint?**
A: Follow the "When Adding New Endpoints" checklist above

---

This system prevents the endpoint mismatch issues that caused the original "Unexpected end of JSON input" error with TransformBase mutations.