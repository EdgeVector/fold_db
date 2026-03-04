#!/bin/bash

# Script to run the FoldDB Tauri app in development mode
# This script starts the FoldDB server and opens the native app

set -e

echo "🚀 Starting FoldDB Native App (Development Mode)"
echo "=================================================="

# Check if Rust toolchain is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust toolchain not found. Please install Rust first."
    echo "   Visit: https://rustup.rs/"
    exit 1
fi

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js not found. Please install Node.js first."
    echo "   Visit: https://nodejs.org/"
    exit 1
fi

# Navigate to the React project directory
cd src/server/static-react

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    npm ci
fi

# Build the frontend first
echo "🔨 Building frontend..."
npm run build

# Check if Tauri CLI is available
if ! command -v tauri &> /dev/null; then
    echo "📦 Installing Tauri CLI..."
    npm install -g @tauri-apps/cli
fi

# Run Tauri in dev mode
echo "🎯 Starting Tauri app..."
echo ""
echo "The app will open in a native window."
echo "The FoldDB server will start automatically on port 9001."
echo "Data will be stored in: ~/.folddb/data"
echo ""
echo "Press Ctrl+C to stop the app."
echo ""

npm run tauri:dev

