# DataFold TCP Server

The DataFold TCP Server is a component of the DataFold node that allows applications to communicate with the node using TCP sockets. This makes it easier to develop and test applications on different platforms, as it removes the need for Unix sockets or other platform-specific IPC mechanisms.

## Overview

The TCP server listens on a specified port (default: 9000) and accepts connections from clients. It handles requests in the same format as the DataFold node's API, making it compatible with the DataFold SDK.

## Features

- **TCP Socket Communication**: Allows applications to connect to the node using standard TCP sockets.
- **JSON-based Protocol**: Uses a simple JSON-based protocol for requests and responses.
- **Full API Support**: Supports all operations available in the DataFold node's API.
- **Concurrent Connections**: Handles multiple client connections concurrently.

## Usage

### Starting the Node with TCP Server

To start a DataFold node with the TCP server enabled:

```bash
cargo run --bin datafold_node -- --port 9876 --tcp-port 9000
```

Where:
- `--port` specifies the port for the P2P network (default: 9000)
- `--tcp-port` specifies the port for the TCP server (default: 9000)

### Connecting to the TCP Server

Applications can connect to the TCP server using standard TCP sockets. The DataFold SDK provides a `NodeConnection::TcpSocket` connection type that can be used to connect to the TCP server:

```rust
use datafold_sdk::{
    client::DataFoldClient,
    types::NodeConnection,
};

// Create a client with a TCP connection to the node
let connection = NodeConnection::TcpSocket("127.0.0.1".to_string(), 9000);
let client = DataFoldClient::with_connection(
    "my-app",
    "private-key-placeholder",
    "public-key-placeholder",
    connection,
);
```

## Protocol

The TCP server uses a simple protocol for communication:

1. **Request**: The client sends a request in the following format:
   - 4-byte length prefix (u32, little-endian)
   - JSON-encoded request body

2. **Response**: The server sends a response in the following format:
   - 4-byte length prefix (u32, little-endian)
   - JSON-encoded response body

### Request Format

```json
{
  "app_id": "my-app",
  "operation": "query",
  "params": {
    "schema": "user",
    "fields": ["id", "username", "full_name"],
    "filter": {
      "field": "username",
      "operator": "eq",
      "value": "alice"
    }
  },
  "signature": "signed-my-app-1234567890",
  "timestamp": 1234567890
}
```

### Response Format

```json
{
  "results": [
    {
      "id": "1",
      "username": "alice",
      "full_name": "Alice Johnson"
    }
  ],
  "errors": []
}
```

## Supported Operations

The TCP server supports the following operations:

- `list_schemas`: List schemas currently loaded
- `list_available_schemas`: List all schemas stored on disk
- `get_schema`: Get a schema by name
- `create_schema`: Create a new schema
- `unload_schema`: Unload a schema from memory
- `query`: Query data from a schema
- `mutation`: Mutate data in a schema
- `discover_nodes`: Discover remote nodes

## Implementation Details

The TCP server is implemented in the `src/datafold_node/tcp_server.rs` file. It uses Tokio for asynchronous I/O and handles each client connection in a separate task.

## Future Improvements

- **Authentication**: Add support for authenticating clients
- **TLS Support**: Add support for TLS encryption
- **WebSocket Support**: Add support for WebSocket connections
- **HTTP API**: Add support for HTTP API
