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

# Optionally reset the database from test_db template
reset_db() {
    echo "Resetting database from test_db template..."
    rm -rf data
    cp -R test_db data
    echo "Database reset complete."
}

# Optionally start with an empty database directory
empty_db() {
    echo "Initializing empty database directory..."
    rm -rf data
    mkdir -p data
    echo "Empty database directory ready."
}

# Parse flags
RESET_DB=false
EMPTY_DB=false
LOCAL_SCHEMA=false
for arg in "$@"; do
    case "$arg" in
        --reset-db)
            RESET_DB=true
            shift
            ;;
        --empty-db)
            EMPTY_DB=true
            shift
            ;;
        --local-schema)
            LOCAL_SCHEMA=true
            shift
            ;;
        *)
            ;;
    esac
done

# Clean up any existing locks and processes
cleanup_locks

# Reset DB if requested
if [ "$RESET_DB" = true ]; then
    reset_db
fi

if [ "$EMPTY_DB" = true ]; then
    empty_db
fi

# Ensure local configuration (override any cloud config from run.sh)
echo "Setting up local configuration..."
CONFIG_FILE="config/node_config.json"
mkdir -p config
cat > "$CONFIG_FILE" <<EOF
{
  "database": {
    "type": "local",
    "path": "data"
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
echo "Local configuration saved to $CONFIG_FILE"

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

# Ensure frontend dependencies are installed
cd src/server/static-react
if [ ! -d "node_modules" ]; then
    echo "Installing frontend dependencies..."
    npm install
    if [ $? -ne 0 ]; then
        echo "Failed to install frontend dependencies. Exiting."
        exit 1
    fi
fi
cd ../../..

# Schema Service Configuration
# By default, use the global schema service at schema.folddb.com
# Use --local-schema flag to run a local schema service for testing/offline development
SCHEMA_SERVICE_URL="https://schema.folddb.com"

if [ "$LOCAL_SCHEMA" = true ]; then
    SCHEMA_SERVICE_URL="http://127.0.0.1:9002"
    
    # Start the schema service
    echo "Starting LOCAL schema service on port 9002 in the background..."
    nohup cargo run --bin schema_service -- --port 9002 --db-path schema_registry > schema_service.log 2>&1 &

    # Get the schema service process ID
    SCHEMA_SERVICE_PID=$!

    # Wait for schema service to be healthy with proper health check
    echo "Waiting for local schema service to be ready..."
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
        echo "Local schema service started successfully with PID: $SCHEMA_SERVICE_PID"
        echo "Schema service logs are being written to: schema_service.log"
    else
        echo "Local schema service failed to become healthy within 30 seconds. Check schema_service.log for details."
        kill $SCHEMA_SERVICE_PID 2>/dev/null
        exit 1
    fi
else
    echo "Using global schema service at: $SCHEMA_SERVICE_URL"
    
    # Verify global schema service is reachable
    echo "Checking schema service connectivity..."
    if curl -s --connect-timeout 5 "$SCHEMA_SERVICE_URL" > /dev/null 2>&1; then
        echo "Schema service is reachable."
    else
        echo "WARNING: Global schema service at $SCHEMA_SERVICE_URL may not be reachable."
        echo "         Consider using --local-schema flag for offline development."
        echo "         The server will still start, but schema operations may fail."
    fi
fi

# Run the HTTP server in the background with schema service URL
echo "Starting the HTTP server on port 9001 in the background..."
# Export OPENROUTER_API_KEY if set in .zshrc
source ~/.zshrc 2>/dev/null || true
nohup cargo run --bin datafold_http_server -- --port 9001 --schema-service-url "$SCHEMA_SERVICE_URL" > server.log 2>&1 &

# Get the process ID
SERVER_PID=$!

# Wait for HTTP server to be healthy with proper health check
echo "Waiting for HTTP server to be ready..."
HTTP_READY=false
for i in {1..30}; do
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
    echo "Schema service: $SCHEMA_SERVICE_URL"
    echo ""
    echo "Starting Vite dev server with hot reload..."
    echo "Access app at: http://localhost:5173"
    echo ""

    # Start Vite dev server (foreground for hot reload)
    cd src/server/static-react
    npm run dev

    # Cleanup when Vite exits
    kill $SERVER_PID 2>/dev/null || true
    if [ "$LOCAL_SCHEMA" = true ]; then
        kill $SCHEMA_SERVICE_PID 2>/dev/null || true
    fi
else
    echo "HTTP server failed to become healthy within 30 seconds. Check server.log for details."
    if [ "$LOCAL_SCHEMA" = true ]; then
        kill $SCHEMA_SERVICE_PID 2>/dev/null
    fi
    kill $SERVER_PID 2>/dev/null
    exit 1
fi