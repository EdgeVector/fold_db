# Design: End-to-End Encryption for FoldDB

> **Status**: Draft — Feb 2026

---

## 1. Goal

Encrypt two things:

1. **Atom content** — the user's data stored in each atom
2. **Index keywords** — the search terms in the native index

The encryption key is derived from the user's **passkey** so the same key is available on any device where the passkey syncs (iCloud Keychain, Google Password Manager, 1Password).

Everything else (schema definitions, storage keys, atom UUIDs, field names, timestamps) stays plaintext. This keeps the system simple and avoids touching the storage layer.

---

## 2. Key Derivation from Passkey

### How Passkey PRF Works

The [WebAuthn PRF extension](https://w3c.github.io/webauthn/#prf-extension) lets you derive a deterministic secret during any passkey authentication ceremony. The same passkey + the same salt always produces the same secret, on any device.

```
User authenticates with passkey
        │
        ▼  PRF extension (salt = "fold:e2e:v1")
        │
        Deterministic 32-byte secret
        │
        ▼  HKDF-SHA256(secret, info="fold:content-key")
        │
    Encryption Key (32 bytes, AES-256-GCM)
        │
        ▼  HKDF-SHA256(secret, info="fold:index-key")
        │
    Index Key (32 bytes, HMAC-SHA256)
```

Two keys derived from one passkey secret:
- **Encryption Key** — AES-256-GCM for atom content
- **Index Key** — HMAC-SHA256 for blind index tokens

### Where the Key Lives

| Context | Key Source | Lifetime |
|:--------|:----------|:---------|
| Browser (exemem.com) | Passkey PRF during login | In-memory until tab closes |
| Native app (Exemem Client) | Passkey PRF during login | In-memory until app closes |
| Local CLI (fold_db) | Passkey PRF or file-based key | `~/.fold_db/e2e.key` for headless use |

The key is never sent to the server. The server never sees plaintext content.

### Browser Support

PRF is supported in Chrome 116+, Safari 18+, Edge 116+. Passkeys sync across devices via the platform's credential manager.

---

## 3. Encrypted Atom Content

### What Changes

The `Atom.content` field (`serde_json::Value`) holds the user's data. Currently stored as plaintext JSON. With E2E, it's encrypted before storage and decrypted after retrieval.

### Write Path (Mutation)

```
1. Client receives mutation with plaintext fields_and_values
2. Serialize content to JSON bytes
3. Encrypt: AES-256-GCM(encryption_key, nonce, json_bytes) → ciphertext
4. Store atom with content = base64(ciphertext) instead of JSON
5. Everything else (uuid, schema_name, timestamps) stays plaintext
```

### Read Path (Query)

```
1. Retrieve atom from storage
2. content field contains base64(ciphertext)
3. Decrypt: AES-256-GCM(encryption_key, ciphertext) → json_bytes
4. Deserialize JSON → Value
5. Return plaintext to caller
```

### Where Encryption Happens

Encryption sits in the mutation/query handlers — not in the storage layer. The existing `EncryptingKvStore` is unrelated and can be removed later. This is application-level encryption of a single field, not storage-level encryption of all values.

```
                    Mutation Handler
                         │
              ┌──────────▼──────────┐
              │  encrypt(content)    │  ← NEW: one line
              └──────────┬──────────┘
                         │
                    FoldDB Core
                         │
                    Storage (Sled/DynamoDB)
                    (stores opaque ciphertext, doesn't care)
```

### Implementation

In `src/fold_db_core/mutation_manager.rs`, before creating the atom:

```rust
// Encrypt content before creating atom
let encrypted_content = e2e.encrypt_content(&mutation.fields_and_values)?;
let atom = Atom::new(schema_name, pub_key, encrypted_content);
```

In the query path, after retrieving atoms:

```rust
// Decrypt content after retrieving atom
let plaintext_content = e2e.decrypt_content(atom.content())?;
```

### Atom UUID

The atom UUID (`src/atom/atom_def.rs:47`) is currently `SHA256(schema + content)`. Since content is now encrypted, and ciphertext is non-deterministic (random nonce), the UUID must be computed from **plaintext** content before encryption:

```rust
// Compute UUID from plaintext, then encrypt content
let uuid = Atom::generate_content_uuid(&schema_name, &plaintext_content);
let encrypted = e2e.encrypt_content(&plaintext_content)?;
let atom = Atom::with_uuid(uuid, schema_name, pub_key, encrypted);
```

This preserves content-based deduplication.

---

## 4. Encrypted Index Keywords

### Current Index Format

From `src/db_operations/native_index/types.rs`:

```
Storage key:  idx:word:{keyword}:{schema}:{field}:{key_hash}
Example:      idx:word:developer:contacts:role:abc123
```

The keyword (`developer`) is plaintext and visible in storage.

### Encrypted Index Format

Replace the plaintext keyword with an HMAC token. Schema, field, and key_hash stay plaintext.

```
Storage key:  idx:word:{HMAC(index_key, keyword)}:{schema}:{field}:{key_hash}
Example:      idx:word:Ht4xK2mN9pQrS1w=:contacts:role:abc123
```

HMAC-SHA256 is deterministic — the same keyword always produces the same token. This means exact-match search still works.

### Write Path (Indexing)

In `src/db_operations/native_index/indexing.rs`:

```rust
// Before:
let term = format!("word:{}", keyword);
let storage_key = entry.storage_key(&term);

// After:
let blind_token = e2e.blind_token(&keyword);
let term = format!("word:{}", blind_token);
let storage_key = entry.storage_key(&term);
```

### Read Path (Search)

In `src/db_operations/native_index/search.rs`:

```rust
// Before:
let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, normalized);

// After:
let blind_token = e2e.blind_token(&normalized);
let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, blind_token);
```

The prefix scan finds the same entries because the same keyword produces the same token at index time and search time.

### Field Name Indexing

Same pattern for `batch_index_field_names`:

```rust
// Before:
let term = format!("field:{}", normalized);

// After:
let blind_token = e2e.blind_token(&normalized);
let term = format!("field:{}", blind_token);
```

### Multi-Word Search

Works unchanged. Each word gets its own blind token. The intersection logic in `search()` operates on `IndexEntry` structs (which contain plaintext schema/field/key refs) — it doesn't depend on the storage key format.

---

## 5. New Code

### `src/crypto/e2e.rs`

```rust
use aes_gcm::{aead::{Aead, KeyInit, OsRng}, Aes256Gcm, Nonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64URL, Engine};

/// Holds the two derived keys for E2E encryption.
pub struct E2eContext {
    /// AES-256-GCM key for atom content
    encryption_key: [u8; 32],
    /// HMAC-SHA256 key for blind index tokens
    index_key: [u8; 32],
}

impl E2eContext {
    /// Derive keys from a passkey PRF secret.
    pub fn from_passkey_secret(secret: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(Some(b"fold:e2e:v1"), secret);

        let mut encryption_key = [0u8; 32];
        hk.expand(b"fold:content-key", &mut encryption_key).unwrap();

        let mut index_key = [0u8; 32];
        hk.expand(b"fold:index-key", &mut index_key).unwrap();

        Self { encryption_key, index_key }
    }

    /// Encrypt atom content (JSON Value → ciphertext bytes).
    pub fn encrypt_content(&self, content: &serde_json::Value) -> Result<Vec<u8>, E2eError> {
        let plaintext = serde_json::to_vec(content)?;
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())?;

        // Format: [nonce:12][ciphertext+tag:variable]
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt atom content (ciphertext bytes → JSON Value).
    pub fn decrypt_content(&self, encrypted: &[u8]) -> Result<serde_json::Value, E2eError> {
        if encrypted.len() < 12 {
            return Err(E2eError::InvalidCiphertext);
        }
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext)?;
        Ok(serde_json::from_slice(&plaintext)?)
    }

    /// Generate a deterministic blind token for a search term.
    /// Same term + same key = same token, every time, on every device.
    pub fn blind_token(&self, term: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.index_key).unwrap();
        mac.update(term.as_bytes());
        B64URL.encode(&mac.finalize().into_bytes()[..16])
    }
}
```

### Browser Side (Web Crypto API)

For the Exemem dashboard, the same operations in JavaScript:

```javascript
// During passkey login, request PRF extension:
const assertion = await navigator.credentials.get({
  publicKey: {
    ...options,
    extensions: {
      prf: { eval: { first: new TextEncoder().encode("fold:e2e:v1") } }
    }
  }
});

// Extract the PRF secret:
const prfResult = assertion.getClientExtensionResults().prf;
const secret = new Uint8Array(prfResult.results.first);

// Derive keys using Web Crypto:
const baseKey = await crypto.subtle.importKey("raw", secret, "HKDF", false, ["deriveKey"]);

const encryptionKey = await crypto.subtle.deriveKey(
  { name: "HKDF", hash: "SHA-256", salt: enc("fold:e2e:v1"), info: enc("fold:content-key") },
  baseKey, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]
);

const indexKey = await crypto.subtle.deriveKey(
  { name: "HKDF", hash: "SHA-256", salt: enc("fold:e2e:v1"), info: enc("fold:index-key") },
  baseKey, { name: "AES-GCM", length: 256 }, true, ["sign"]
  // Export indexKey to use with HMAC
);
```

Both Rust and JavaScript use the same HKDF parameters, so the same passkey produces the same keys on both platforms.

---

## 6. Files Changed

| File | Change |
|:-----|:-------|
| `src/crypto/mod.rs` | Add `pub mod e2e;` |
| `src/crypto/e2e.rs` | **NEW** — `E2eContext` with `encrypt_content`, `decrypt_content`, `blind_token` |
| `src/fold_node/node.rs` | Add `e2e: Arc<E2eContext>` to `FoldNode` |
| `src/fold_db_core/mutation_manager.rs` | Encrypt `fields_and_values` before creating atom |
| `src/fold_db_core/fold_db.rs` | Decrypt atom content after retrieval in query path |
| `src/db_operations/native_index/indexing.rs` | Use `e2e.blind_token()` when writing index entries |
| `src/db_operations/native_index/search.rs` | Use `e2e.blind_token()` when building search prefix |
| `Cargo.toml` | Add `hkdf`, `hmac` dependencies |

---

## 7. What's NOT Changed

- Storage keys (`atom:{uuid}`, `ref:{mol_uuid}`) — stay plaintext
- Atom UUID generation — still `SHA256(schema + content)`, computed from plaintext before encryption
- Schema names, field names in index keys — stay plaintext (structural metadata)
- `EncryptingKvStore` / `EncryptingNamespacedStore` — not involved, can be removed in a future cleanup
- `KmsCryptoProvider` / `LocalCryptoProvider` — not involved
- Index entry values (`IndexEntry` JSON) — stay plaintext (contain schema refs, not user content)
- Storage layer (`KvStore` trait, Sled, DynamoDB) — no changes

---

## 8. Limitations

| Limitation | Why |
|:-----------|:----|
| Exact keyword match only | HMAC is deterministic but not order-preserving. No substring or fuzzy search on encrypted tokens. |
| LLM queries decrypt in memory | AI-powered queries need plaintext. Content is decrypted transiently for the LLM call, then discarded. |
| Passkey PRF required | Older browsers without PRF support cannot derive the key. Fallback: manual key file. |
| No key rotation without re-index | Changing the passkey (and thus the PRF secret) invalidates all blind tokens. Full index rebuild required. |

---

## 9. Data Flow Summary

### Mutation (Write)

```
Client has plaintext content
    │
    ├─ Compute atom UUID from plaintext: SHA256(schema + content)
    ├─ Encrypt content: AES-256-GCM(encryption_key, content) → ciphertext
    ├─ Extract keywords from plaintext (LLM or tokenizer)
    ├─ Blind each keyword: HMAC(index_key, keyword) → token
    │
    ▼
Store:
    atom:{uuid} → { uuid, schema, ciphertext, ... }
    idx:word:{token}:{schema}:{field}:{key_hash} → IndexEntry
```

### Query (Read)

```
Retrieve atom from storage
    │
    ├─ atom.content = ciphertext
    ├─ Decrypt: AES-256-GCM(encryption_key, ciphertext) → plaintext
    │
    ▼
Return plaintext content to caller
```

### Search

```
User searches for "developer"
    │
    ├─ Blind: HMAC(index_key, "developer") → token
    ├─ Prefix scan: idx:word:{token}:
    ├─ Get matching IndexEntry structs (schema, field, key refs)
    ├─ Retrieve atoms by key
    ├─ Decrypt atom content
    │
    ▼
Return plaintext results
```
