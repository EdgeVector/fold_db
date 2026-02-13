# Proposal: Exemem Client — fold_db with Cloud Storage and E2E Encryption

> **Status**: Draft — Feb 2026  
> **Relates to**: [GOAL.md](./GOAL.md), [STRATEGY.md](./STRATEGY.md)

---

## 1. Executive Summary

**Exemem Client** is fold_db's current process — duplicated, cloud-only, with end-to-end encryption. The HTTP server wraps requests in LLM calls exactly as fold_db does today. The LLM calls Exemem APIs to fetch or persist data (Query, Mutation, Index). Storage is routed through **Exemem's cloud API** (mutation/query endpoints) rather than hitting DynamoDB directly. Atoms are **encrypted before being written** and **decrypted when fetched**, enabling true end-to-end encryption.

This applies to **all data surfaces**: DynamoDB atoms AND S3 file uploads.

fold_db remains the open-source product with local storage. Exemem Client is the commercial product: fold_db minus local storage, with cloud API-backed persistence and E2E encryption.

---

## 2. Product Boundary

|                  | **fold_db** (Open Source)            | **Exemem Client** (Commercial)                                 |
| :--------------- | :----------------------------------- | :------------------------------------------------------------- |
| **Storage**      | Sled (local) + DynamoDB (direct)     | **Exemem Cloud API** (mutation/query endpoints → DynamoDB)     |
| **File Uploads** | Local filesystem                     | **Encrypted before upload to S3**                              |
| **LLM / AI**     | LLM query, ingestion, chat, analyze  | Same — inherited from fold_db                                  |
| **HTTP Server**  | Actix-web, same handler layer        | Same — duplicated from fold_db                                 |
| **Encryption**   | Server-side KMS (encryption at rest) | **Client-side E2E** (all data encrypted before leaving device) |
| **Dashboard**    | Local React UI                       | **Web dashboard with E2E encryption** (via Web Crypto API)     |
| **Schemas**      | Global schema registry               | Same                                                           |
| **Ingestion**    | file_to_json + LLM pipeline          | Same                                                           |
| **License**      | Open source                          | Proprietary                                                    |

> [!IMPORTANT]
> Exemem Client is **not a reimagining** — it's the same fold_db process where the storage backend calls Exemem's cloud API instead of local Sled, with E2E encryption layered on top.

---

## 3. Architecture

### 3.1 How It Fits Together

```
┌───────────────────────────────────────────────────────────┐
│                  Exemem Client (Local App)                 │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  fold_db Process (same as today)                    │  │
│  │  • HTTP Server (Actix-web)                          │  │
│  │  • LLM pipeline (query, chat, analyze, ingest)      │  │
│  │  • Handler layer                                    │  │
│  └────────────────────────┬────────────────────────────┘  │
│                           │                                │
│  ┌────────────────────────▼────────────────────────────┐  │
│  │  Cloud Storage Backend                              │  │
│  │  (implements fold_db storage traits)                 │  │
│  │                                                     │  │
│  │  Instead of Sled or direct DynamoDB:                 │  │
│  │  Routes through Exemem Storage API                  │  │
│  │   → POST /storage/put         (write encrypted KV)  │  │
│  │   → POST /storage/get         (read encrypted KV)   │  │
│  │   → POST /storage/scan-prefix (search/list)         │  │
│  │   → POST /storage/batch-put   (bulk writes)         │  │
│  │   → POST /storage/upload      (encrypted S3 files)  │  │
│  └────────────────────────┬────────────────────────────┘  │
└───────────────────────────┼────────────────────────────────┘
                            │ HTTPS
                            ▼
              ┌──────────────────────────┐
              │   Exemem Cloud API       │
              │   (API Gateway + Lambda) │
              │                          │
              │   → DynamoDB (opaque     │
              │     encrypted atoms)     │
              │   → S3 (opaque           │
              │     encrypted files)     │
              └──────────────────────────┘
```

### 3.2 New Storage Abstraction: `ExememApiStore`

Rather than intercepting the handler layer, Exemem Client introduces a **new storage backend** — `ExememApiStore` — that implements fold_db's existing storage traits (`KvStore`, `NamespacedStore`). Under the hood, it calls **new Exemem Storage API endpoints** — a different set of APIs from the existing handler endpoints.

From fold_db's perspective, it's just another storage backend:

| Backend                    | Implements                   | Backing Store                 |
| :------------------------- | :--------------------------- | :---------------------------- |
| **Sled**                   | `KvStore`, `NamespacedStore` | Local embedded DB             |
| **DynamoDB**               | `KvStore`, `NamespacedStore` | AWS DynamoDB (direct)         |
| **InMemory**               | `KvStore`, `NamespacedStore` | HashMap (testing)             |
| **ExememApiStore** _(new)_ | `KvStore`, `NamespacedStore` | **Exemem Storage API (REST)** |

### 3.3 Exemem Storage API (New Endpoints)

These are **storage-level endpoints** — distinct from the existing handler endpoints (`/query`, `/mutation`, etc.). They map 1:1 to fold_db's `KvStore` and `NamespacedStore` traits.

> [!WARNING]
> These are NOT the existing handler endpoints. The handler endpoints (e.g., `POST /query`, `POST /mutation`) are high-level business logic. The Storage API is low-level key-value operations that the `ExememApiStore` calls under the hood.

#### KvStore Trait → Storage API

| `KvStore` Method      | Exemem Storage API Endpoint            | Description                          |
| :-------------------- | :------------------------------------- | :----------------------------------- |
| `get(key)`            | `POST /storage/get`                    | Get a value by key                   |
| `put(key, value)`     | `POST /storage/put`                    | Put a key-value pair                 |
| `delete(key)`         | `POST /storage/delete`                 | Delete a key                         |
| `exists(key)`         | `POST /storage/exists`                 | Check if a key exists                |
| `scan_prefix(prefix)` | `POST /storage/scan-prefix`            | Scan all keys with a given prefix    |
| `batch_put(items)`    | `POST /storage/batch-put`              | Batch write multiple key-value pairs |
| `batch_delete(keys)`  | `POST /storage/batch-delete`           | Batch delete multiple keys           |
| `flush()`             | No-op (cloud is eventually consistent) | —                                    |

#### NamespacedStore Trait → Storage API

| `NamespacedStore` Method | Exemem Storage API Endpoint        | Description                         |
| :----------------------- | :--------------------------------- | :---------------------------------- |
| `open_namespace(name)`   | `POST /storage/namespace/open`     | Open or create a namespace          |
| `list_namespaces()`      | `GET /storage/namespaces`          | List all namespaces                 |
| `delete_namespace(name)` | `DELETE /storage/namespace/{name}` | Delete a namespace and all its data |

#### Request Format

All Storage API requests include `namespace` (which DynamoDB table/namespace) and operate on opaque `Vec<u8>` key-value pairs — the server never interprets the content:

```json
// POST /storage/put
{
  "namespace": "main",
  "key": "base64-encoded-key",
  "value": "base64-encoded-encrypted-ciphertext"
}

// POST /storage/scan-prefix
{
  "namespace": "main",
  "prefix": "base64-encoded-prefix"
}

// POST /storage/batch-put
{
  "namespace": "main",
  "items": [
    { "key": "base64-key-1", "value": "base64-encrypted-value-1" },
    { "key": "base64-key-2", "value": "base64-encrypted-value-2" }
  ]
}
```

#### Implementation in ExememApiStore

```rust
#[async_trait]
impl KvStore for ExememApiStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // POST /storage/get { namespace, key } → Exemem Storage API
    }
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        // POST /storage/put { namespace, key, value } → Exemem Storage API
    }
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // POST /storage/scan-prefix { namespace, prefix } → Exemem Storage API
    }
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        // POST /storage/batch-put { namespace, items } → Exemem Storage API
    }
    // ... delete, exists, batch_delete follow the same pattern
}
```

This means the entire fold_db engine — queries, mutations, indexing, schemas — runs unchanged on the client. Only the storage layer is swapped.

> [!NOTE]
> Encryption happens **before** data reaches `ExememApiStore`. The store sends and receives opaque ciphertext. It never interprets the content — just passes base64-encoded bytes to the Exemem Storage API.

---

## 4. End-to-End Encryption

### 4.1 What Gets Encrypted

| Data Surface        | Current State                                   | Proposed                                              |
| :------------------ | :---------------------------------------------- | :---------------------------------------------------- |
| **DynamoDB atoms**  | Server-side KMS encryption (server can decrypt) | **Client-side E2E** (server stores opaque ciphertext) |
| **S3 file uploads** | **Not encrypted**                               | **Client-side encrypted before upload**               |
| **Schema metadata** | Plaintext                                       | Remains plaintext (schemas are public by design)      |

> [!WARNING]
> **S3 files are currently stored unencrypted.** This proposal adds mandatory client-side encryption for all file uploads.

### 4.2 Current Model (Server-Side)

```
Client → HTTP Server → fold_db → Encrypt (KMS DEK) → DynamoDB
Client → HTTP Server → S3 (plaintext!)
                        Server sees all data ↑
```

### 4.3 Proposed Model (Client-Side E2E)

```
Client → Encrypt (client DEK) → Exemem Client → Cloud API → DynamoDB (opaque)
Client → Encrypt (client DEK) → Exemem Client → Cloud API → S3 (opaque)
                                  Server never sees plaintext ↑
```

---

## 5. Key Management: Passkey PRF

### 5.1 Cross-Device Key Derivation

The encryption key is derived from the user's **passkey** using the **WebAuthn PRF extension**. Because passkeys sync across devices (iCloud Keychain, Google Password Manager, 1Password), the DEK is available wherever the user authenticates.

```
Passkey (syncs across devices)
    │
    ▼ PRF Extension (during authentication)
    │
    Deterministic Secret (same passkey + salt = same secret)
    │
    ▼ HKDF
    │
    Wrapping Key
    │
    ▼ Unwrap
    │
    Data Encryption Key (DEK)
    • AES-256-GCM
    • Stored encrypted on server (only passkey holder can unwrap)
    • Same DEK from any device with the passkey
```

### 5.2 How It Works

1. **First time**: Generate random DEK → encrypt with PRF-derived wrapping key → store encrypted DEK on Exemem Cloud
2. **Any device**: Authenticate with passkey → PRF gives wrapping key → decrypt stored DEK → use for atoms and files
3. **Passkey sync**: iCloud Keychain / Google Password Manager syncs the passkey → PRF works on any synced device

### 5.3 Browser Support

PRF is supported in Chrome 116+, Safari 18+ (macOS/iOS), and Edge. Synced passkeys carry the PRF capability across devices.

---

## 6. Dashboard E2E Encryption

The **Exemem website dashboard** (exemem.com, React app) also supports full E2E encryption using the same mechanism:

```
Browser (exemem.com dashboard)
    │
    ├── 1. User logs in with passkey
    ├── 2. PRF extension → wrapping key
    ├── 3. Unwrap stored encrypted DEK
    ├── 4. DEK held in memory (JavaScript)
    │
    ├── On mutation: encrypt atom → send ciphertext to API
    ├── On query:    receive ciphertext → decrypt with DEK
    ├── On file upload: encrypt chunks → upload to S3
    └── On AI query: decrypt locally → send plaintext transiently to LLM
```

- **Web Crypto API** (`crypto.subtle`) provides AES-256-GCM encrypt/decrypt entirely client-side
- **Same DEK** as native client — derived from the same passkey via PRF
- Same trust model as Signal Desktop, ProtonMail — code served by the server must be trusted

---

## 7. Encrypted S3 File Uploads

### 7.1 The Problem

S3 files (uploads via `/ingestion/upload`) are **currently stored unencrypted**. Any server administrator or AWS user with S3 access can read them.

### 7.2 The Solution: Chunked Client-Side Encryption

```
Large File (e.g. 2GB video)
    │
    ├── Chunk 1 (5MB) → AES-256-GCM encrypt (unique nonce) → upload part 1
    ├── Chunk 2 (5MB) → AES-256-GCM encrypt (unique nonce) → upload part 2
    ├── Chunk 3 (5MB) → AES-256-GCM encrypt (unique nonce) → upload part 3
    └── ...

Encrypted manifest (chunk order + nonces) stored alongside.
S3 multipart upload handles the transport.
Server only ever sees encrypted chunks.
```

### 7.3 Atom ↔ S3 Linkage

Each uploaded file has a corresponding **atom with a metadata field** that points to the S3 object. The atom stores the reference and any extracted/structured data; the S3 object stores the encrypted raw file.

```
┌─────────────────────────────────────┐       ┌──────────────────────┐
│  Atom (DynamoDB)                    │       │  S3 Object           │
│                                     │       │                      │
│  fields: { ... extracted data ... } │       │  (encrypted chunks)  │
│  s3_ref: "uploads/{user}/{file_id}" │──────►│                      │
│                                     │       │                      │
│  Both encrypted with same DEK       │       │                      │
└─────────────────────────────────────┘       └──────────────────────┘
```

The `s3_ref` metadata field lets the client retrieve and decrypt the original file from any device. The atom content (structured fields) and the S3 file (raw source) are both encrypted with the same client-side DEK.

### 7.4 Works Everywhere

| Surface               | Method                                                              |
| :-------------------- | :------------------------------------------------------------------ |
| **Native client**     | Streaming file I/O + chunked AES-256-GCM (Rust)                     |
| **Browser dashboard** | `File.slice()` + `crypto.subtle.encrypt()` per chunk                |
| **Download**          | Reverse: download encrypted parts → decrypt each chunk → reassemble |

---

## 8. Data Paths Summary

### 8.1 Write Path (Mutation)

1. Client encrypts atom content with DEK → ciphertext
2. Exemem Client sends encrypted mutation via Cloud API (`POST /mutation`)
3. Cloud API persists opaque ciphertext to DynamoDB

### 8.2 Read Path (Query)

1. Exemem Client calls Cloud API (`POST /query`)
2. Cloud API returns encrypted atoms
3. Client decrypts atoms with DEK → plaintext

### 8.3 File Upload Path

1. Client encrypts file in chunks with DEK
2. Exemem Client uploads encrypted chunks via Cloud API → S3
3. S3 stores opaque encrypted chunks

### 8.4 LLM-Mediated Path (AI Queries)

1. Client decrypts relevant atoms locally
2. Sends decrypted context + question in transient request
3. LLM computes answer, returns mediated response
4. Raw data never persisted server-side

### 8.5 Search Path (Native Index)

1. Client computes `HMAC-SHA256(DEK, search_term)` → deterministic encrypted key
2. ExememApiStore looks up `feature:encrypted_term` via Storage API
3. Server returns index entries (schema references — not encrypted, don't contain user data)
4. Client uses schema references to retrieve and decrypt the actual atoms

> [!NOTE]
> Because the encryption is deterministic (HMAC), the same search term always produces the same encrypted key. The server can do exact-match lookups without ever seeing the plaintext term.

---

## 9. Migration Path

### Phase 1: Duplicate fold_db into Exemem Client

- Copy fold_db's HTTP server, handlers, AI code into Exemem Client
- Implement cloud storage backend that routes through Exemem Cloud API (mutation/query endpoints)
- Remove Sled/local storage option
- Verify: works identically to current cloud deployment

### Phase 2: Client-Side Encryption

- DEK generation + passkey PRF wrapping
- Encrypt-before-send for atoms (mutations) and files (S3 uploads)
- Decrypt-after-receive for queries and file downloads
- Remove server-side KMS encryption for new tenants (dual-read for migration)
- Dashboard (web UI) encryption via Web Crypto API

### Phase 3: SDK and Third-Party Access

- Publish Exemem Client SDK (JS/TS)
- Integrate with Passkey-native auth proxy for third-party apps
- Disclosure policy enforcement via LLM mediator

---

## 10. Relationship to GOAL.md

| GOAL.md Concept         | How This Delivers It                                                             |
| :---------------------- | :------------------------------------------------------------------------------- |
| **Data Sovereignty**    | Client-side DEK — only the user can decrypt (atoms AND files)                    |
| **Mediated Disclosure** | fold_db's existing LLM pipeline, inherited as-is                                 |
| **Custodial Hosting**   | Exemem Cloud is the custodian — but E2E means even the custodian can't read data |
| **Self-Hosted Option**  | fold_db (open-source, Sled) is the self-hosted PDN                               |

---

## 11. Encrypted Search

The native index stores search terms as **keys** (`feature:term` format, e.g., `word:hello`). The values just point to schemas — they don't contain user data and don't need encryption.

### 11.1 How It Works

Search terms are encrypted client-side using **deterministic encryption** (HMAC-SHA256 with the DEK as key). The same term always produces the same ciphertext, so exact-match lookups work normally.

```
Plaintext index key:   word:hello
                            │
                   HMAC-SHA256(DEK, "hello")
                            │
                            ▼
Encrypted index key:   word:a3f8b2c1e9d4...  (deterministic — same every time)

Index value:           → schema reference (not encrypted)
```

### 11.2 Write Path (Indexing)

1. fold_db extracts terms from ingested data (e.g., words, emails, dates)
2. Client encrypts each term: `HMAC-SHA256(DEK, term)` → encrypted term
3. Stores `feature:encrypted_term` → schema reference via ExememApiStore
4. Server stores opaque keys it cannot reverse

### 11.3 Read Path (Search)

1. User searches for "hello"
2. Client computes `HMAC-SHA256(DEK, "hello")` → same encrypted term as write time
3. ExememApiStore queries `word:encrypted_term` via Storage API
4. Server returns matching schema references (unencrypted)
5. Client uses schema references to retrieve and decrypt actual atoms

### 11.4 What's Protected

| Component                    | Encrypted?              | Why                                                  |
| :--------------------------- | :---------------------- | :--------------------------------------------------- |
| **Search term (key)**        | ✅ Deterministic (HMAC) | Prevents server from learning what terms exist       |
| **Feature/classification**   | ❌ Plaintext            | Generic categories ("word", "email") — not sensitive |
| **Index value (schema ref)** | ❌ Plaintext            | Just points to a schema — no user data               |
| **Atom content**             | ✅ AES-256-GCM          | Full E2E encryption as described in §4               |

### 11.5 Limitation: Prefix Search

Deterministic encryption supports **exact-match** lookups only. Prefix search (e.g., "hel\*") won't work because `HMAC("hel")` has no relationship to `HMAC("hello")`. Options:

- Client-side filtering after decrypting broader results
- LLM-mediated search for fuzzy/semantic queries

---

## 12. Third-Party App Access

Exemem Client exposes a **local HTTP server** that third-party apps can connect to. The user's device is the decryption boundary — apps never access the cloud API directly.

### 12.1 Architecture

```
┌─────────────────────────────────────────────────────┐
│  User's Device                                       │
│                                                      │
│  ┌───────────────┐     ┌──────────────────────────┐ │
│  │  Third-Party   │     │  Exemem Client            │ │
│  │  App            │────►│                          │ │
│  │                │     │  Local HTTP Server        │ │
│  │  API Key: xyz  │     │   → Validates API key     │ │
│  └───────────────┘     │   → Enforces access policy │ │
│                         │   → Decrypts atoms locally │ │
│  ┌───────────────┐     │   → Returns permitted data │ │
│  │  Another App   │────►│                          │ │
│  │  API Key: abc  │     └──────────┬───────────────┘ │
│  └───────────────┘                │                  │
│                          Exemem Storage API           │
└──────────────────────────┼───────────────────────────┘
                           │ HTTPS (encrypted)
                           ▼
                    Exemem Cloud API
```

### 12.2 App-Specific API Keys

Each third-party app gets its own API key, registered by the user in Exemem Client:

| API Key  | App          | Allowed Schemas                | Denied Schemas                    |
| :------- | :----------- | :----------------------------- | :-------------------------------- |
| `xyz...` | Health App   | `health_record`, `fitness_log` | `financial`, `personal_notes`     |
| `abc...` | Finance App  | `financial`, `tax_record`      | `health_record`, `personal_notes` |
| `def...` | AI Assistant | All                            | None                              |

### 12.3 Access Policy Enforcement

The local HTTP server enforces access policies **before** decrypting and returning data:

1. App sends request with API key to `localhost:{port}`
2. Exemem Client validates the API key
3. Checks which schemas this key has access to
4. If permitted: decrypts atoms locally, returns plaintext response
5. If denied: returns 403 — app never sees the data

### 12.4 Why Local

- **Data never leaves the device unencrypted** except to the app on the same machine
- **No cloud-side access control needed** — the user's device IS the gatekeeper
- **User controls everything** — add/revoke API keys, change policies at any time

### 12.5 Key Recovery

The user is responsible for storing their passkey safely. Passkeys sync via iCloud Keychain, Google Password Manager, or 1Password. If the user loses access to all synced devices and their passkey provider, their data is unrecoverable by design — same trust model as a crypto wallet.

---

## 13. Summary

**Exemem Client = fold_db's current process, cloud API storage, E2E encryption.**

- **Same LLM pipeline** — inherited from fold_db as-is
- **Storage via Exemem Storage API** — new `/storage/*` endpoints, not existing handlers
- **Everything encrypted client-side** — atoms, S3 files, and search term keys
- **Passkey PRF for key management** — DEK migrates across devices with the passkey
- **Dashboard support** — web UI does E2E encryption via Web Crypto API
- **Deterministic encrypted search** — HMAC on search terms, index values stay plaintext
- **fold_db stays open-source** — local Sled storage, full features, no cloud dependency
