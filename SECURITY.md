# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to the maintainers or through GitHub's private vulnerability reporting feature if available.

When reporting, please include:

1. Description of the vulnerability
2. Steps to reproduce the issue
3. Potential impact
4. Any suggested fixes (optional)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours.
- **Assessment**: We will investigate and assess the vulnerability within 7 days.
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days.
- **Disclosure**: We will coordinate with you on public disclosure timing.

### Security Best Practices for Users

1. **API Keys**: Never commit API keys (e.g., `FOLD_OPENROUTER_API_KEY`) to version control
2. **AWS Credentials**: Use IAM roles or environment variables, never hardcode credentials
3. **Network**: Use TLS in production environments
4. **Updates**: Keep your DataFold installation up to date

## Security Features

DataFold includes several security features:

- **Ed25519 Signatures**: Cryptographic signing for data integrity
- **AES-GCM Encryption**: Optional encryption at rest
- **User Isolation**: Multi-tenant isolation via user hash partitioning
- **Permission System**: Fine-grained access control for schemas and fields

## Scope

This security policy covers:

- The DataFold core library (`fold_db`)
- The HTTP server (`folddb_server`)
- The schema service (`schema_service`)

Third-party dependencies are outside the scope of this policy, but we monitor them for known vulnerabilities.
