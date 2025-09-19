# Deployment Guide

This guide covers deployment patterns, configuration options, and best practices for running Fold DB in various environments.

## Table of Contents

1. [Deployment Patterns](#deployment-patterns)
2. [Configuration](#configuration)
3. [Environment Setup](#environment-setup)
4. [Production Deployment](#production-deployment)
5. [Monitoring and Maintenance](#monitoring-and-maintenance)
6. [Scaling Strategies](#scaling-strategies)
7. [Security Configuration](#security-configuration)
8. [Troubleshooting](#troubleshooting)

## Deployment Patterns

### Standalone Node

A single node deployment for development or small-scale applications.

**Use Cases:**
- Development and testing
- Small applications with local data
- Proof of concept implementations
- Edge computing scenarios

**Configuration:**
```json
{
  "storage_path": "data/db",
  "default_trust_distance": 0,
  "network": {
    "enabled": false
  },
  "api": {
    "http_port": 9001,
    "tcp_port": 9000,
    "enable_cors": true,
    "allowed_origins": ["http://localhost:3000"]
  }
}
```

**Deployment:**
```bash
# Build the application
cargo build --release

# Start standalone node
./target/release/datafold_node --config standalone_config.json

# Or with HTTP server for web UI
./target/release/datafold_http_server --port 9001 --config standalone_config.json
```

### Clustered Deployment

Multiple nodes forming a distributed cluster.

**Use Cases:**
- High availability requirements
- Load distribution
- Geographic distribution
- Fault tolerance

**Architecture:**
```
    ┌─────────────┐       ┌─────────────┐       ┌─────────────┐
    │   Node A    │◀─────▶│   Node B    │◀─────▶│   Node C    │
    │   (Leader)  │       │ (Follower)  │       │ (Follower)  │
    └─────────────┘       └─────────────┘       └─────────────┘
           │                       │                       │
           └───────────────────────┼───────────────────────┘
                                   │
                              ┌─────────────┐
                              │   Node D    │
                              │ (Follower)  │
                              └─────────────┘
```

**Node Configuration:**
```json
{
  "node_id": "node-a",
  "storage_path": "data/node-a/db",
  "default_trust_distance": 1,
  "network": {
    "enabled": true,
    "port": 9000,
    "enable_mdns": true,
    "bootstrap_peers": [
      "/ip4/10.0.0.2/tcp/9000/p2p/12D3KooWBNodeB...",
      "/ip4/10.0.0.3/tcp/9000/p2p/12D3KooWCNodeC..."
    ]
  },
  "api": {
    "http_port": 9001,
    "tcp_port": 9000
  },
  "cluster": {
    "role": "leader",
    "peers": ["node-b", "node-c", "node-d"],
    "heartbeat_interval": 5000,
    "election_timeout": 15000
  }
}
```

**Deployment with Docker Compose:**
```yaml
version: '3.8'
services:
  node-a:
    image: folddb:latest
    ports:
      - "9001:9001"
      - "9000:9000"
    volumes:
      - ./data/node-a:/app/data
      - ./config/node-a.json:/app/config.json
    command: ["./datafold_http_server", "--config", "/app/config.json"]
    
  node-b:
    image: folddb:latest
    ports:
      - "9011:9001"
      - "9010:9000"
    volumes:
      - ./data/node-b:/app/data
      - ./config/node-b.json:/app/config.json
    command: ["./datafold_http_server", "--config", "/app/config.json"]
    
  node-c:
    image: folddb:latest
    ports:
      - "9021:9001"
      - "9020:9000"
    volumes:
      - ./data/node-c:/app/data
      - ./config/node-c.json:/app/config.json
    command: ["./datafold_http_server", "--config", "/app/config.json"]
```

### Embedded Library

Integrate Fold DB as a library within applications.

**Use Cases:**
- Application-specific data management
- Embedded systems
- Mobile applications
- Desktop applications with local data

**Rust Integration:**
```rust
use fold_node::{DataFoldNode, NodeConfig, FoldDB};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NodeConfig {
        storage_path: PathBuf::from("./app_data"),
        default_trust_distance: 0,
        network_enabled: false,
    };
    
    let node = DataFoldNode::new(config).await?;
    
    // Load application schemas
    let schema_json = include_str!("schemas/app_schema.json");
    let schema: Schema = serde_json::from_str(schema_json)?;
    node.load_schema(schema).await?;
    
    // Use the node for application data operations
    let query_result = node.query(Query::new("AppData")
        .select(&["field1", "field2"])
        .filter("id", "eq", "123")).await?;
    
    Ok(())
}
```

**C FFI Interface:**
```c
// fold_db_c.h
typedef struct FoldDBNode FoldDBNode;

FoldDBNode* folddb_create(const char* config_path);
void folddb_destroy(FoldDBNode* node);
int folddb_load_schema(FoldDBNode* node, const char* schema_json);
char* folddb_query(FoldDBNode* node, const char* query_json);
int folddb_mutate(FoldDBNode* node, const char* mutation_json);
```

### Kubernetes Deployment

Container orchestration for production environments.

**Namespace Configuration:**
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: folddb
```

**ConfigMap:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: folddb-config
  namespace: folddb
data:
  config.json: |
    {
      "storage_path": "/data/db",
      "default_trust_distance": 1,
      "network": {
        "enabled": true,
        "port": 9000,
        "enable_mdns": false,
        "bootstrap_peers": []
      },
      "api": {
        "http_port": 9001,
        "tcp_port": 9000
      }
    }
```

**StatefulSet:**
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: folddb
  namespace: folddb
spec:
  serviceName: folddb
  replicas: 3
  selector:
    matchLabels:
      app: folddb
  template:
    metadata:
      labels:
        app: folddb
    spec:
      containers:
      - name: folddb
        image: folddb:1.0.0
        ports:
        - containerPort: 9001
          name: http
        - containerPort: 9000
          name: tcp
        volumeMounts:
        - name: config
          mountPath: /app/config
        - name: data
          mountPath: /data
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: config
        configMap:
          name: folddb-config
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
```

**Service:**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: folddb-service
  namespace: folddb
spec:
  selector:
    app: folddb
  ports:
  - name: http
    port: 9001
    targetPort: 9001
  - name: tcp
    port: 9000
    targetPort: 9000
  type: ClusterIP
```

**Ingress:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: folddb-ingress
  namespace: folddb
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
  - hosts:
    - folddb.example.com
    secretName: folddb-tls
  rules:
  - host: folddb.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: folddb-service
            port:
              number: 9001
```

## Configuration

### Node Configuration

**Complete Configuration File:**
```json
{
  "node_id": "unique-node-identifier",
  "storage_path": "data/db",
  "default_trust_distance": 1,
  
  "network": {
    "enabled": true,
    "port": 9000,
    "bind_address": "0.0.0.0",
    "enable_mdns": true,
    "mdns_service_name": "folddb",
    "bootstrap_peers": [
      "/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWNode1...",
      "/ip4/192.168.1.101/tcp/9000/p2p/12D3KooWNode2..."
    ],
    "max_connections": 100,
    "connection_timeout": 10000,
    "keep_alive_interval": 30000
  },
  
  "api": {
    "http_port": 9001,
    "tcp_port": 9000,
    "bind_address": "0.0.0.0",
    "enable_cors": true,
    "allowed_origins": ["*"],
    "max_request_size": 10485760,
    "request_timeout": 30000,
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 1000,
      "burst_size": 100
    }
  },
  
  "storage": {
    "backend": "sled",
    "path": "data/db",
    "cache_size": 268435456,
    "compression": true,
    "sync_mode": "normal",
    "backup": {
      "enabled": true,
      "interval": 3600,
      "retention_days": 7,
      "path": "backups/"
    }
  },
  
  "logging": {
    "level": "INFO",
    "format": "structured",
    "outputs": {
      "console": {
        "enabled": true,
        "level": "INFO"
      },
      "file": {
        "enabled": true,
        "level": "DEBUG",
        "path": "logs/folddb.log",
        "rotation": "daily",
        "max_files": 30
      },
      "web": {
        "enabled": true,
        "level": "INFO",
        "buffer_size": 1000
      }
    },
    "features": {
      "network": "DEBUG",
      "schema": "INFO",
      "transform": "INFO",
      "database": "WARN",
      "permissions": "INFO"
    }
  },
  
  "security": {
    "tls": {
      "enabled": false,
      "cert_file": "certs/server.crt",
      "key_file": "certs/server.key",
      "ca_file": "certs/ca.crt"
    },
    "authentication": {
      "enabled": true,
      "method": "public_key",
      "api_keys": {
        "admin": "secure-api-key-here"
      }
    }
  },
  
  "performance": {
    "worker_threads": 4,
    "max_concurrent_requests": 1000,
    "query_timeout": 30000,
    "mutation_timeout": 60000,
    "transform_timeout": 10000,
    "gc_interval": 300000
  },
  
  "monitoring": {
    "metrics": {
      "enabled": true,
      "port": 9090,
      "path": "/metrics"
    },
    "health_check": {
      "enabled": true,
      "port": 9091,
      "path": "/health"
    }
  }
}
```

### Environment Variables

**Configuration Override:**
```bash
# Core settings
export FOLDDB_STORAGE_PATH=/data/db
export FOLDDB_DEFAULT_TRUST_DISTANCE=1

# Network settings
export FOLDDB_NETWORK_PORT=9000
export FOLDDB_NETWORK_ENABLED=true
export FOLDDB_NETWORK_MDNS=true

# API settings
export FOLDDB_HTTP_PORT=9001
export FOLDDB_TCP_PORT=9000
export FOLDDB_ENABLE_CORS=true

# Logging settings
export FOLDDB_LOG_LEVEL=INFO
export FOLDDB_LOG_FORMAT=structured
export FOLDDB_LOG_FILE_PATH=/var/log/folddb.log

# Performance settings
export FOLDDB_WORKER_THREADS=4
export FOLDDB_MAX_CONCURRENT_REQUESTS=1000

# Security settings
export FOLDDB_TLS_ENABLED=false
export FOLDDB_AUTH_ENABLED=true
```

### Schema Directory Configuration

**Schema Loading:**
```json
{
  "schemas": {
    "auto_load": true,
    "directory": "schemas/",
    "patterns": ["*.json"],
    "validation": "strict",
    "reload_on_change": true
  }
}
```

**Directory Structure:**
```
schemas/
├── core/
│   ├── user_profile.json
│   └── system_config.json
├── analytics/
│   ├── event_analytics.json
│   └── user_behavior.json
└── custom/
    ├── app_specific.json
    └── transforms.json
```

## Environment Setup

### Development Environment

**Prerequisites:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add clippy

# Install development tools
cargo install cargo-watch
cargo install cargo-llvm-cov
```

**Development Configuration:**
```json
{
  "storage_path": "dev_data/db",
  "default_trust_distance": 0,
  "network": {
    "enabled": false
  },
  "api": {
    "http_port": 9001,
    "enable_cors": true,
    "allowed_origins": ["http://localhost:3000", "http://localhost:5173"]
  },
  "logging": {
    "level": "DEBUG",
    "format": "human",
    "outputs": {
      "console": {"enabled": true, "level": "DEBUG"}
    }
  }
}
```

**Development Scripts:**
```bash
#!/bin/bash
# scripts/dev.sh

# Clean previous data
rm -rf dev_data/

# Start development server with hot reload
cargo watch -x "run --bin datafold_http_server -- --config dev_config.json"
```

### Testing Environment

**Test Configuration:**
```json
{
  "storage_path": "test_data/db",
  "default_trust_distance": 0,
  "network": {
    "enabled": true,
    "port": 19000
  },
  "api": {
    "http_port": 19001,
    "tcp_port": 19000
  },
  "logging": {
    "level": "WARN",
    "outputs": {
      "console": {"enabled": false}
    }
  }
}
```

**Test Script:**
```bash
#!/bin/bash
# scripts/test.sh

# Setup test environment
export FOLDDB_CONFIG=test_config.json
export RUST_LOG=warn

# Run all tests
cargo test --workspace --release

# Generate coverage report
cargo llvm-cov --html --output-dir target/coverage
```

### Staging Environment

**Staging Configuration:**
```json
{
  "storage_path": "/data/staging/db",
  "default_trust_distance": 1,
  "network": {
    "enabled": true,
    "port": 9000,
    "bootstrap_peers": ["staging-peers..."]
  },
  "api": {
    "http_port": 9001,
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 500
    }
  },
  "logging": {
    "level": "INFO",
    "outputs": {
      "file": {
        "enabled": true,
        "path": "/var/log/folddb-staging.log"
      }
    }
  },
  "monitoring": {
    "metrics": {"enabled": true},
    "health_check": {"enabled": true}
  }
}
```

## Production Deployment

### Infrastructure Requirements

**Minimum Requirements:**
- CPU: 2 cores
- RAM: 4GB
- Storage: 20GB SSD
- Network: 100 Mbps

**Recommended Production:**
- CPU: 8 cores
- RAM: 16GB
- Storage: 100GB NVMe SSD
- Network: 1 Gbps

### Production Configuration

```json
{
  "storage_path": "/data/folddb",
  "default_trust_distance": 2,
  
  "network": {
    "enabled": true,
    "port": 9000,
    "bind_address": "0.0.0.0",
    "max_connections": 200,
    "bootstrap_peers": ["production-bootstrap-peers..."]
  },
  
  "api": {
    "http_port": 9001,
    "bind_address": "127.0.0.1",
    "enable_cors": false,
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 2000,
      "burst_size": 200
    }
  },
  
  "storage": {
    "cache_size": 1073741824,
    "backup": {
      "enabled": true,
      "interval": 1800,
      "retention_days": 30
    }
  },
  
  "logging": {
    "level": "INFO",
    "outputs": {
      "file": {
        "enabled": true,
        "path": "/var/log/folddb/folddb.log",
        "rotation": "daily",
        "max_files": 90
      }
    }
  },
  
  "security": {
    "tls": {
      "enabled": true,
      "cert_file": "/etc/ssl/certs/folddb.crt",
      "key_file": "/etc/ssl/private/folddb.key"
    },
    "authentication": {
      "enabled": true,
      "method": "public_key"
    }
  },
  
  "performance": {
    "worker_threads": 8,
    "max_concurrent_requests": 2000,
    "gc_interval": 600000
  },
  
  "monitoring": {
    "metrics": {"enabled": true, "port": 9090},
    "health_check": {"enabled": true, "port": 9091}
  }
}
```

### Systemd Service

**Service File:**
```ini
# /etc/systemd/system/folddb.service
[Unit]
Description=Fold DB Node
After=network.target
Wants=network.target

[Service]
Type=simple
User=folddb
Group=folddb
ExecStart=/usr/local/bin/datafold_http_server --config /etc/folddb/production.json
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=folddb

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/data/folddb /var/log/folddb

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

**Service Management:**
```bash
# Enable and start service
sudo systemctl enable folddb
sudo systemctl start folddb

# Check status
sudo systemctl status folddb

# View logs
sudo journalctl -u folddb -f

# Restart service
sudo systemctl restart folddb
```

### Load Balancer Configuration

**Nginx Configuration:**
```nginx
upstream folddb_nodes {
    least_conn;
    server 10.0.0.10:9001 max_fails=3 fail_timeout=30s;
    server 10.0.0.11:9001 max_fails=3 fail_timeout=30s;
    server 10.0.0.12:9001 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    server_name folddb.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name folddb.example.com;
    
    ssl_certificate /etc/ssl/certs/folddb.crt;
    ssl_certificate_key /etc/ssl/private/folddb.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    
    location / {
        proxy_pass http://folddb_nodes;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        proxy_connect_timeout 5s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
    
    location /api/logs/stream {
        proxy_pass http://folddb_nodes;
        proxy_buffering off;
        proxy_cache off;
        proxy_set_header Connection '';
        proxy_http_version 1.1;
        chunked_transfer_encoding off;
    }
    
    location /health {
        access_log off;
        proxy_pass http://folddb_nodes/api/health;
    }
}
```

### Database Backup Strategy

**Backup Script:**
```bash
#!/bin/bash
# scripts/backup.sh

BACKUP_DIR="/backups/folddb"
DATE=$(date +%Y%m%d_%H%M%S)
DB_PATH="/data/folddb"

# Create backup directory
mkdir -p "$BACKUP_DIR/$DATE"

# Stop writes (optional, for consistency)
curl -X POST http://localhost:9001/api/system/pause-writes

# Create backup
tar -czf "$BACKUP_DIR/$DATE/folddb_backup.tar.gz" -C "$DB_PATH" .

# Resume writes
curl -X POST http://localhost:9001/api/system/resume-writes

# Cleanup old backups (keep 30 days)
find "$BACKUP_DIR" -type d -mtime +30 -exec rm -rf {} \;

echo "Backup completed: $BACKUP_DIR/$DATE/folddb_backup.tar.gz"
```

**Cron Configuration:**
```bash
# Run backup every 6 hours
0 */6 * * * /opt/folddb/scripts/backup.sh >> /var/log/folddb/backup.log 2>&1

# Run daily maintenance
0 2 * * * /opt/folddb/scripts/maintenance.sh >> /var/log/folddb/maintenance.log 2>&1
```

## Monitoring and Maintenance

### Health Monitoring

**Health Check Script:**
```bash
#!/bin/bash
# scripts/health_check.sh

ENDPOINT="http://localhost:9001/api/health"
TIMEOUT=10

response=$(curl -s -w "%{http_code}" --max-time $TIMEOUT "$ENDPOINT")
http_code="${response: -3}"

if [ "$http_code" = "200" ]; then
    echo "OK: Fold DB is healthy"
    exit 0
else
    echo "CRITICAL: Fold DB health check failed (HTTP $http_code)"
    exit 2
fi
```

**Prometheus Metrics:**
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'folddb'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 30s
    metrics_path: /metrics
```

**Grafana Dashboard:**
```json
{
  "dashboard": {
    "title": "Fold DB Monitoring",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(folddb_requests_total[5m])",
            "legendFormat": "{{method}} {{endpoint}}"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph", 
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(folddb_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      }
    ]
  }
}
```

### Log Aggregation

**Filebeat Configuration:**
```yaml
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/folddb/*.log
  json.keys_under_root: true
  json.add_error_key: true
  fields:
    service: folddb
    environment: production

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "folddb-logs-%{+yyyy.MM.dd}"

logging.level: info
```

### Maintenance Tasks

**Daily Maintenance Script:**
```bash
#!/bin/bash
# scripts/maintenance.sh

# Rotate logs
/usr/sbin/logrotate /etc/logrotate.d/folddb

# Cleanup temporary files
find /tmp -name "folddb-*" -mtime +1 -delete

# Check disk space
df -h /data/folddb

# Check memory usage
free -h

# Database statistics
curl -s http://localhost:9001/api/stats | jq .

# Network connectivity
curl -s http://localhost:9001/api/network/status | jq .
```

## Scaling Strategies

### Horizontal Scaling

**Node Addition:**
```bash
# Add new node to cluster
./deploy_new_node.sh --cluster-id production --node-id node-04 --bootstrap-peers "existing-peers..."
```

**Load Distribution:**
```yaml
# HAProxy configuration
backend folddb_cluster
    balance roundrobin
    option httpchk GET /api/health
    server node1 10.0.0.10:9001 check
    server node2 10.0.0.11:9001 check
    server node3 10.0.0.12:9001 check
    server node4 10.0.0.13:9001 check backup
```

### Vertical Scaling

**Resource Optimization:**
```json
{
  "performance": {
    "worker_threads": 16,
    "max_concurrent_requests": 4000,
    "query_timeout": 60000
  },
  "storage": {
    "cache_size": 4294967296
  }
}
```

### Database Sharding

**Shard Configuration:**
```json
{
  "sharding": {
    "enabled": true,
    "strategy": "schema_based",
    "shards": {
      "user_data": ["node-01", "node-02"],
      "analytics": ["node-03", "node-04"],
      "logs": ["node-05", "node-06"]
    }
  }
}
```

## Security Configuration

### TLS/SSL Setup

**Certificate Generation:**
```bash
# Generate CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 3650 -key ca.key -out ca.crt

# Generate server certificate
openssl genrsa -out server.key 4096
openssl req -new -key server.key -out server.csr
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key -out server.crt
```

**TLS Configuration:**
```json
{
  "security": {
    "tls": {
      "enabled": true,
      "cert_file": "/etc/ssl/certs/folddb.crt",
      "key_file": "/etc/ssl/private/folddb.key",
      "ca_file": "/etc/ssl/certs/ca.crt",
      "protocols": ["TLSv1.2", "TLSv1.3"],
      "ciphers": "HIGH:!aNULL:!MD5",
      "verify_client": true
    }
  }
}
```

### Firewall Configuration

**UFW Rules:**
```bash
# Allow SSH
sudo ufw allow 22/tcp

# Allow Fold DB ports
sudo ufw allow 9000/tcp  # P2P network
sudo ufw allow 9001/tcp  # HTTP API

# Allow monitoring
sudo ufw allow 9090/tcp  # Metrics
sudo ufw allow 9091/tcp  # Health check

# Enable firewall
sudo ufw enable
```

**iptables Rules:**
```bash
# Allow established connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow loopback
iptables -A INPUT -i lo -j ACCEPT

# Allow SSH
iptables -A INPUT -p tcp --dport 22 -j ACCEPT

# Allow Fold DB
iptables -A INPUT -p tcp --dport 9000:9001 -j ACCEPT

# Drop all other traffic
iptables -A INPUT -j DROP
```

## Troubleshooting

### Common Issues

**Node Won't Start:**
```bash
# Check configuration
./datafold_node --config config.json --validate

# Check permissions
ls -la /data/folddb
sudo chown -R folddb:folddb /data/folddb

# Check ports
netstat -tlnp | grep 900
```

**Network Connectivity Issues:**
```bash
# Check network status
curl http://localhost:9001/api/network/status

# Test peer connectivity
telnet peer-address 9000

# Check firewall
sudo ufw status
```

**Performance Issues:**
```bash
# Check system resources
top
htop
iotop

# Check database stats
curl http://localhost:9001/api/stats

# Analyze logs
grep -i "slow\|timeout\|error" /var/log/folddb/folddb.log
```

**Schema Loading Failures:**
```bash
# Validate schema
curl -X POST http://localhost:9001/api/schema/validate \
  -H "Content-Type: application/json" \
  -d @schema.json

# Check schema conflicts
curl http://localhost:9001/api/schemas
```

### Diagnostic Commands

**System Health:**
```bash
# Node status
curl http://localhost:9001/api/status | jq

# Health check
curl http://localhost:9001/api/health | jq

# Metrics
curl http://localhost:9001/api/metrics

# Resource usage
curl http://localhost:9001/api/system/resources | jq
```

**Network Diagnostics:**
```bash
# Peer list
curl http://localhost:9001/api/network/peers | jq

# Connection status
curl http://localhost:9001/api/network/connections | jq

# Discovery status
curl http://localhost:9001/api/network/discovery | jq
```

### Recovery Procedures

**Database Recovery:**
```bash
# Stop service
sudo systemctl stop folddb

# Restore from backup
tar -xzf /backups/folddb/20240115_020000/folddb_backup.tar.gz -C /data/folddb/

# Start service
sudo systemctl start folddb
```

**Configuration Recovery:**
```bash
# Reset to default config
cp /etc/folddb/default.json /etc/folddb/production.json

# Restart service
sudo systemctl restart folddb
```

---

**Next**: See [Schema Management](./schema-management.md) for detailed schema system documentation.