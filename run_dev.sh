#!/bin/bash

# Development mode with hot reloading
# - Rust backend on port 9001
# - Vite dev server on port 5173 (with HMR)
# - Access app at http://localhost:5173

set -e

# Kill existing processes
pkill -f folddb_server 2>/dev/null || true
pkill -f "vite" 2>/dev/null || true
sleep 1

# Ensure node_modules exists
cd src/server/static-react
if [ ! -d "node_modules" ]; then
    echo "Installing frontend dependencies..."
    npm install
fi
cd ../../..

# Start Rust backend in background
echo "Starting Rust backend on port 9001..."
cargo run --bin folddb_server -- --port 9001 &
BACKEND_PID=$!

# Wait for backend to be ready
echo "Waiting for backend..."
for i in {1..30}; do
    if curl -s http://localhost:9001/api/system/status > /dev/null 2>&1; then
        echo "Backend ready!"
        break
    fi
    sleep 1
done

# Start Vite dev server (foreground for hot reload output)
echo ""
echo "Starting Vite dev server with hot reload..."
echo "Access app at: http://localhost:5173"
echo ""
cd src/server/static-react
npm run dev

# Cleanup on exit
kill $BACKEND_PID 2>/dev/null || true
