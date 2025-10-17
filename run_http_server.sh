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
echo "Installing frontend dependencies..."
npm ci

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

# Migrate schemas from available_schemas to the database
echo "Migrating schemas from available_schemas to database..."
python3 scripts/migrate_schemas_to_db.py --schemas-dir available_schemas --service-url http://127.0.0.1:9002

if [ $? -ne 0 ]; then
    echo "Warning: Schema migration had issues. Check output above."
    echo "Continuing with server startup..."
fi

# Run the HTTP server in the background with schema service URL
echo "Starting the HTTP server on port 9001 in the background..."
nohup cargo run --bin datafold_http_server -- --port 9001 --schema-service-url "http://127.0.0.1:9002" > server.log 2>&1 &

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
    echo "To stop both servers, run: kill $SCHEMA_SERVICE_PID $SERVER_PID"
    echo "To view server logs, run: tail -f server.log"
    echo "To view schema service logs, run: tail -f schema_service.log"
else
    echo "HTTP server failed to become healthy within 30 seconds. Check server.log for details."
    kill $SCHEMA_SERVICE_PID 2>/dev/null
    kill $SERVER_PID 2>/dev/null
    exit 1
fi