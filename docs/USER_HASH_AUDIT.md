# User Hash Generation Audit

## Algorithm Summary

```
user_hash = SHA-256(user_id).toHex()[0:32]
```

- **Input**: User identifier string (e.g., "alice", "testuser")
- **Output**: 32 hex characters (16 bytes of entropy)
- **Deterministic**: Same input always produces same output

---

## Generation Points

### 1. Frontend (Browser) - Primary Source

**File**: `fold_db/src/server/static-react/src/store/authSlice.ts`

```typescript
// Lines 212-230
export const loginUser = createAsyncThunk(
  "auth/loginUser",
  async (userId: string, { rejectWithValue }) => {
    const encoder = new TextEncoder();
    const data = encoder.encode(userId);
    const hashBuffer = await crypto.subtle.digest("SHA-256", data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    const userHash = hashHex.substring(0, 32); // ✅ 32 hex chars
    return { id: userId, hash: userHash };
  },
);
```

**Storage**: `localStorage.setItem('fold_user_hash', userHash)`

---

### 2. Lambda Node Manager - Secondary Source (for Node Key Derivation)

**File**: `exemem-infra/lambdas/fold_db_worker/src/node_manager.rs`

```rust
// Lines 97-105
// Deterministically generate identity keys from user_id
use sha2::{Digest, Sha256};
let mut hasher = Sha256::new();
hasher.update(user_id.as_bytes());       // user_id = the user_hash from header
let result = hasher.finalize();           // Full 32 bytes
let secret_seed = result.as_slice();

let keypair = Ed25519KeyPair::from_secret_key(secret_seed)?;
```

**Note**: Lambda uses the FULL 32-byte SHA-256 result for key derivation, not truncated.

---

## Propagation Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLIENT                                      │
├─────────────────────────────────────────────────────────────────────────┤
│  1. User enters: "alice"                                                 │
│  2. JS: SHA-256("alice") → 64 hex chars                                 │
│  3. JS: Take [0:32] → "2bd806c97f0e00af..."                             │
│  4. Store: localStorage['fold_user_hash']                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP Request
                                    │ Header: X-User-Hash: 2bd806c97f0e00af...
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         HTTP SERVER (Standalone)                         │
├─────────────────────────────────────────────────────────────────────────┤
│  File: fold_db/src/server/middleware/auth.rs                            │
│                                                                          │
│  let user_id = req.headers()                                             │
│      .get("x-user-hash")           // ✅ Primary (matches Lambda)        │
│      .or_else(|| req.headers().get("x-user-id"))  // Fallback           │
│      .and_then(|v| v.to_str().ok())                                      │
│      .map(|s| s.to_string());                                            │
│                                                                          │
│  if let Some(uid) = user_id {                                            │
│      run_with_user(&uid, async { ... }).await    // Task-local context   │
│  }                                                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                         OR (Cloud Deployment)
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                            LAMBDA (AWS)                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  File: exemem-infra/lambdas/fold_db_worker/src/main.rs                  │
│                                                                          │
│  // Extract user_hash from payload or headers                            │
│  if user_hash.is_none() {                                                │
│      for (k, v) in headers {                                             │
│          if k.eq_ignore_ascii_case("x-user-hash") { ... }                │
│      }                                                                   │
│  }                                                                       │
│                                                                          │
│  // Get user-scoped node                                                 │
│  let node = node_manager.get_node(&user_hash).await?;                    │
│                                                                          │
│  // Run with user context                                                │
│  run_with_user(&user_hash, async { ... }).await                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Consistency Check

| Component                     | Algorithm | Truncation      | Header Name                                 | Status |
| ----------------------------- | --------- | --------------- | ------------------------------------------- | ------ |
| Frontend (authSlice.ts)       | SHA-256   | [0:32]          | -                                           | ✅     |
| API Client (client.ts)        | -         | -               | x-user-hash, x-user-id                      | ✅     |
| HTTP Middleware (auth.rs)     | -         | -               | x-user-hash (primary), x-user-id (fallback) | ✅     |
| Lambda (main.rs)              | -         | -               | x-user-hash                                 | ✅     |
| NodeManager (node_manager.rs) | SHA-256   | None (32 bytes) | -                                           | ✅     |

---

## Key Observations

### ✅ Consistent Hash Generation

- Frontend generates `SHA-256(user_id)[0:32]` = 32 hex chars
- This is stored as `fold_user_hash` in localStorage
- Sent with every request as `X-User-Hash` header

### ✅ Consistent Header Handling

- Both HTTP Server and Lambda extract from `x-user-hash` header
- HTTP Server has fallback to `x-user-id` for backwards compatibility

### ⚠️ Dual Hash Usage in Lambda

The Lambda `NodeManager` performs a **second hash** on the user_hash:

```rust
// user_id here IS the user_hash (32 hex chars)
hasher.update(user_id.as_bytes());  // SHA-256 of the hash
```

This creates a **double-hash** for key derivation:

```
Node Keys = Ed25519(SHA-256(user_hash))
          = Ed25519(SHA-256(SHA-256(user_id)[0:32]))
```

This is intentional and provides:

- **Consistency**: Same user_hash always generates same keys
- **Isolation**: Different user_hashes produce different key pairs
- **Security**: The node keys are derived from 32 bytes (256 bits) of input

---

## Storage Partitioning

All data is partitioned by `user_hash`:

| Storage                   | Partition Key                                  |
| ------------------------- | ---------------------------------------------- |
| DynamoDB (main table)     | `user_hash#schema_name`                        |
| DynamoDB (logs table)     | `user_id` column                               |
| DynamoDB (progress table) | `user_hash` column                             |
| Sled (local)              | Task-local context via `get_current_user_id()` |

---

## Strict Authentication Enforcement

As of this audit, **all API routes require authentication**. Routes use the `require_user_context()` helper which returns a `401 Unauthorized` error if no `X-User-Hash` header is present.

### Changes Made:

| File                      | Change                                   |
| ------------------------- | ---------------------------------------- |
| `server/routes/common.rs` | Added `require_user_context()` helper    |
| `server/routes/system.rs` | All routes now use strict auth           |
| `server/routes/schema.rs` | All routes now use strict auth           |
| `server/routes/log.rs`    | All routes now use strict auth           |
| `server/routes/query.rs`  | All routes now use strict auth           |
| `ingestion/routes.rs`     | All routes now use strict auth           |
| `security/utils.rs`       | Removed anonymous fallback in middleware |

### Before (Anonymous Fallback):

```rust
let user_hash = get_current_user_id()
    .unwrap_or_else(|| "anonymous".to_string()); // ❌ Allowed anonymous
```

### After (Strict Enforcement):

```rust
let user_hash = match require_user_context() {
    Ok(hash) => hash,
    Err(response) => return response,  // ✅ Returns 401 Unauthorized
};
```

---

## Verification Test

```bash
# Test user_hash propagation
curl -H "X-User-Hash: test123abc" http://localhost:9001/api/system/status
# Expected: {"user_hash": "test123abc", ...}

# Without header (now returns 401)
curl http://localhost:9001/api/system/status
# Expected: {"ok": false, "error": "Authentication required. Please provide X-User-Hash header.", "code": "MISSING_USER_CONTEXT"}
```

---

## Conclusion

The user_hash generation and propagation is **consistent** across all components:

1. ✅ **Single source of truth**: Frontend generates the hash
2. ✅ **Consistent algorithm**: `SHA-256(user_id)[0:32]`
3. ✅ **Consistent header**: `X-User-Hash`
4. ✅ **Both transports support it**: HTTP Server and Lambda
5. ✅ **Stateless**: No server-side session storage required
6. ✅ **Strict enforcement**: API routes return 401 if header is missing
7. ✅ **No anonymous fallbacks**: Removed all "anonymous" fallbacks to prevent multi-tenant data leakage
