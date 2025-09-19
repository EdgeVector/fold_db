# Permissions and Fee System

Fold DB implements a comprehensive permission and fee system that provides fine-grained access control and configurable payment requirements for data operations.

## Table of Contents

1. [Permission System Overview](#permission-system-overview)
2. [Permission Types](#permission-types)
3. [Trust Distance Model](#trust-distance-model)
4. [Fee Configuration](#fee-configuration)
5. [Payment Integration](#payment-integration)
6. [Permission Management](#permission-management)
7. [Fee Calculation](#fee-calculation)
8. [Lightning Network Integration](#lightning-network-integration)
9. [API Operations](#api-operations)
10. [Best Practices](#best-practices)

## Permission System Overview

### Multi-Layered Security

Fold DB implements a multi-layered permission system:

1. **Schema-Level Permissions**: Default permissions for all fields
2. **Field-Level Permissions**: Specific permissions per field
3. **Explicit Permissions**: Runtime permission grants
4. **Trust Distance**: Network-based access control
5. **Payment Requirements**: Economic access barriers

### Permission Hierarchy

```
Explicit Permissions (highest priority)
        ↓
Field-Level Permissions
        ↓
Schema-Level Permissions
        ↓
Default Node Permissions (lowest priority)
```

### Access Control Flow

```
Request → Authentication → Permission Check → Fee Calculation → Access Grant/Deny
```

## Permission Types

### NoRequirement

**Public Access:**
```json
{
  "permission_policy": {
    "read_policy": {"NoRequirement": null},
    "write_policy": {"NoRequirement": null}
  }
}
```

**Use Cases:**
- Public information
- Open data sharing
- Community content
- Marketing data

### Distance-Based Permissions

**Trust Distance Requirements:**
```json
{
  "permission_policy": {
    "read_policy": {"Distance": 1},
    "write_policy": {"Distance": 0}
  }
}
```

**Distance Levels:**
- **Distance 0**: Local node only (highest security)
- **Distance 1**: Direct trusted peers
- **Distance 2**: Peers of trusted peers
- **Distance 3+**: Extended network access

### Public Key Permissions

**Specific Key Requirements:**
```json
{
  "permission_policy": {
    "read_policy": {"PublicKey": "ed25519:ABC123DEF456..."},
    "write_policy": {"PublicKey": "ed25519:XYZ789UVW012..."}
  }
}
```

**Key Management:**
```bash
# Generate key pair
openssl genpkey -algorithm Ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Extract public key for configuration
openssl pkey -in public.pem -pubin -outform DER | base64
```

### Explicit Permissions

**Named Permission Requirements:**
```json
{
  "permission_policy": {
    "read_policy": {"Explicit": "admin_read"},
    "write_policy": {"Explicit": "admin_write"}
  }
}
```

**Permission Registry:**
```json
{
  "explicit_permissions": {
    "admin_read": {
      "description": "Administrative read access",
      "granted_to": [
        "ed25519:AdminKey123...",
        "ed25519:AdminKey456..."
      ]
    },
    "premium_access": {
      "description": "Premium user access",
      "requirements": {
        "payment_verified": true,
        "subscription_active": true
      }
    }
  }
}
```

## Trust Distance Model

### Distance Calculation

**Direct Connections:**
```
Node A ←→ Node B (Distance 1)
Node A ←→ Node C (Distance 1)
```

**Indirect Connections:**
```
Node A ←→ Node B ←→ Node D (Distance 2 from A to D)
```

**Trust Propagation:**
```json
{
  "trust_configuration": {
    "default_distance": 1,
    "max_trust_distance": 3,
    "trust_decay_factor": 0.9,
    "explicit_trust": {
      "12D3KooWDirectPeer...": 0,
      "12D3KooWTrustedPeer...": 1,
      "12D3KooWPartnerPeer...": 2
    }
  }
}
```

### Trust Management

**Set Trust Distance:**
```bash
curl -X POST http://localhost:9001/api/permissions/trust-distance \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL...",
    "trust_distance": 1,
    "reason": "verified_partner",
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

**Trust Verification:**
```json
{
  "trust_verification": {
    "require_signatures": true,
    "verify_certificates": true,
    "trust_chain_validation": true,
    "reputation_threshold": 0.8
  }
}
```

### Dynamic Trust

**Reputation-Based Trust:**
```json
{
  "reputation_system": {
    "enabled": true,
    "factors": {
      "uptime": 0.25,
      "response_accuracy": 0.25,
      "data_consistency": 0.25,
      "community_rating": 0.25
    },
    "trust_adjustment": {
      "high_reputation": -1,
      "medium_reputation": 0,
      "low_reputation": +1
    }
  }
}
```

## Fee Configuration

### Payment Models

**Flat Fee Model:**
```json
{
  "payment_config": {
    "base_multiplier": 100.0,
    "trust_distance_scaling": {"None": null},
    "min_payment": 100
  }
}
```

**Linear Scaling Model:**
```json
{
  "payment_config": {
    "base_multiplier": 50.0,
    "trust_distance_scaling": {
      "Linear": {
        "slope": 25.0,
        "intercept": 1.0,
        "min_factor": 1.0
      }
    },
    "min_payment": 25
  }
}
```

**Formula:** `fee = base_multiplier * (slope * distance + intercept)`

**Exponential Scaling Model:**
```json
{
  "payment_config": {
    "base_multiplier": 10.0,
    "trust_distance_scaling": {
      "Exponential": {
        "base": 2.0,
        "scale": 1.0,
        "min_factor": 1.0
      }
    },
    "min_payment": 10
  }
}
```

**Formula:** `fee = base_multiplier * (base^distance * scale)`

### Schema-Level Fee Configuration

**Default Fee Settings:**
```json
{
  "name": "PremiumData",
  "payment_config": {
    "base_multiplier": 200.0,
    "min_payment_threshold": 500
  },
  "fields": {
    "public_field": {
      "payment_config": {
        "base_multiplier": 0.0
      }
    },
    "premium_field": {
      "payment_config": {
        "base_multiplier": 500.0,
        "min_payment": 1000
      }
    }
  }
}
```

### Dynamic Pricing

**Time-Based Pricing:**
```json
{
  "dynamic_pricing": {
    "enabled": true,
    "time_multipliers": {
      "peak_hours": 1.5,
      "off_peak_hours": 0.8,
      "weekend": 1.2
    },
    "peak_hours": ["09:00-17:00"]
  }
}
```

**Demand-Based Pricing:**
```json
{
  "demand_pricing": {
    "enabled": true,
    "load_multipliers": {
      "low": 0.8,
      "medium": 1.0,
      "high": 1.5,
      "critical": 2.0
    },
    "load_thresholds": {
      "medium": 70,
      "high": 85,
      "critical": 95
    }
  }
}
```

## Payment Integration

### Payment Types

**Lightning Network:**
```json
{
  "payment_methods": {
    "lightning": {
      "enabled": true,
      "node_uri": "03abc123...@localhost:9735",
      "invoice_expiry": 3600,
      "min_payment": 1,
      "max_payment": 1000000
    }
  }
}
```

**On-Chain Bitcoin:**
```json
{
  "payment_methods": {
    "bitcoin": {
      "enabled": true,
      "wallet_type": "electrum",
      "min_confirmations": 1,
      "fee_rate": "medium"
    }
  }
}
```

**Credit System:**
```json
{
  "payment_methods": {
    "credits": {
      "enabled": true,
      "credit_rate": 100,
      "auto_recharge": {
        "enabled": true,
        "threshold": 1000,
        "amount": 10000
      }
    }
  }
}
```

### Payment Workflow

**1. Fee Calculation:**
```bash
curl -X POST http://localhost:9001/api/payments/calculate-fee \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "query",
    "schema": "UserProfile",
    "fields": ["email", "phone"],
    "requester_distance": 2
  }'
```

**Response:**
```json
{
  "total_fee_sats": 250,
  "breakdown": {
    "email": {
      "base_fee": 100,
      "distance_multiplier": 2.0,
      "final_fee": 200
    },
    "phone": {
      "base_fee": 25,
      "distance_multiplier": 2.0,
      "final_fee": 50
    }
  },
  "payment_methods": ["lightning", "credits"]
}
```

**2. Payment Generation:**
```bash
curl -X POST http://localhost:9001/api/payments/lightning/invoice \
  -H "Content-Type: application/json" \
  -d '{
    "amount_sats": 250,
    "description": "Access to UserProfile fields",
    "expiry": 3600,
    "operation_id": "op_123456"
  }'
```

**Response:**
```json
{
  "payment_request": "lnbc2500n1p...",
  "payment_hash": "abc123...",
  "expires_at": "2024-01-15T12:00:00Z",
  "amount_sats": 250
}
```

**3. Payment Verification:**
```bash
curl -X POST http://localhost:9001/api/payments/verify \
  -H "Content-Type: application/json" \
  -d '{
    "payment_hash": "abc123...",
    "operation_id": "op_123456"
  }'
```

**4. Access Grant:**
```json
{
  "access_granted": true,
  "access_token": "access_token_xyz...",
  "expires_at": "2024-01-15T13:00:00Z",
  "permissions": {
    "schema": "UserProfile",
    "fields": ["email", "phone"],
    "operations": ["read"]
  }
}
```

## Permission Management

### Runtime Permission Grants

**Grant Explicit Permission:**
```bash
curl -X POST http://localhost:9001/api/permissions/explicit \
  -H "Content-Type: application/json" \
  -d '{
    "permission_name": "emergency_access",
    "schema": "HealthData",
    "field": "medical_records",
    "permission": "read",
    "public_key": "ed25519:EmergencyKey...",
    "granted_by": "ed25519:AdminKey...",
    "reason": "Medical emergency authorization",
    "expires_at": "2024-01-15T18:00:00Z"
  }'
```

**Revoke Permission:**
```bash
curl -X DELETE http://localhost:9001/api/permissions/explicit/emergency_access
```

### Permission Auditing

**Permission History:**
```bash
curl http://localhost:9001/api/permissions/audit/UserProfile/email
```

**Response:**
```json
{
  "field": "UserProfile.email",
  "access_history": [
    {
      "timestamp": "2024-01-15T10:30:00Z",
      "operation": "read",
      "requester": "ed25519:ClientKey...",
      "permission_used": "Distance(1)",
      "payment_amount": 100,
      "access_granted": true
    },
    {
      "timestamp": "2024-01-15T10:45:00Z",
      "operation": "read",
      "requester": "ed25519:UnknownKey...",
      "permission_used": "Distance(3)",
      "access_granted": false,
      "denial_reason": "Trust distance exceeded"
    }
  ]
}
```

### Bulk Permission Management

**Permission Templates:**
```json
{
  "permission_templates": {
    "admin_template": {
      "schemas": ["*"],
      "permissions": {
        "read_policy": {"Distance": 0},
        "write_policy": {"Distance": 0}
      }
    },
    "readonly_template": {
      "schemas": ["UserProfile", "Analytics"],
      "permissions": {
        "read_policy": {"Distance": 1},
        "write_policy": {"Explicit": "never"}
      }
    }
  }
}
```

**Apply Template:**
```bash
curl -X POST http://localhost:9001/api/permissions/apply-template \
  -H "Content-Type: application/json" \
  -d '{
    "template": "readonly_template",
    "target": "ed25519:ReadOnlyUser...",
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

## Fee Calculation

### Calculation Examples

**Example 1: Simple Linear Scaling**
```json
{
  "field_config": {
    "base_multiplier": 100.0,
    "trust_distance_scaling": {
      "Linear": {
        "slope": 50.0,
        "intercept": 1.0,
        "min_factor": 1.0
      }
    }
  },
  "requester_distance": 2
}
```

**Calculation:**
```
factor = slope * distance + intercept
factor = 50.0 * 2 + 1.0 = 101.0
fee = base_multiplier * factor
fee = 100.0 * 101.0 = 10,100 satoshis
```

**Example 2: Exponential Scaling**
```json
{
  "field_config": {
    "base_multiplier": 10.0,
    "trust_distance_scaling": {
      "Exponential": {
        "base": 2.0,
        "scale": 1.0,
        "min_factor": 1.0
      }
    }
  },
  "requester_distance": 3
}
```

**Calculation:**
```
factor = base^distance * scale
factor = 2.0^3 * 1.0 = 8.0
fee = base_multiplier * factor
fee = 10.0 * 8.0 = 80 satoshis
```

### Fee Optimization

**Batch Operation Discounts:**
```json
{
  "batch_discounts": {
    "enabled": true,
    "thresholds": {
      "5_fields": 0.95,
      "10_fields": 0.9,
      "20_fields": 0.85
    }
  }
}
```

**Volume Discounts:**
```json
{
  "volume_discounts": {
    "enabled": true,
    "monthly_thresholds": {
      "bronze": {"min_sats": 10000, "discount": 0.05},
      "silver": {"min_sats": 50000, "discount": 0.1},
      "gold": {"min_sats": 100000, "discount": 0.15}
    }
  }
}
```

## Lightning Network Integration

### Lightning Configuration

**LND Integration:**
```json
{
  "lightning": {
    "implementation": "lnd",
    "host": "localhost:10009",
    "tls_cert_path": "/lnd/tls.cert",
    "macaroon_path": "/lnd/data/chain/bitcoin/mainnet/admin.macaroon",
    "network": "mainnet"
  }
}
```

**CLN Integration:**
```json
{
  "lightning": {
    "implementation": "cln",
    "socket_path": "/lightning-rpc",
    "network": "bitcoin"
  }
}
```

### Invoice Management

**Generate Invoice:**
```bash
curl -X POST http://localhost:9001/api/payments/lightning/invoice \
  -H "Content-Type: application/json" \
  -d '{
    "amount_sats": 1000,
    "description": "Data access payment",
    "expiry": 3600,
    "metadata": {
      "operation_id": "op_123",
      "schema": "UserProfile",
      "fields": ["email"]
    }
  }'
```

**Check Payment Status:**
```bash
curl http://localhost:9001/api/payments/lightning/status/abc123...
```

**Response:**
```json
{
  "payment_hash": "abc123...",
  "status": "paid",
  "amount_paid_sats": 1000,
  "paid_at": "2024-01-15T11:30:00Z",
  "preimage": "def456...",
  "operation_authorized": true
}
```

### Automatic Payment Processing

**Payment Webhooks:**
```json
{
  "webhooks": {
    "payment_received": {
      "url": "http://localhost:9001/api/payments/webhook",
      "events": ["payment.received", "payment.failed"],
      "retry_attempts": 3
    }
  }
}
```

**Payment Processing:**
```javascript
// Automatic access grant on payment
function processPayment(paymentHash, amount, metadata) {
  const operation = getOperation(metadata.operation_id);
  if (verifyPayment(paymentHash, amount)) {
    grantAccess(operation, metadata.schema, metadata.fields);
    sendAccessToken(operation.requester);
  }
}
```

## API Operations

### Permission Check API

**Check Access:**
```bash
curl -X POST http://localhost:9001/api/permissions/check \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "read",
    "schema": "UserProfile",
    "field": "email",
    "requester": "ed25519:ClientKey...",
    "requester_distance": 2
  }'
```

**Response:**
```json
{
  "access_granted": false,
  "reason": "payment_required",
  "required_payment": {
    "amount_sats": 200,
    "payment_methods": ["lightning", "credits"]
  },
  "alternative_access": {
    "explicit_permission": "premium_user",
    "reduced_distance": 1
  }
}
```

### Fee Calculation API

**Calculate Fees:**
```bash
curl -X POST http://localhost:9001/api/payments/calculate \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [
      {
        "type": "query",
        "schema": "Analytics", 
        "fields": ["conversion_rate", "revenue"]
      },
      {
        "type": "mutation",
        "schema": "UserProfile",
        "operation": "update"
      }
    ],
    "requester_distance": 1
  }'
```

### Batch Permission Management

**Bulk Permission Update:**
```bash
curl -X POST http://localhost:9001/api/permissions/bulk-update \
  -H "Content-Type: application/json" \
  -d '{
    "updates": [
      {
        "schema": "UserProfile",
        "field": "email",
        "permission": {
          "read_policy": {"Distance": 2},
          "write_policy": {"Distance": 1}
        }
      },
      {
        "schema": "Analytics",
        "field": "revenue",
        "payment_config": {
          "base_multiplier": 500.0,
          "min_payment": 1000
        }
      }
    ]
  }'
```

## Best Practices

### Security Best Practices

**1. Principle of Least Privilege:**
```json
{
  "default_permissions": {
    "read_policy": {"Distance": 1},
    "write_policy": {"Distance": 0}
  },
  "sensitive_fields": {
    "read_policy": {"Explicit": "authorized_access"},
    "write_policy": {"Explicit": "admin_access"}
  }
}
```

**2. Regular Permission Audits:**
```bash
# Daily audit script
curl http://localhost:9001/api/permissions/audit/summary | \
  jq '.suspicious_access[] | select(.risk_score > 0.8)'

# Weekly permission review
curl http://localhost:9001/api/permissions/report/weekly
```

**3. Secure Key Management:**
```bash
# Hardware Security Module integration
export FOLD_DB_HSM_PROVIDER=yubihsm
export FOLD_DB_HSM_CONNECTOR=http://localhost:12345

# Key rotation
curl -X POST http://localhost:9001/api/security/rotate-keys
```

### Payment Configuration Best Practices

**1. Fair Pricing Structure:**
```json
{
  "pricing_principles": {
    "public_data": {"base_multiplier": 0.0},
    "personal_data": {"base_multiplier": 10.0},
    "business_data": {"base_multiplier": 100.0},
    "premium_analytics": {"base_multiplier": 1000.0}
  }
}
```

**2. Progressive Fee Scaling:**
```json
{
  "fee_structure": {
    "distance_0": {"multiplier": 0.0},
    "distance_1": {"multiplier": 1.0},
    "distance_2": {"multiplier": 2.0},
    "distance_3": {"multiplier": 5.0}
  }
}
```

**3. Payment Method Diversity:**
```json
{
  "payment_options": {
    "lightning": {"preferred": true, "instant": true},
    "credits": {"convenient": true, "prepaid": true},
    "bitcoin": {"backup": true, "high_value": true}
  }
}
```

### Performance Optimization

**1. Permission Caching:**
```json
{
  "permission_cache": {
    "enabled": true,
    "ttl": 300,
    "max_entries": 10000,
    "invalidation_strategy": "time_based"
  }
}
```

**2. Fee Calculation Caching:**
```json
{
  "fee_cache": {
    "enabled": true,
    "ttl": 600,
    "cache_key_fields": ["schema", "field", "distance"],
    "dynamic_pricing_refresh": 60
  }
}
```

**3. Batch Processing:**
```json
{
  "batch_processing": {
    "max_batch_size": 100,
    "batch_timeout": 5000,
    "parallel_processing": true
  }
}
```

### Monitoring and Alerting

**1. Access Monitoring:**
```bash
# Monitor unusual access patterns
curl http://localhost:9001/api/monitoring/access-anomalies

# Track high-value data access
curl http://localhost:9001/api/monitoring/premium-access
```

**2. Payment Monitoring:**
```bash
# Payment failure alerts
curl http://localhost:9001/api/monitoring/payment-failures

# Revenue tracking
curl http://localhost:9001/api/monitoring/revenue-summary
```

**3. Security Alerts:**
```json
{
  "alerts": {
    "permission_escalation": {
      "enabled": true,
      "threshold": 5,
      "window": 300
    },
    "payment_fraud": {
      "enabled": true,
      "patterns": ["rapid_succession", "amount_anomaly"]
    }
  }
}
```

---

**Next**: See [Developer Guide](./developer-guide.md) for integration and embedding documentation.