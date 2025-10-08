#!/bin/bash

# Function to kill existing datafold processes and clean up locks
cleanup_locks() {
    echo "Checking for existing datafold processes..."
    
    # Kill any existing datafold processes
    pkill -f datafold_http_server 2>/dev/null || true
    pkill -f "cargo run.*datafold_http_server" 2>/dev/null || true
    
    # Wait a moment for processes to terminate
    sleep 2
    
    # Force kill if still running
    pkill -9 -f datafold_http_server 2>/dev/null || true
    pkill -9 -f "cargo run.*datafold_http_server" 2>/dev/null || true
    
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

# Run the HTTP server in the background
echo "Starting the HTTP server on port 9001 in the background..."
nohup cargo run --bin datafold_http_server -- --port 9001 > server.log 2>&1 &

# Get the process ID
SERVER_PID=$!

# Wait a moment to check if the server started successfully
sleep 3

# Check if the process is still running
if kill -0 $SERVER_PID 2>/dev/null; then
    echo "HTTP server started successfully with PID: $SERVER_PID"
    echo "Server logs are being written to: server.log"
    echo "To stop the server, run: kill $SERVER_PID"
    echo "To view logs, run: tail -f server.log"
else
    echo "Failed to start HTTP server. Check server.log for details."
    exit 1
fi