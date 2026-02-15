# Simplest Client-Side Login Implementation

## Overview

This document describes the simple, client-side only authentication system for FoldDB that aligns with the stateless Lambda implementation in Exemem.

## Architecture

### Before (Stateful HTTP Server)

```
┌──────────────────┐     ┌─────────────────────────┐
│  HTTP Server     │     │  FoldNode               │
│  (Startup)       │────▶│  (Single Identity)      │
│  - user_id: X    │     │  - keys from X          │
└──────────────────┘     └─────────────────────────┘
         │
         ▼
┌──────────────────┐
│  All Requests    │
│  (No Auth Check) │
└──────────────────┘
```

### After (Stateless HTTP Server - Matches Lambda)

```
┌────────────────────┐    ┌─────────────────────────┐
│  Client (Browser)  │    │  HTTP Server            │
│                    │───▶│  (Stateless)            │
│  Login:            │    │                         │
│  1. user enters ID │    │  Per-Request:           │
│  2. hash = SHA256  │    │  1. Extract X-User-Hash │
│  3. store locally  │    │  2. Set task-local ctx  │
│  4. send header    │    │  3. Isolate user data   │
└────────────────────┘    └─────────────────────────┘
```

## How It Works

### 1. Client-Side Login (UI)

When a user enters their identifier on the login page:

```typescript
// In authSlice.ts - loginUser thunk
const encoder = new TextEncoder();
const data = encoder.encode(userId);
const hashBuffer = await crypto.subtle.digest("SHA-256", data);
const hashArray = Array.from(new Uint8Array(hashBuffer));
const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
const userHash = hashHex.substring(0, 32); // First 32 hex chars

// Stored in localStorage
localStorage.setItem("fold_user_id", userId);
localStorage.setItem("fold_user_hash", userHash);
```

### 2. Header Propagation (API Client)

Every API request includes the user hash:

```typescript
// In api/core/client.ts - performRequest
if (typeof window !== "undefined") {
  const userHash =
    localStorage.getItem("fold_user_hash") ||
    localStorage.getItem("exemem_user_hash");
  if (userHash) {
    headers["x-user-hash"] = userHash; // Primary (Lambda)
    headers["x-user-id"] = userHash; // Fallback (Legacy)
  }
}
```

### 3. Server-Side Extraction (Rust Middleware)

The middleware extracts the user hash and sets task-local context:

```rust
// In server/middleware/auth.rs
let user_id = req
    .headers()
    .get("x-user-hash")
    .or_else(|| req.headers().get("x-user-id"))
    .and_then(|v| v.to_str().ok())
    .map(|s| s.to_string());

if let Some(uid) = user_id {
    run_with_user(&uid, async move { svc.call(req).await }).await
}
```

### 4. Data Isolation

All downstream operations use `get_current_user_id()` to:

- Partition data in storage
- Scope logs to the user
- Isolate ingestion progress

## Comparison with Lambda

| Aspect          | Lambda (Exemem)             | HTTP Server (FoldDB)       |
| --------------- | --------------------------- | -------------------------- |
| Identity Source | X-User-Hash header          | X-User-Hash header         |
| Startup Auth    | None required               | None required              |
| User Isolation  | Per-request via NodeManager | Per-request via task-local |
| Hash Algorithm  | SHA256[0:32] (client-side)  | SHA256[0:32] (client-side) |

## Key Files Changed

1. **`src/bin/folddb_server.rs`** - Removed startup user_id requirement
2. **`src/server/middleware/auth.rs`** - Added support for x-user-hash header

## Key Files (Unchanged, Already Correct)

1. **`src/server/static-react/src/store/authSlice.ts`** - loginUser generates hash
2. **`src/server/static-react/src/api/core/client.ts`** - Sends headers
3. **`src/server/static-react/src/components/LoginPage.jsx`** - UI for login

## Running the Server

```bash
# No user_id needed at startup anymore!
cargo run --bin folddb_server -- --port 9001

# Optional: with schema service
cargo run --bin folddb_server -- --port 9001 --schema-service-url http://localhost:9002
```

## Security Notes

1. **Client-Side Only**: The user_hash is generated entirely on the client. There is no server-side validation of the identity.
2. **No Passwords**: This is a pseudonymous identity system - users provide any identifier, and it's hashed.
3. **Deterministic**: The same identifier always produces the same hash, allowing session persistence.
4. **Privacy**: The server never sees the original identifier, only the hash.

## Migration from Previous Version

If you were running the old version that required `--user-id`:

1. Stop the old server
2. Update to the new version
3. Start without `--user-id`: `cargo run --bin folddb_server`
4. Users log in via the UI (their data is already partitioned by hash)
