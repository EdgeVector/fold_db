# Authentication Development Mode

## Current Status

**All endpoints currently operate in development mode with authentication disabled.**

## Overview

The DataFold system is currently configured to operate without authentication requirements for all endpoints. This simplifies development and testing workflows by removing the complexity of cryptographic signatures and key management.

## Implementation Details

### Mutation Endpoint
- **Location**: `src/datafold_node/query_routes.rs:120-135`
- **Implementation**: Uses mock verification result with `is_valid: true`
- **Identity**: All requests automatically use "web-ui" identity
- **Trust Distance**: Set to `0` for all operations

```rust
// Create a mock verification result for mutations (no signing required)
let verification_data = VerificationResult {
    is_valid: true,
    public_key_info: Some(crate::security::types::PublicKeyInfo {
        id: "web-ui".to_string(),
        // ... mock verification data
    }),
    // ...
};
```

### All Endpoints
- **Authentication**: None required
- **Default Identity**: "web-ui" with `trust_distance: 0`
- **Signature Verification**: Disabled
- **Key Management**: Not required

## Affected Endpoints

| Endpoint Category | Authentication Status | Notes |
|------------------|---------------------|-------|
| `/api/query` | None required | Uses "web-ui" identity |
| `/api/mutation` | None required | Mock verification |
| `/api/schemas/*` | None required | All schema operations |
| `/api/system/*` | None required | Administrative functions |
| `/api/ingestion/*` | None required | Data ingestion |
| `/api/logs/*` | None required | Log access |
| `/api/security/*` | None required | Security operations |

## Security Implications

### Development Mode Benefits
- **Simplified Development**: No need to manage keys or signatures
- **Faster Testing**: Direct API calls without authentication setup
- **Reduced Complexity**: Focus on core functionality development

### Security Considerations
- **Not Production Ready**: This configuration should not be used in production
- **All Operations Unrestricted**: Any client can perform any operation
- **No Access Control**: No user-based permissions or restrictions
- **Administrative Access**: Database reset and other admin functions are accessible

## Migration to Production

When ready for production deployment, the following changes will be needed:

1. **Enable Authentication**: Implement proper Ed25519 signature verification
2. **Key Management**: Add public key registration and management
3. **Permission System**: Implement user-based access control
4. **Secure Endpoints**: Protect administrative and sensitive operations
5. **Session Management**: Add proper authentication flows

## Documentation Updates

The following documentation has been updated to reflect the current development mode:

- `docs/http_routes.md`: Updated authentication requirements
- `docs/ui_backend_alignment.md`: Updated authentication integration notes
- `docs/security_review_report.md`: Updated security assessment
- `docs/project_logic.md`: Added AUTH-DEV-001 logic entry
- `docs/delivery/PBI-AUTH-001/prd.md`: Added development mode status note

## Testing

All API endpoints can be tested directly without authentication:

```bash
# Example mutation (no authentication required)
curl -X POST http://localhost:9001/api/mutation \
  -H "Content-Type: application/json" \
  -d '{
    "type": "mutation",
    "schema": "BlogPost",
    "mutation_type": "create",
    "data": {
      "author": "test",
      "title": "test title",
      "content": "hello world test",
      "tags": "test",
      "publish_date": "2025-01-27T10:00:00Z"
    }
  }'
```

## Notes

- This development mode is intentional and documented
- All authentication-related PBIs are currently not implemented
- The system uses mock verification for all operations
- No changes to client-side code are needed for development
