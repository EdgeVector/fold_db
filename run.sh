#!/bin/bash

set -e
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
        # --table-name is deprecated/removed to prevent temporary table creation
        # --table-name=*)
        #     echo "WARNING: --table-name is disabled. Using default 'DataFoldStorage'."
        #     shift
        #     ;;
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

# Enforce strict table naming to prevent temporary table creation
if [ "$TABLE_NAME" != "DataFoldStorage" ]; then
    echo "WARNING: TABLE_NAME was modified. Resetting to 'DataFoldStorage' to prevent temporary table creation."
    TABLE_NAME="DataFoldStorage"
fi

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

echo "Ensuring node identity..."
# Run ensure_identity to get the public key
USER_ID=$(cargo run --quiet --bin ensure_identity)
echo "Node Identity (User ID): $USER_ID"

echo "Setting up Cloud configuration..."
echo "Table name: $TABLE_NAME"
echo "Region: $REGION"
if [ -n "$USER_ID" ]; then
    echo "User ID: $USER_ID"
fi

# Create or update node_config.json with DynamoDB settings
cat > "$CONFIG_FILE" <<EOF
{
  "database": {
    "type": "cloud",
    "region": "$REGION",
    "tables": {
      "main": "${TABLE_NAME}-main",
      "metadata": "${TABLE_NAME}-metadata",
      "permissions": "${TABLE_NAME}-node_id_schema_permissions",
      "transforms": "${TABLE_NAME}-transforms",
      "orchestrator": "${TABLE_NAME}-orchestrator_state",
      "schema_states": "${TABLE_NAME}-schema_states",
      "schemas": "${TABLE_NAME}-schemas",
      "public_keys": "${TABLE_NAME}-public_keys",
      "transform_queue": "${TABLE_NAME}-transform_queue_tree",
      "native_index": "${TABLE_NAME}-native_index",
      "process": "${TABLE_NAME}-process",
      "logs": "${TABLE_NAME}-logs"
    },
    "auto_create": true,
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
  "schema_service_url": "https://schema.folddb.com"
}
EOF

echo "Cloud configuration saved to $CONFIG_FILE"

# Build the Rust project first (needed to generate OpenAPI spec)
echo "Building the Rust project..."
cargo build --features aws-backend



# Generate OpenAPI spec to a local file for the UI prebuild
echo "Generating OpenAPI spec..."
mkdir -p target
cargo run --features aws-backend --quiet --bin openapi_dump > target/openapi.json



# Build the React frontend (prebuild will read OPENAPI_URL file)
echo "Building the React frontend..."
cd src/server/static-react

# Only install if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "Installing frontend dependencies..."
    npm install
else
    echo "Frontend dependencies already installed, skipping npm install"
fi

OPENAPI_URL="file://$PWD/../../../target/openapi.json" npm run build



# Go back to root directory
cd ../../..

# Using the global schema service at schema.folddb.com
SCHEMA_SERVICE_URL="https://schema.folddb.com"
echo "Using global schema service at: $SCHEMA_SERVICE_URL"

# Export DynamoDB config for ProgressStore (uses prefix to generate table names)
export DATAFOLD_DYNAMODB_TABLE_PREFIX="$TABLE_NAME"
export DATAFOLD_DYNAMODB_REGION="$REGION"
if [ -n "$USER_ID" ]; then
    export DATAFOLD_DYNAMODB_USER_ID="$USER_ID"
fi

# Verify global schema service is reachable
echo "Checking schema service connectivity..."
if curl -s --connect-timeout 5 "$SCHEMA_SERVICE_URL" > /dev/null 2>&1; then
    echo "Schema service is reachable."
else
    echo "WARNING: Global schema service at $SCHEMA_SERVICE_URL may not be reachable."
    echo "         The server will still start, but schema operations may fail."
fi

# Run the HTTP server in the background with schema service URL
echo "Starting the HTTP server on port 9001 with Cloud backend..."
echo "Note: Cloud/DynamoDB tables will be created automatically if they don't exist."
echo "Make sure AWS credentials are configured (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, or IAM role)"

# Debug: Print AWS Credential Status

# Load shell profile to get API keys
source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true

# Ensure FOLD_OPENROUTER_API_KEY is set (check common variable names)
if [ -z "$FOLD_OPENROUTER_API_KEY" ]; then
    if [ -n "$OPENROUTER_API_KEY" ]; then
        export FOLD_OPENROUTER_API_KEY="$OPENROUTER_API_KEY"
    fi
fi

if [ -z "$FOLD_OPENROUTER_API_KEY" ]; then
    echo "WARNING: FOLD_OPENROUTER_API_KEY not set. Ingestion will not work."
    echo "         Set it in your shell profile: export FOLD_OPENROUTER_API_KEY=your_key"
else
    # Ensure it's exported for child processes
    export FOLD_OPENROUTER_API_KEY
    echo "OpenRouter API key configured"
fi


# Server is now stateless - user identity comes from X-User-Hash header per request
RUST_LOG=debug nohup cargo run --features aws-backend --bin datafold_http_server -- --port 9001 --schema-service-url "$SCHEMA_SERVICE_URL" > server.log 2>&1 &

# Get the process ID
SERVER_PID=$!

# Wait for HTTP server to be healthy with proper health check
echo "Waiting for HTTP server to be ready..."
HTTP_READY=false
for i in {1..180}; do
    if kill -0 $SERVER_PID 2>/dev/null; then
        if curl -s http://127.0.0.1:9001/api/system/status > /dev/null 2>&1; then
            HTTP_READY=true
            break
        fi
        sleep 1
    else
        echo "HTTP server process died. Check server.log for details."
        exit 1
    fi
done

if [ "$HTTP_READY" = true ]; then
    echo "HTTP server started successfully with PID: $SERVER_PID"
    echo "Server logs are being written to: server.log"
    echo "Cloud configuration:"
    echo "  Table name: $TABLE_NAME"
    echo "  Region: $REGION"
    if [ -n "$USER_ID" ]; then
        echo "  User ID: $USER_ID"
    fi
    echo "  Schema Service: $SCHEMA_SERVICE_URL"
    echo ""
    echo "To stop the server, run: kill $SERVER_PID"
    echo "To view server logs, run: tail -f server.log"
    echo ""
    echo "Note: DynamoDB tables will be created automatically on first use."
else
    echo "HTTP server failed to become healthy within 60 seconds. Check server.log for details."
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

