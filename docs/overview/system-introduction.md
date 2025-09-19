# DataFold System Introduction

Welcome to DataFold! This document provides a comprehensive introduction to the system structure for new team members, developers, and stakeholders.

## 🎯 What is DataFold?

DataFold is a **distributed, event-driven data platform** built in Rust that provides:

- **Schema-based data storage** with automatic validation
- **AI-powered data ingestion** and schema generation
- **Real-time data processing** through programmable transforms
- **Peer-to-peer networking** for distributed operations
- **Fine-grained access control** with trust-based permissions
- **High-performance storage** using embedded databases

## 🏗️ High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        DataFold Platform                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   Web UI    │  │   HTTP API  │  │   CLI Tool  │            │
│  │ (React App) │  │   Server    │  │             │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
├─────────────────────────────────────────────────────────────────┤
│                    Core DataFold Node                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   Schema    │  │   Transform │  │   Storage   │            │
│  │ Management  │  │    Engine   │  │   Engine    │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │  Ingestion  │  │  Security   │  │  Network    │            │
│  │    Core     │  │   Layer     │  │   Layer     │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

## 🔧 Core Components

### 1. **DataFold Node** (`src/datafold_node/`)
The central component that orchestrates all operations:

- **Configuration Management**: Node settings, storage paths, network config
- **Database Operations**: Core CRUD operations for atoms (data units)
- **Event Processing**: Handles field changes and triggers transforms
- **Schema Management**: Loads, validates, and enforces schemas

**Key Files:**
- `src/datafold_node/mod.rs` - Main node implementation
- `src/datafold_node/config.rs` - Configuration management
- `src/datafold_node/db.rs` - Database interface

### 2. **Schema System** (`src/schema/`)
Defines data structure, validation rules, and behavior:

- **Field Types**: Single values, collections, and range fields
- **Permission Policies**: Read/write access control
- **Payment Configuration**: Fee structures for data access
- **Transform Rules**: Automatic computation logic

**Key Files:**
- `src/schema/core.rs` - Core schema functionality
- `src/schema/types/` - Field type definitions
- `src/schema/discovery.rs` - Schema detection and management

### 3. **Transform Engine** (`src/transform/`)
Programmable computation system with custom DSL:

- **AST Parser**: Parses transform expressions
- **Interpreter**: Executes transform logic
- **Built-in Functions**: Math, conversions, and utilities
- **Event Triggers**: Automatic execution on data changes

**Key Files:**
- `src/transform/ast.rs` - Abstract syntax tree
- `src/transform/interpreter/` - Execution engine
- `src/transform/parser/` - Language parsing

### 4. **Storage Engine** (`src/fold_db_core/`)
High-performance embedded database layer:

- **Sled Database**: Rust-native key-value store
- **Atom Management**: Immutable data units with versioning
- **Event Bus**: Asynchronous event processing
- **Orchestration**: Coordinates complex operations

**Key Files:**
- `src/fold_db_core/infrastructure/` - Core infrastructure
- `src/fold_db_core/managers/` - Component managers
- `src/fold_db_core/services/` - Core services

### 5. **Ingestion System** (`src/ingestion/`)
AI-powered data processing and schema generation:

- **JSON Processing**: Handles incoming data streams
- **Schema Inference**: Automatic field detection
- **Field Mapping**: Intelligent data organization
- **Pipeline Management**: Extensible processing workflows

**Key Files:**
- `src/ingestion/core.rs` - Main ingestion logic
- `src/ingestion/config.rs` - Configuration management

### 6. **Security Layer** (`src/security/`)
Comprehensive security and access control:

- **Cryptography**: Encryption, hashing, and key management
- **Permission System**: Field-level access control
- **Audit Logging**: Complete operation tracking
- **Trust Management**: Node-to-node authentication

**Key Files:**
- `src/security/encryption.rs` - Encryption utilities
- `src/security/keys.rs` - Key management
- `src/security/audit.rs` - Audit logging

### 7. **Network Layer** (`src/network/`)
Peer-to-peer networking and discovery:

- **LibP2P Integration**: Modern P2P networking
- **Peer Discovery**: Automatic node finding
- **Connection Management**: Reliable communication
- **Message Routing**: Efficient data distribution

**Key Files:**
- `src/network/core.rs` - Network core functionality
- `src/network/connections.rs` - Connection management

## 🌐 User Interfaces

### **Web UI** (`src/datafold_node/static-react/`)
Modern React-based web interface:

- **Schema Management**: Create, edit, and manage schemas
- **Data Querying**: Interactive data exploration
- **Transform Editor**: Visual transform creation
- **User Management**: Authentication and permissions

**Key Components:**
- `src/components/` - React components
- `src/hooks/` - Custom React hooks
- `src/store/` - State management (Redux Toolkit)

### **HTTP API Server** (`src/bin/datafold_http_server.rs`)
RESTful API for programmatic access:

- **Schema Operations**: CRUD operations for schemas
- **Data Operations**: Query, insert, update, delete
- **Transform Execution**: Run transforms and computations
- **Authentication**: Secure API access

### **CLI Tool** (`src/bin/datafold_cli.rs`)
Command-line interface for system administration:

- **Node Management**: Start, stop, configure nodes
- **Schema Operations**: Import/export schemas
- **Data Operations**: Bulk data operations
- **System Monitoring**: Health checks and diagnostics

## 📊 Data Flow

### 1. **Data Ingestion Flow**
```
External Data → Ingestion Core → Schema Inference → Storage Engine → Event Bus
                                    ↓
                              Transform Engine → Updated Data
```

### 2. **Query Processing Flow**
```
Client Request → HTTP API → Permission Check → Query Engine → Storage → Response
                                    ↓
                              Transform Execution (if needed)
```

### 3. **Event Processing Flow**
```
Field Change → Event Bus → Transform Trigger → Transform Engine → Storage Update
                                    ↓
                              Notification to Subscribers
```

## 🔑 Key Concepts

### **Atoms**
- Immutable data units with unique UUIDs
- Version history for all changes
- Atomic operations with consistency guarantees

### **Schemas**
- JSON-based data definitions
- Field-level permissions and payment config
- Transform rules for automatic computation

### **Transforms**
- Domain-specific language for data processing
- Event-driven execution
- Built-in functions and custom logic

### **Trust Distance**
- Permission-based access control
- Configurable trust levels between nodes
- Secure peer-to-peer communication

## 🚀 Getting Started

### **For Developers**
1. **Setup Environment**: Install Rust and dependencies
2. **Run Tests**: `cargo test` to verify installation
3. **Start Node**: `cargo run --bin datafold_node`
4. **Explore UI**: `cargo run --bin datafold_http_server`

### **For Users**
1. **Install CLI**: `cargo install datafold`
2. **Start Server**: `datafold_http_server --port 9001`
3. **Access Web UI**: Visit `http://localhost:9001`
4. **Create Schema**: Use the web interface to define data structures

### **For System Administrators**
1. **Review Configuration**: Check `config.toml` files
2. **Setup Storage**: Configure persistent storage paths
3. **Network Setup**: Configure P2P networking settings
4. **Security**: Setup encryption keys and permissions

## 📚 Documentation Structure

- **`docs/overview/architecture.md`** - Detailed technical architecture
- **`docs/guides/development/developer-guide.md`** - Integration and development guide
- **`docs/reference/api-reference.md`** - Complete API documentation
- **`docs/guides/operations/deployment-guide.md`** - Production deployment guide
- **`docs/overview/use-cases.md`** - Common use cases and examples

## 🔍 Project Organization

```
datafold/
├── src/                    # Rust source code
│   ├── bin/               # Executable binaries
│   ├── datafold_node/     # Core node implementation
│   ├── schema/            # Schema management
│   ├── transform/         # Transform engine
│   ├── fold_db_core/      # Storage engine
│   ├── ingestion/         # Data ingestion
│   ├── security/          # Security layer
│   └── network/           # Networking layer
├── docs/                  # Documentation
├── tests/                 # Test suites
├── available_schemas/     # Example schemas
└── scripts/               # Utility scripts
```

## 🤝 Contributing

### **Development Workflow**
1. **Create Issue**: Describe the problem or feature
2. **Fork Repository**: Create your development branch
3. **Implement Changes**: Follow Rust best practices
4. **Run Tests**: Ensure all tests pass
5. **Submit PR**: Create pull request with description

### **Code Standards**
- **Rust**: Follow Rust coding standards and idioms
- **Documentation**: Document all public APIs
- **Testing**: Include unit and integration tests
- **Error Handling**: Use proper error types and handling

## 🆘 Getting Help

### **Resources**
- **GitHub Issues**: Report bugs and request features
- **Documentation**: Comprehensive guides and references
- **Examples**: Sample code and use cases
- **Community**: Developer discussions and support

### **Common Issues**
- **Compilation Errors**: Check Rust version and dependencies
- **Runtime Errors**: Review logs and configuration
- **Performance Issues**: Profile and optimize bottlenecks
- **Network Issues**: Verify P2P configuration

---

**Welcome to DataFold!** This system represents a new approach to distributed data management, combining the power of Rust with modern distributed systems principles. Take your time exploring the components, and don't hesitate to ask questions or contribute to the project.

For more detailed information, explore the specific documentation files mentioned above, or dive into the source code to understand how everything works together.
