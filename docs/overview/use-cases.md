# Fold DB Use Cases

This document provides comprehensive examples of how to use Fold DB for various scenarios, from simple data storage to complex distributed applications.

## Table of Contents

1. [Schema Management](#schema-management)
2. [Data Ingestion and Mutation](#data-ingestion-and-mutation)
3. [Querying and Data Retrieval](#querying-and-data-retrieval)
4. [Programmable Transforms](#programmable-transforms)
5. [Distributed Operations](#distributed-operations)
6. [Node Management](#node-management)
7. [API Access Patterns](#api-access-patterns)
8. [Permission and Fee Management](#permission-and-fee-management)
9. [Real-World Scenarios](#real-world-scenarios)

## Schema Management

### Loading Schemas

**Basic Schema Definition:**
```json
{
  "name": "UserProfile",
  "fields": {
    "username": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 1}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "trust_distance_scaling": {"None": null},
        "min_payment": null
      }
    },
    "email": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"Distance": 1},
        "write_policy": {"Distance": 1}
      },
      "payment_config": {
        "base_multiplier": 1.5,
        "min_payment": 50
      }
    },
    "profile_data": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 0}
      },
      "payment_config": {
        "base_multiplier": 2.0,
        "trust_distance_scaling": {
          "Linear": {
            "slope": 0.5,
            "intercept": 1.0,
            "min_factor": 1.0
          }
        }
      }
    }
  },
  "payment_config": {
    "base_multiplier": 1.2,
    "min_payment_threshold": 100
  }
}
```

**Loading via HTTP API:**
```bash
curl -X POST http://localhost:9001/api/schema \
  -H "Content-Type: application/json" \
  -d @userprofile_schema.json
```

**Loading via CLI:**
```bash
datafold_cli load-schema userprofile_schema.json
```

### Schema Immutability

Schemas are immutable in fold db. See [Schema Immutability](schema-management.md#schema-immutability) for detailed principles and migration patterns.

**Creating New Schema Versions:**
```json
{
  "name": "UserProfileV2",
  "fields": {
    "username": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 1}
      }
    },
    "email": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"Distance": 1},
        "write_policy": {"Distance": 1}
      }
    },
    "profile_data": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 0}
      }
    },
    "created_at": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 0}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment": null
      }
    }
  }
}
```

**Migration Patterns:**
When you need structural changes, follow these patterns:
1. Create a new schema with the desired structure
2. Migrate data from the old schema to the new schema
3. Update applications to use the new schema
4. Optionally remove the old schema when migration is complete

### Schema Validation

**Listing Loaded Schemas:**
```bash
# CLI
datafold_cli list-schemas

# HTTP API
curl http://localhost:9001/api/schemas
```

**Getting Schema Details:**
```bash
# CLI
datafold_cli get-schema UserProfile

# HTTP API
curl http://localhost:9001/api/schema/UserProfile
```

## Data Ingestion and Mutation

### Creating Records

**Basic Record Creation:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"UserProfile\",\"operation\":\"create\",\"data\":{\"username\":\"alice\",\"email\":\"alice@example.com\",\"profile_data\":{\"location\":\"San Francisco\",\"bio\":\"Software Engineer\",\"interests\":\"technology,hiking\"}}}"
  }'
```

**Batch Record Creation:**
```json
{
  "type": "mutation",
  "schema": "UserProfile",
  "operation": "create_batch",
  "data": [
    {
      "username": "bob",
      "email": "bob@example.com",
      "profile_data": {
        "location": "New York",
        "bio": "Product Manager"
      }
    },
    {
      "username": "carol", 
      "email": "carol@example.com",
      "profile_data": {
        "location": "Austin",
        "bio": "Designer"
      }
    }
  ]
}
```

### Updating Records

**Field Updates:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"UserProfile\",\"operation\":\"update\",\"filter\":{\"username\":\"alice\"},\"data\":{\"profile_data\":{\"location\":\"Seattle\",\"bio\":\"Senior Software Engineer\"}}}"
  }'
```

**Range Field Updates:**
```json
{
  "type": "mutation",
  "schema": "UserProfile", 
  "operation": "update",
  "filter": {"username": "alice"},
  "data": {
    "profile_data": {
      "skills:programming": "advanced",
      "skills:leadership": "intermediate",
      "certifications:aws": "solutions-architect"
    }
  }
}
```

### Deleting Records

**Single Record Deletion:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"UserProfile\",\"operation\":\"delete\",\"filter\":{\"username\":\"alice\"}}"
  }'
```

**Conditional Deletion:**
```json
{
  "type": "mutation",
  "schema": "UserProfile",
  "operation": "delete",
  "filter": {
    "field": "profile_data",
    "range_filter": {
      "Key": "status:inactive"
    }
  }
}
```

## Querying and Data Retrieval

### Basic Queries

**Field Selection:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"query\",\"schema\":\"UserProfile\",\"fields\":[\"username\",\"email\"],\"filter\":null}"
  }'
```

**Filtered Queries:**
```json
{
  "type": "query",
  "schema": "UserProfile",
  "fields": ["username", "email", "profile_data"],
  "filter": {
    "username": "alice"
  }
}
```

### Range Field Queries

**Key-Based Queries:**
```json
{
  "type": "query",
  "schema": "EventAnalytics",
  "fields": ["event_name", "metrics_by_timeframe"],
  "filter": {
    "field": "metrics_by_timeframe",
    "range_filter": {
      "Key": "2024-01-01:daily"
    }
  }
}
```

**Prefix Queries:**
```json
{
  "type": "query",
  "schema": "EventAnalytics", 
  "fields": ["event_name", "metrics_by_timeframe"],
  "filter": {
    "field": "metrics_by_timeframe",
    "range_filter": {
      "KeyPrefix": "2024-01-01:hourly"
    }
  }
}
```

**Range Queries:**
```json
{
  "type": "query",
  "schema": "EventAnalytics",
  "fields": ["event_name", "user_segments"],
  "filter": {
    "field": "user_segments",
    "range_filter": {
      "KeyRange": {
        "start": "geo:us-east",
        "end": "geo:us-west"
      }
    }
  }
}
```

**Pattern Matching:**
```json
{
  "type": "query",
  "schema": "EventAnalytics",
  "fields": ["event_name", "user_segments"],
  "filter": {
    "field": "user_segments", 
    "range_filter": {
      "KeyPattern": "device:*"
    }
  }
}
```

**Multiple Key Queries:**
```json
{
  "type": "query",
  "schema": "EventAnalytics",
  "fields": ["event_name", "user_segments"],
  "filter": {
    "field": "user_segments",
    "range_filter": {
      "Keys": [
        "segment:premium",
        "segment:basic",
        "geo:us-east"
      ]
    }
  }
}
```

**Value-Based Queries:**
```json
{
  "type": "query",
  "schema": "UserProfile",
  "fields": ["username", "profile_data"],
  "filter": {
    "field": "profile_data",
    "range_filter": {
      "Value": "San Francisco"
    }
  }
}
```

### Complex Filtering

**Compound Filters:**
```json
{
  "type": "query",
  "schema": "UserProfile",
  "fields": ["username", "email"],
  "filter": {
    "and": [
      {
        "field": "profile_data",
        "range_filter": {
          "Key": "location"
        }
      },
      {
        "field": "profile_data", 
        "range_filter": {
          "Value": "San Francisco"
        }
      }
    ]
  }
}
```

## Programmable Transforms

### Transform Definition

**Basic Transform:**
```json
{
  "name": "UserStatus",
  "fields": {
    "age": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 1}
      }
    },
    "status": {
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"NoRequirement": null},
        "write_policy": {"Distance": 0}
      },
      "transform": {
        "inputs": ["age"],
        "logic": "if age >= 18 { return \"adult\" } else { return \"minor\" }",
        "output": "UserStatus.status"
      },
      "writable": false
    }
  }
}
```

**Mathematical Transform:**
```json
{
  "name": "Analytics",
  "fields": {
    "session_start": {"field_type": "Single"},
    "session_end": {"field_type": "Single"},
    "session_duration": {
      "field_type": "Single",
      "transform": {
        "inputs": ["session_start", "session_end"],
        "logic": "return session_end - session_start",
        "output": "Analytics.session_duration"
      },
      "writable": false
    }
  }
}
```

**Aggregation Transform:**
```json
{
  "name": "Analytics",
  "fields": {
    "conversions": {"field_type": "Single"},
    "total_visits": {"field_type": "Single"},
    "conversion_rate": {
      "field_type": "Single",
      "transform": {
        "inputs": ["conversions", "total_visits"],
        "logic": "return (conversions / total_visits) * 100",
        "output": "Analytics.conversion_rate"
      },
      "writable": false
    }
  }
}
```

### Transform Execution

**Automatic Execution:**
When a field with a transform dependency changes, the transform executes automatically:

```bash
# Update session_end field
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"Analytics\",\"operation\":\"update\",\"filter\":{\"id\":\"session123\"},\"data\":{\"session_end\":1609459200}}"
  }'

# session_duration is automatically calculated
```

**Manual Transform Registration:**
```bash
curl -X POST http://localhost:9001/api/transform/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "custom_calculation",
    "inputs": ["field1", "field2"],
    "logic": "return field1 * field2 + 10",
    "output": "result_field"
  }'
```

## Distributed Operations

### Node Discovery

**Start Networking:**
```bash
curl -X POST http://localhost:9001/api/network/start \
  -H "Content-Type: application/json" \
  -d '{
    "port": 9000,
    "enable_mdns": true
  }'
```

**Discover Peers:**
```bash
curl -X POST http://localhost:9001/api/network/discover
```

**Connect to Specific Node:**
```bash
curl -X POST http://localhost:9001/api/network/connect \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "12D3KooWGK8YLjL...",
    "address": "/ip4/192.168.1.100/tcp/9000"
  }'
```

### Schema Synchronization

**Request Schema from Peer:**
```bash
curl -X POST http://localhost:9001/api/network/request-schema \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL...",
    "schema_name": "UserProfile"
  }'
```

**Share Schema with Network:**
```bash
curl -X POST http://localhost:9001/api/network/share-schema \
  -H "Content-Type: application/json" \
  -d '{
    "schema_name": "UserProfile",
    "trust_distance": 2
  }'
```

### Distributed Queries

**Cross-Node Query:**
```json
{
  "type": "distributed_query",
  "schema": "UserProfile",
  "fields": ["username", "email"],
  "filter": {
    "field": "profile_data",
    "range_filter": {
      "Key": "location"
    }
  },
  "nodes": [
    "12D3KooWGK8YLjL...",
    "12D3KooWABC123..."
  ]
}
```

## Node Management

### Configuration

**Node Configuration File:**
```json
{
  "storage_path": "data/db",
  "default_trust_distance": 1,
  "network": {
    "port": 9000,
    "enable_mdns": true,
    "bootstrap_peers": [
      "/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWGK8YLjL..."
    ]
  },
  "api": {
    "http_port": 9001,
    "tcp_port": 9000,
    "enable_cors": true
  },
  "logging": {
    "level": "INFO",
    "features": {
      "network": "DEBUG",
      "schema": "WARN"
    }
  }
}
```

### Service Management

**Start Node:**
```bash
# Standalone node
cargo run --bin datafold_node -- --config config.json

# HTTP server with web UI
cargo run --bin datafold_http_server -- --port 9001
```

**Health Check:**
```bash
curl http://localhost:9001/api/health
```

**Node Status:**
```bash
curl http://localhost:9001/api/status
```

**Stop Services:**
```bash
curl -X POST http://localhost:9001/api/system/shutdown
```

### Monitoring

**System Metrics:**
```bash
curl http://localhost:9001/api/metrics
```

**Log Streaming:**
```bash
curl http://localhost:9001/api/logs/stream
```

**Performance Statistics:**
```bash
curl http://localhost:9001/api/stats
```

## API Access Patterns

### CLI Usage

**Schema Operations:**
```bash
# Load schema
datafold_cli load-schema schema.json

# List schemas
datafold_cli list-schemas

# Get schema details
datafold_cli get-schema UserProfile
```

**Data Operations:**
```bash
# Query data
datafold_cli query --schema UserProfile --fields username,email

# Create record
datafold_cli mutate --schema UserProfile --operation create --data '{"username":"alice","email":"alice@example.com"}'

# Update record
datafold_cli mutate --schema UserProfile --operation update --filter '{"username":"alice"}' --data '{"email":"newemail@example.com"}'
```

### HTTP REST API

**Authentication:**
```bash
# Using API key
curl -H "Authorization: Bearer your-api-key" \
  http://localhost:9001/api/schemas

# Using signature
curl -H "X-Signature: signature-hash" \
  -H "X-Timestamp: 1609459200" \
  http://localhost:9001/api/schemas
```

**Batch Operations:**
```bash
curl -X POST http://localhost:9001/api/batch \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [
      {"type": "query", "schema": "UserProfile", "fields": ["username"]},
      {"type": "mutation", "schema": "UserProfile", "operation": "create", "data": {...}}
    ]
  }'
```

### TCP Protocol

**Python Client Example:**
```python
import socket
import json
import struct

def send_request(sock, request):
    request_json = json.dumps(request).encode('utf-8')
    sock.sendall(struct.pack('!I', len(request_json)))
    sock.sendall(request_json)
    
    response_len = struct.unpack('!I', sock.recv(4))[0]
    response_json = sock.recv(response_len)
    return json.loads(response_json.decode('utf-8'))

# Connect and query
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('localhost', 9000))

request = {
    "app_id": "my-app",
    "operation": "query",
    "params": {
        "schema": "UserProfile",
        "fields": ["username", "email"]
    }
}

response = send_request(sock, request)
print(response)
```

## Permission and Fee Management

### Access Control Configuration

**Trust Distance Setup:**
```bash
curl -X POST http://localhost:9001/api/permissions/trust-distance \
  -H "Content-Type: application/json" \
  -d '{
    "default_distance": 1,
    "peer_distances": {
      "12D3KooWGK8YLjL...": 0,
      "12D3KooWABC123...": 2
    }
  }'
```

**Explicit Permissions:**
```bash
curl -X POST http://localhost:9001/api/permissions/explicit \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "UserProfile",
    "field": "email",
    "permission": "read",
    "public_key": "ed25519:ABC123...",
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

### Fee Management

**Payment Configuration:**
```json
{
  "schema": "Analytics",
  "field": "conversion_rate",
  "payment_config": {
    "base_multiplier": 3.0,
    "trust_distance_scaling": {
      "Exponential": {
        "base": 1.5,
        "scale": 1.0,
        "min_factor": 1.0
      }
    },
    "min_payment": 25
  }
}
```

**Lightning Network Integration:**
```bash
curl -X POST http://localhost:9001/api/payments/lightning/invoice \
  -H "Content-Type: application/json" \
  -d '{
    "amount_sats": 1000,
    "description": "Access to UserProfile.email field",
    "expiry": 3600
  }'
```

**Payment Verification:**
```bash
curl -X POST http://localhost:9001/api/payments/verify \
  -H "Content-Type: application/json" \
  -d '{
    "payment_hash": "abc123...",
    "operation": "query",
    "schema": "UserProfile",
    "fields": ["email"]
  }'
```

## Real-World Scenarios

### E-commerce Platform

**Product Catalog Schema:**
```json
{
  "name": "ProductCatalog",
  "fields": {
    "name": {"field_type": "Single"},
    "description": {"field_type": "Single"},
    "price": {"field_type": "Single"},
    "inventory_by_location": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"Distance": 1},
        "write_policy": {"Distance": 0}
      }
    },
    "attributes": {
      "field_type": "Range"
    }
  }
}
```

**Inventory Management:**
```bash
# Add product with inventory
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"ProductCatalog\",\"operation\":\"create\",\"data\":{\"name\":\"Laptop\",\"price\":\"999.99\",\"inventory_by_location\":{\"warehouse:north\":\"50\",\"warehouse:south\":\"30\",\"store:downtown\":\"5\"}}}"
  }'

# Query inventory by location
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"query\",\"schema\":\"ProductCatalog\",\"fields\":[\"name\",\"inventory_by_location\"],\"filter\":{\"field\":\"inventory_by_location\",\"range_filter\":{\"KeyPrefix\":\"warehouse:\"}}}"
  }'
```

### Analytics Platform

**Event Analytics Schema:**
```json
{
  "name": "EventAnalytics",
  "fields": {
    "event_name": {"field_type": "Single"},
    "event_type": {"field_type": "Single"},
    "metrics_by_timeframe": {
      "field_type": "Range",
      "payment_config": {
        "base_multiplier": 1.2,
        "min_payment": 100
      }
    },
    "user_segments": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"Distance": 1}
      }
    }
  }
}
```

**Time-series Data Ingestion:**
```bash
# Ingest hourly metrics
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"EventAnalytics\",\"operation\":\"create\",\"data\":{\"event_name\":\"User Login\",\"metrics_by_timeframe\":{\"2024-01-01:hourly:00\":\"45\",\"2024-01-01:hourly:01\":\"52\",\"2024-01-01:daily\":\"1250\"}}}"
  }'
```

### Social Network

**User Relationship Management:**
```json
{
  "name": "UserRelationships",
  "fields": {
    "user_id": {"field_type": "Single"},
    "relationships": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"Distance": 2},
        "write_policy": {"Distance": 1}
      }
    },
    "privacy_settings": {
      "field_type": "Range",
      "permission_policy": {
        "read_policy": {"Distance": 0},
        "write_policy": {"Distance": 0}
      }
    }
  }
}
```

### IoT Data Collection

**Sensor Data Schema:**
```json
{
  "name": "SensorData",
  "fields": {
    "device_id": {"field_type": "Single"},
    "sensor_readings": {
      "field_type": "Range",
      "payment_config": {
        "base_multiplier": 0.1,
        "min_payment": 1
      }
    },
    "alerts": {
      "field_type": "Range",
      "transform": {
        "inputs": ["sensor_readings"],
        "logic": "if temperature > 75 { return \"HIGH_TEMP\" }",
        "output": "SensorData.alerts"
      }
    }
  }
}
```

---

**Next**: See [API Reference](./api-reference.md) for complete API documentation.