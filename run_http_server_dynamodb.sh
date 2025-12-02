#!/bin/bash

# Function to kill existing datafold processes and clean up locks
cleanup_locks() {
    echo "Checking for existing datafold processes..."
    
    # Kill any existing datafold processes
    pkill -f datafold_http_server 2>/dev/null || true
    pkill -f "cargo run.*datafold_http_server" 2>/dev/null || true
    pkill -f schema_service 2>/dev/null || true
    pkill -f "cargo run.*schema_service" 2>/dev/null || true
    
    # Wait a moment for processes to terminate
    sleep 2
    
    # Force kill if still running
    pkill -9 -f datafold_http_server 2>/dev/null || true
    pkill -9 -f "cargo run.*datafold_http_server" 2>/dev/null || true
    pkill -9 -f schema_service 2>/dev/null || true
    pkill -9 -f "cargo run.*schema_service" 2>/dev/null || true
    
    echo "Cleaned up existing processes."
}

# Parse flags
TABLE_NAME="DataFoldStorage"
REGION="us-west-2"
USER_ID=""
for arg in "$@"; do
    case "$arg" in
        --table-name=*)
            TABLE_NAME="${arg#*=}"
            shift
            ;;
        --region=*)
            REGION="${arg#*=}"
            shift
            ;;
        --user-id=*)
            USER_ID="${arg#*=}"
            shift
            ;;
        *)
            ;;
    esac
done

# Clean up any existing locks and processes
cleanup_locks

# Backup existing config if it exists
CONFIG_FILE="config/node_config.json"
if [ -f "$CONFIG_FILE" ]; then
    echo "Backing up existing node_config.json..."
    cp "$CONFIG_FILE" "${CONFIG_FILE}.backup"
fi

# Ensure config directory exists
mkdir -p config

# Create DynamoDB configuration
echo "Setting up DynamoDB configuration..."
echo "Table name: $TABLE_NAME"
echo "Region: $REGION"
if [ -n "$USER_ID" ]; then
    echo "User ID: $USER_ID"
fi

# Create or update node_config.json with DynamoDB settings
cat > "$CONFIG_FILE" <<EOF
{
  "database": {
    "type": "dynamodb",
    "table_name": "$TABLE_NAME",
    "region": "$REGION",
    "user_id": $(if [ -n "$USER_ID" ]; then echo "\"$USER_ID\""; else echo "null"; fi)
  },
  "storage_path": "data",
  "default_trust_distance": 1,
  "network_listen_address": "/ip4/0.0.0.0/tcp/0",
  "security_config": {
    "require_tls": false,
    "require_signatures": false,
    "encrypt_at_rest": false
  },
  "schema_service_url": "http://127.0.0.1:9002"
}
EOF

echo "DynamoDB configuration saved to $CONFIG_FILE"

# Build the Rust project first (needed to generate OpenAPI spec)
echo "Building the Rust project..."
cargo build

if [ $? -ne 0 ]; then
    echo "Rust build failed. Exiting."
    exit 1
fi

# Generate OpenAPI spec to a local file for the UI prebuild
echo "Generating OpenAPI spec..."
mkdir -p target
cargo run --quiet --bin openapi_dump > target/openapi.json

if [ $? -ne 0 ]; then
    echo "Failed to generate OpenAPI spec. Exiting."
    exit 1
fi

# Build the React frontend (prebuild will read OPENAPI_URL file)
echo "Building the React frontend..."
cd src/datafold_node/static-react

# Clean up node_modules if it exists to avoid ENOTEMPTY errors
if [ -d "node_modules" ]; then
    echo "Cleaning up existing node_modules..."
    rm -rf node_modules
fi

# Remove package-lock.json if it exists to ensure a clean install
if [ -f "package-lock.json" ]; then
    echo "Removing package-lock.json for clean install..."
    rm -f package-lock.json
fi

echo "Installing frontend dependencies..."
npm install

if [ $? -ne 0 ]; then
    echo "Failed to install frontend dependencies. Exiting."
    exit 1
fi

OPENAPI_URL="file://$PWD/../../../target/openapi.json" npm run build

if [ $? -ne 0 ]; then
    echo "React build failed. Exiting."
    exit 1
fi

# Go back to root directory
cd ../../..

# Start the schema service first
echo "Starting the schema service on port 9002 in the background..."
nohup cargo run --bin schema_service -- --port 9002 --db-path schema_registry > schema_service.log 2>&1 &

# Get the schema service process ID
SCHEMA_SERVICE_PID=$!

# Wait for schema service to be healthy with proper health check
echo "Waiting for schema service to be ready..."
SCHEMA_READY=false
for i in {1..30}; do
    if kill -0 $SCHEMA_SERVICE_PID 2>/dev/null; then
        if curl -s http://127.0.0.1:9002/api/health > /dev/null 2>&1; then
            SCHEMA_READY=true
            break
        fi
        sleep 1
    else
        echo "Schema service process died. Check schema_service.log for details."
        exit 1
    fi
done

if [ "$SCHEMA_READY" = true ]; then
    echo "Schema service started successfully with PID: $SCHEMA_SERVICE_PID"
    echo "Schema service logs are being written to: schema_service.log"
else
    echo "Schema service failed to become healthy within 30 seconds. Check schema_service.log for details."
    kill $SCHEMA_SERVICE_PID 2>/dev/null
    exit 1
fi

echo "Schema migration is disabled. Schema service will start with an empty database."

# Run the HTTP server in the background with schema service URL
echo "Starting the HTTP server on port 9001 with DynamoDB backend..."
echo "Note: DynamoDB tables will be created automatically if they don't exist."
echo "Make sure AWS credentials are configured (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, or IAM role)"

# Export OPENROUTER_API_KEY if set in .zshrc
source ~/.zshrc 2>/dev/null || true
nohup cargo run --bin datafold_http_server -- --port 9001 --schema-service-url "http://127.0.0.1:9002" > server.log 2>&1 &

# Get the process ID
SERVER_PID=$!

# Wait for HTTP server to be healthy with proper health check
echo "Waiting for HTTP server to be ready..."
HTTP_READY=false
for i in {1..60}; do
    if kill -0 $SERVER_PID 2>/dev/null; then
        if curl -s http://127.0.0.1:9001/api/system/status > /dev/null 2>&1; then
            HTTP_READY=true
            break
        fi
        sleep 1
    else
        echo "HTTP server process died. Check server.log for details."
        kill $SCHEMA_SERVICE_PID 2>/dev/null
        exit 1
    fi
done

if [ "$HTTP_READY" = true ]; then
    echo "HTTP server started successfully with PID: $SERVER_PID"
    echo "Server logs are being written to: server.log"
    echo "DynamoDB configuration:"
    echo "  Table name: $TABLE_NAME"
    echo "  Region: $REGION"
    if [ -n "$USER_ID" ]; then
        echo "  User ID: $USER_ID"
    fi
    echo ""
    echo "To stop both servers, run: kill $SCHEMA_SERVICE_PID $SERVER_PID"
    echo "To view server logs, run: tail -f server.log"
    echo "To view schema service logs, run: tail -f schema_service.log"
    echo ""
    echo "Note: DynamoDB tables will be created automatically on first use."
else
    echo "HTTP server failed to become healthy within 60 seconds. Check server.log for details."
    kill $SCHEMA_SERVICE_PID 2>/dev/null
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

