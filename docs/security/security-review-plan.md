# Security Review Plan

**Scope:** Focus on external threats; system is internet-facing. Review all aspects: cryptography, authentication, API exposure, and auditability.

---

## 1. Cryptography

### 1.1. Key Management
- Review Ed25519 key generation, storage, and rotation in [`src/security/keys.rs`](../src/security/keys.rs).
- Assess how master keys for AES-GCM are generated, stored, and protected ([`src/security/encryption.rs`](../src/security/encryption.rs)).
- Check for secure key derivation and salt usage.

### 1.2. Encryption
- Ensure AES-GCM is used correctly for data at rest.
- Validate that sensitive data is encrypted before storage and decrypted only when necessary.
- Check for proper error handling and validation of encrypted data.

### 1.3. Signing
- Review message signing and verification logic ([`src/security/signing.rs`](../src/security/signing.rs)).
- Ensure signatures are required for sensitive operations and that timestamp validation is enforced to prevent replay attacks.

---

## 2. Authentication & Authorization

### 2.1. Authentication
- Confirm that all sensitive endpoints require cryptographic authentication (e.g., signed requests).
- Review the use of `SecurityManager` and `verify_signed_request` in [`src/datafold_node/security_routes.rs`](../src/datafold_node/security_routes.rs).
- Check for proper error handling on failed authentication.

### 2.2. Authorization
- Review permission checks at the API and data access layers:
  - Route-level: Ensure protected endpoints use permission checks.
  - Data-level: Ensure `PermissionManager` and `PermissionWrapper` enforce policies for read/write and field-level access ([`src/permissions/`](../src/permissions/)).
- Validate that schema and node trust management cannot be bypassed.

---

## 3. API Exposure

### 3.1. Endpoint Review
- Enumerate all public API endpoints ([`src/datafold_node/http_server.rs`](../src/datafold_node/http_server.rs)).
- Identify which endpoints are public, protected, or admin-only.
- Check for proper use of CORS and HTTP method restrictions.

### 3.2. Input Validation
- Review input validation for all endpoints, especially those handling user-supplied data (e.g., key registration, schema creation, ingestion).
- Check for deserialization vulnerabilities and ensure strict schema validation.

### 3.3. Error Handling
- Ensure sensitive information is not leaked in error messages or logs.

---

## 4. Auditability

### 4.1. Audit Logging
- Review audit logging of security events ([`src/security/audit.rs`](../src/security/audit.rs)).
- Ensure all authentication, authorization, and key management events are logged with sufficient detail.
- Check for log integrity and protection against tampering.

### 4.2. Monitoring & Metrics
- Assess the use of security metrics and performance stats for anomaly detection.

---

## 5. Testing & Coverage

- Review security-related tests ([`src/security/`], [`tests/integration/security_api_tests.rs`](../tests/integration/security_api_tests.rs)).
- Identify gaps in test coverage for authentication, authorization, and cryptographic operations.

---

## 6. Reporting & Recommendations

- Summarize findings for each area above.
- Provide actionable recommendations for any identified weaknesses or best practice gaps.

---

## Security Review Scope Diagram

```mermaid
flowchart TD
    A[Cryptography]
    B[Authentication & Authorization]
    C[API Exposure]
    D[Auditability]
    E[Testing & Coverage]
    F[Reporting & Recommendations]

    A --> A1(Key Management)
    A --> A2(Encryption)
    A --> A3(Signing)

    B --> B1(Authentication)
    B --> B2(Authorization)

    C --> C1(Endpoint Review)
    C --> C2(Input Validation)
    C --> C3(Error Handling)

    D --> D1(Audit Logging)
    D --> D2(Monitoring & Metrics)

    E --> F
    D --> F
    C --> F
    B --> F
    A --> F