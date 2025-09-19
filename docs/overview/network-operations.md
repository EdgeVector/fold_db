# Network Operations

Fold DB provides comprehensive peer-to-peer networking capabilities that enable distributed data sharing, schema synchronization, and collaborative operations across multiple nodes.

## Table of Contents

1. [Network Architecture](#network-architecture)
2. [Node Discovery](#node-discovery)
3. [Connection Management](#connection-management)
4. [Schema Synchronization](#schema-synchronization)
5. [Distributed Queries](#distributed-queries)
6. [Trust Management](#trust-management)
7. [Network Configuration](#network-configuration)
8. [Monitoring and Diagnostics](#monitoring-and-diagnostics)
9. [Security](#security)
10. [Troubleshooting](#troubleshooting)

## Network Architecture

### P2P Foundation

Fold DB uses libp2p for peer-to-peer networking, providing:
- **Decentralized Architecture**: No single point of failure
- **Protocol Flexibility**: Support for multiple transport protocols
- **Security**: Built-in encryption and authentication
- **Discoverability**: Automatic peer discovery mechanisms

### Network Stack

```
┌─────────────────────────────────────────────┐
│               Application Layer              │
│     (Schema Sync, Queries, Data Sharing)   │
├─────────────────────────────────────────────┤
│            Request-Response Protocol         │
│        (Custom Fold DB Protocol)           │
├─────────────────────────────────────────────┤
│              Stream Multiplexing            │
│                 (Yamux)                     │
├─────────────────────────────────────────────┤
│              Security Layer                 │
│                (Noise)                      │
├─────────────────────────────────────────────┤
│             Transport Layer                 │
│               (TCP/UDP)                     │
└─────────────────────────────────────────────┘
```

### Network Topology

**Mesh Network:**
```
    Node A ←→ Node B
      ↕        ↕
    Node C ←→ Node D
```
Each node connects to multiple peers for redundancy.

**Hub-and-Spoke:**
```
  Node B ←→ Node A ←→ Node C
              ↕
           Node D
```
Central node coordinates communication.

**Hybrid Topology:**
```
Cluster 1: A ←→ B ←→ C
              ↕
           Gateway
              ↕
Cluster 2: D ←→ E ←→ F
```
Multiple clusters connected via gateways.

## Node Discovery

### mDNS Discovery

**Local Network Discovery:**
Automatically discover peers on the same network segment.

```json
{
  "network": {
    "enable_mdns": true,
    "mdns_service_name": "folddb",
    "mdns_interval": 30
  }
}
```

**Discovery Process:**
1. Node broadcasts mDNS service announcement
2. Other nodes respond with their addresses
3. Automatic connection establishment
4. Capability exchange and trust verification

### Bootstrap Peers

**Static Peer Configuration:**
```json
{
  "network": {
    "bootstrap_peers": [
      "/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWGK8YLjL...",
      "/ip4/10.0.0.5/tcp/9000/p2p/12D3KooWABC123...",
      "/dns4/folddb-node1.example.com/tcp/9000/p2p/12D3KooWXYZ789..."
    ]
  }
}
```

**Dynamic Bootstrap:**
```bash
# Start discovery
curl -X POST http://localhost:9001/api/network/start \
  -H "Content-Type: application/json" \
  -d '{
    "port": 9000,
    "enable_mdns": true
  }'

# Discover peers
curl -X POST http://localhost:9001/api/network/discover
```

### DHT (Distributed Hash Table)

**DHT Configuration:**
```json
{
  "network": {
    "enable_dht": true,
    "dht_bootstrap_peers": [
      "/dnsaddr/bootstrap.libp2p.io"
    ]
  }
}
```

**Peer Discovery via DHT:**
- Nodes advertise their services to the DHT
- Other nodes query DHT for specific services
- Enables discovery across internet boundaries
- Provides content-based routing

## Connection Management

### Connection Lifecycle

**Connection Establishment:**
1. **Discovery**: Find peer addresses
2. **Dial**: Initiate connection
3. **Handshake**: Authenticate and negotiate
4. **Protocol Selection**: Choose communication protocols
5. **Ready**: Connection available for use

### Connection Limits

**Configuration:**
```json
{
  "network": {
    "max_connections": 100,
    "max_connections_per_peer": 1,
    "connection_timeout": 10000,
    "keep_alive_interval": 30000
  }
}
```

**Connection Prioritization:**
- **Direct Trust**: Highest priority connections
- **Schema Sharing**: Connections with relevant schemas
- **Geographic**: Prefer closer nodes
- **Load Balancing**: Distribute connections evenly

### Connection Health

**Health Monitoring:**
```bash
curl http://localhost:9001/api/network/connections
```

**Response:**
```json
{
  "connections": [
    {
      "peer_id": "12D3KooWGK8YLjL...",
      "address": "/ip4/192.168.1.100/tcp/9000",
      "status": "connected",
      "connected_at": "2024-01-15T10:30:00Z",
      "latency_ms": 25,
      "bytes_sent": 1048576,
      "bytes_received": 2097152,
      "last_activity": "2024-01-15T11:00:00Z"
    }
  ]
}
```

**Connection Recovery:**
- Automatic reconnection on failure
- Exponential backoff for failed connections
- Circuit breaker pattern for problematic peers
- Graceful degradation when peers unavailable

## Schema Synchronization

### Schema Discovery

**Available Schemas Query:**
```bash
curl -X POST http://localhost:9001/api/network/query-schemas \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL..."
  }'
```

**Response:**
```json
{
  "peer_id": "12D3KooWGK8YLjL...",
  "schemas": [
    {
      "name": "UserProfile",
      "version": "1.0.0",
      "hash": "abc123...",
      "public": true,
      "permissions": {
        "read_distance": 1,
        "write_distance": 2
      }
    }
  ]
}
```

### Schema Sharing

**Request Schema from Peer:**
```bash
curl -X POST http://localhost:9001/api/network/request-schema \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL...",
    "schema_name": "UserProfile",
    "requested_version": "1.0.0"
  }'
```

**Share Schema with Network:**
```bash
curl -X POST http://localhost:9001/api/network/share-schema \
  -H "Content-Type: application/json" \
  -d '{
    "schema_name": "UserProfile",
    "visibility": "public",
    "allowed_peers": ["12D3KooWGK8YLjL..."],
    "trust_distance": 2
  }'
```

### Schema Propagation

**Automatic Propagation:**
```json
{
  "schema_propagation": {
    "enabled": true,
    "propagation_strategy": "flood",
    "max_hops": 3,
    "propagation_delay": 1000
  }
}
```

**Propagation Strategies:**
- **Flood**: Send to all connected peers
- **Gossip**: Random subset propagation
- **Tree**: Hierarchical propagation
- **DHT**: Content-based routing

### Version Management

**Schema Version Handling:**
```json
{
  "version_policy": {
    "auto_update": "minor_versions",
    "conflict_resolution": "ask_user",
    "backward_compatibility": true
  }
}
```

**Version Conflicts:**
```json
{
  "conflict": {
    "schema": "UserProfile",
    "local_version": "1.0.0",
    "remote_version": "1.1.0",
    "resolution_required": true,
    "suggested_action": "update_local"
  }
}
```

## Distributed Queries

### Cross-Node Queries

**Query Multiple Nodes:**
```bash
curl -X POST http://localhost:9001/api/network/distributed-query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "type": "query",
      "schema": "UserProfile",
      "fields": ["username", "email"],
      "filter": {
        "field": "location",
        "value": "San Francisco"
      }
    },
    "target_nodes": [
      "12D3KooWGK8YLjL...",
      "12D3KooWABC123..."
    ],
    "aggregation": "merge"
  }'
```

### Query Routing

**Routing Strategies:**
- **Broadcast**: Send to all nodes
- **Selective**: Send to nodes with relevant data
- **Cascading**: Try nodes in priority order
- **Load Balanced**: Distribute queries across nodes

**Query Optimization:**
```json
{
  "query_optimization": {
    "prefer_local": true,
    "cache_results": true,
    "parallel_execution": true,
    "timeout": 30000
  }
}
```

### Result Aggregation

**Aggregation Methods:**
- **Merge**: Combine all results
- **Union**: Remove duplicates
- **Intersection**: Common results only
- **Custom**: Application-specific logic

**Aggregated Response:**
```json
{
  "results": [
    {
      "source_node": "12D3KooWGK8YLjL...",
      "data": [
        {"username": "alice", "email": "alice@example.com"}
      ]
    },
    {
      "source_node": "12D3KooWABC123...",
      "data": [
        {"username": "bob", "email": "bob@example.com"}
      ]
    }
  ],
  "aggregated": [
    {"username": "alice", "email": "alice@example.com"},
    {"username": "bob", "email": "bob@example.com"}
  ],
  "metadata": {
    "nodes_queried": 2,
    "nodes_responded": 2,
    "total_results": 2,
    "query_time_ms": 150
  }
}
```

## Trust Management

### Trust Distance Model

**Trust Levels:**
- **Distance 0**: Local node (complete trust)
- **Distance 1**: Direct trusted peers
- **Distance 2**: Peers trusted by direct peers
- **Distance 3+**: Extended network trust

### Trust Configuration

**Trust Settings:**
```json
{
  "trust": {
    "default_distance": 1,
    "max_trust_distance": 3,
    "trust_decay": 0.1,
    "explicit_trust": {
      "12D3KooWGK8YLjL...": 0,
      "12D3KooWABC123...": 1
    },
    "trust_verification": {
      "require_signatures": true,
      "verify_certificates": true
    }
  }
}
```

### Trust Establishment

**Manual Trust Assignment:**
```bash
curl -X POST http://localhost:9001/api/network/set-trust \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL...",
    "trust_distance": 1,
    "reason": "verified_partner",
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

**Trust Propagation:**
```json
{
  "trust_propagation": {
    "enabled": true,
    "max_propagation_distance": 2,
    "propagation_weight": 0.8,
    "verification_required": true
  }
}
```

### Trust Verification

**Certificate-Based Trust:**
```json
{
  "trust_verification": {
    "certificate_authority": "trusted_ca.crt",
    "require_valid_certificate": true,
    "certificate_revocation_list": "crl.pem"
  }
}
```

**Reputation System:**
```json
{
  "reputation": {
    "enabled": true,
    "factors": {
      "uptime": 0.3,
      "response_time": 0.2,
      "data_quality": 0.3,
      "schema_compliance": 0.2
    },
    "reputation_threshold": 0.7
  }
}
```

## Network Configuration

### Basic Configuration

**Network Settings:**
```json
{
  "network": {
    "enabled": true,
    "node_id": "custom-node-identifier",
    "port": 9000,
    "bind_address": "0.0.0.0",
    "external_address": "203.0.113.10",
    "protocols": ["tcp", "ws"]
  }
}
```

### Advanced Configuration

**Protocol Settings:**
```json
{
  "network": {
    "transport": {
      "tcp": {
        "enabled": true,
        "nodelay": true,
        "keepalive": true
      },
      "websocket": {
        "enabled": true,
        "port": 9001
      },
      "quic": {
        "enabled": false,
        "port": 9002
      }
    },
    "security": {
      "noise": {
        "enabled": true,
        "keypair_file": "/keys/noise.key"
      }
    },
    "multiplexing": {
      "yamux": {
        "enabled": true,
        "max_buffer_size": 16777216
      }
    }
  }
}
```

### Firewall Configuration

**Required Ports:**
- **9000/tcp**: P2P networking
- **9001/tcp**: HTTP API (optional)
- **9090/tcp**: Metrics (optional)

**UFW Rules:**
```bash
# Allow P2P networking
sudo ufw allow 9000/tcp

# Allow HTTP API
sudo ufw allow 9001/tcp

# Allow specific peers only
sudo ufw allow from 192.168.1.100 to any port 9000
```

**Docker Networking:**
```yaml
version: '3.8'
services:
  folddb:
    image: folddb:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    networks:
      - folddb-network

networks:
  folddb-network:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16
```

## Monitoring and Diagnostics

### Network Status

**Overall Status:**
```bash
curl http://localhost:9001/api/network/status
```

**Response:**
```json
{
  "node_id": "12D3KooWABC123...",
  "network_active": true,
  "listening_addresses": [
    "/ip4/192.168.1.50/tcp/9000",
    "/ip6/::1/tcp/9000"
  ],
  "connected_peers": 3,
  "discovered_peers": 8,
  "uptime": 86400,
  "bytes_sent": 10485760,
  "bytes_received": 20971520
}
```

### Peer Information

**Peer Details:**
```bash
curl http://localhost:9001/api/network/peers
```

**Response:**
```json
{
  "peers": [
    {
      "peer_id": "12D3KooWGK8YLjL...",
      "addresses": ["/ip4/192.168.1.100/tcp/9000"],
      "connection_status": "connected",
      "trust_distance": 1,
      "shared_schemas": ["UserProfile", "Analytics"],
      "last_seen": "2024-01-15T11:00:00Z",
      "network_stats": {
        "latency_ms": 25,
        "bandwidth_mbps": 100,
        "reliability": 0.98
      }
    }
  ]
}
```

### Network Metrics

**Performance Metrics:**
```bash
curl http://localhost:9001/api/network/metrics
```

**Response:**
```json
{
  "connections": {
    "total": 5,
    "active": 4,
    "failed": 1
  },
  "traffic": {
    "messages_sent": 1250,
    "messages_received": 980,
    "bytes_sent": 5242880,
    "bytes_received": 3145728
  },
  "latency": {
    "min_ms": 5,
    "max_ms": 200,
    "avg_ms": 45,
    "p95_ms": 120
  },
  "discovery": {
    "mdns_discoveries": 15,
    "dht_discoveries": 3,
    "bootstrap_successes": 2
  }
}
```

### Network Diagnostics

**Connection Testing:**
```bash
curl -X POST http://localhost:9001/api/network/test-connection \
  -H "Content-Type: application/json" \
  -d '{
    "target": "/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWGK8YLjL...",
    "timeout": 10000
  }'
```

**Ping Test:**
```bash
curl -X POST http://localhost:9001/api/network/ping \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "12D3KooWGK8YLjL...",
    "count": 5
  }'
```

**Network Topology:**
```bash
curl http://localhost:9001/api/network/topology
```

## Security

### Transport Security

**Noise Protocol:**
- Encrypted communication between peers
- Perfect forward secrecy
- Mutual authentication
- Protection against man-in-the-middle attacks

**TLS Support:**
```json
{
  "network": {
    "tls": {
      "enabled": true,
      "cert_file": "/certs/node.crt",
      "key_file": "/certs/node.key",
      "ca_file": "/certs/ca.crt"
    }
  }
}
```

### Authentication

**Public Key Authentication:**
```json
{
  "authentication": {
    "method": "ed25519",
    "private_key_file": "/keys/node.key",
    "public_key_file": "/keys/node.pub"
  }
}
```

**Certificate-Based Authentication:**
```json
{
  "authentication": {
    "method": "certificate",
    "certificate_file": "/certs/node.crt",
    "private_key_file": "/certs/node.key",
    "ca_bundle": "/certs/ca-bundle.crt"
  }
}
```

### Access Control

**Network-Level Permissions:**
```json
{
  "network_permissions": {
    "allow_discovery": true,
    "allow_incoming_connections": true,
    "allowed_peers": [
      "12D3KooWGK8YLjL...",
      "12D3KooWABC123..."
    ],
    "blocked_peers": [
      "12D3KooWBADPEER..."
    ]
  }
}
```

**IP-Based Filtering:**
```json
{
  "ip_filtering": {
    "enabled": true,
    "whitelist": [
      "192.168.1.0/24",
      "10.0.0.0/8"
    ],
    "blacklist": [
      "203.0.113.0/24"
    ]
  }
}
```

## Troubleshooting

### Common Issues

**Connection Failures:**
```bash
# Check listening addresses
curl http://localhost:9001/api/network/status | jq .listening_addresses

# Test specific peer connection
curl -X POST http://localhost:9001/api/network/test-connection \
  -d '{"target": "/ip4/192.168.1.100/tcp/9000"}'

# Check firewall rules
sudo ufw status
netstat -tlnp | grep 9000
```

**Discovery Problems:**
```bash
# Check mDNS configuration
curl http://localhost:9001/api/network/status | jq .mdns_enabled

# Test local discovery
curl -X POST http://localhost:9001/api/network/discover

# Check bootstrap peers
curl http://localhost:9001/api/network/status | jq .bootstrap_peers
```

**Performance Issues:**
```bash
# Check network metrics
curl http://localhost:9001/api/network/metrics

# Test peer latency
curl -X POST http://localhost:9001/api/network/ping \
  -d '{"peer_id": "12D3KooWGK8YLjL...", "count": 10}'

# Monitor bandwidth usage
curl http://localhost:9001/api/network/bandwidth
```

### Diagnostic Commands

**Network Debugging:**
```bash
# Enable debug logging
curl -X POST http://localhost:9001/api/logs/features \
  -d '{"feature": "network", "level": "DEBUG"}'

# Get detailed peer information
curl http://localhost:9001/api/network/peers?detailed=true

# Check protocol versions
curl http://localhost:9001/api/network/protocols

# View connection history
curl http://localhost:9001/api/network/connection-history
```

### Recovery Procedures

**Network Reset:**
```bash
# Stop networking
curl -X POST http://localhost:9001/api/network/stop

# Clear peer cache
curl -X DELETE http://localhost:9001/api/network/peer-cache

# Restart networking
curl -X POST http://localhost:9001/api/network/start
```

**Configuration Reset:**
```bash
# Reset to default network config
curl -X POST http://localhost:9001/api/network/reset-config

# Regenerate node identity
curl -X POST http://localhost:9001/api/network/regenerate-identity
```

**Emergency Procedures:**
```bash
# Disconnect all peers
curl -X POST http://localhost:9001/api/network/disconnect-all

# Enter safe mode
curl -X POST http://localhost:9001/api/network/safe-mode

# Force reconnect to bootstrap peers
curl -X POST http://localhost:9001/api/network/force-bootstrap
```

---

**Next**: See [Permissions and Fees](./permissions-and-fees.md) for access control and payment system documentation.