# DataFold Node API Examples

This directory contains example scripts that demonstrate how to interact with the DataFold Node API. These examples show how to connect to a DataFold node, list available schemas, retrieve schema details, query data, and create new records.

## Prerequisites

- Python 3.x
- A running DataFold node (default port: 9000)

## Running the Examples

### 1. Start a DataFold Node

First, start a DataFold node using the following command:

```bash
cargo run --bin datafold_node --package fold_node
```

This will start a DataFold node on port 9000 by default.

### 2. List Available Schemas

To list all available schemas on the node, run:

```bash
python3 list_schemas.py
```

This script connects to the DataFold node and retrieves a list of all available schemas.

### 3. Get Schema Details

To get the details of a specific schema, run:

```bash
python3 get_schema.py
```

This script retrieves the details of the "user" schema, including its fields, permission policies, and payment configuration.

### 4. Query Data

To query data from a schema, run:

```bash
python3 query_data.py
```

This script queries the "user" schema for the username, full name, and bio fields of all users.

### 5. Create a New Record

To create a new record in a schema, run:

```bash
python3 create_user.py
```

This script creates a new user in the "user" schema with the specified data.

## Understanding the Code

Each example script follows the same pattern:

1. Establish a TCP connection to the DataFold node
2. Format a request as a JSON object
3. Send the request with a length prefix
4. Receive the response with a length prefix
5. Parse and display the response

The key part of each script is the `send_request` function, which handles the communication protocol:

```python
def send_request(sock, request):
    # Serialize the request to JSON
    request_json = json.dumps(request).encode('utf-8')
    
    # Send the length prefix
    sock.sendall(struct.pack('!I', len(request_json)))
    
    # Send the JSON payload
    sock.sendall(request_json)
    
    # Receive the response length
    response_len_bytes = sock.recv(4)
    response_len = struct.unpack('!I', response_len_bytes)[0]
    
    # Receive the response
    response_json = sock.recv(response_len)
    
    # Deserialize the response
    return json.loads(response_json.decode('utf-8'))
```

This function handles the binary protocol used by the DataFold node, which consists of a 4-byte length prefix followed by a JSON payload.

## Next Steps

These examples demonstrate the basic operations of the DataFold Node API. For more advanced usage, refer to the comprehensive API documentation in `cline_docs/datafold_node_api.md`.

You can extend these examples to:

- Query with filters
- Update existing records
- Delete records
- Work with other schemas
- Implement cross-node communication
- Add authentication and payment handling

## Additional Resources

- [DataFold Node API Documentation](../cline_docs/datafold_node_api.md)
- [DataFold Node Source Code](../fold_node/src/bin/datafold_node.rs)
- [DataFold Node TCP Server](../fold_node/src/datafold_node/tcp_server.rs)
