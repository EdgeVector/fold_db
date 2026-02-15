#!/bin/bash

# Script to build the FoldDB macOS native app
# This creates a production-ready .app bundle

set -e

echo "🍎 Building FoldDB macOS Native App"
echo "======================================"

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

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf dist
rm -rf src-tauri/target/release

# Install dependencies
echo "📦 Installing dependencies..."
npm ci

# Build the frontend
echo "🔨 Building frontend..."
npm run build

# Build the Tauri app
echo "🔧 Building Tauri app (this may take a while)..."
npm run tauri:build

# Report success
echo ""
echo "✅ Build complete!"
echo ""
echo "📦 Your macOS app is ready:"
echo "   Location: src/server/static-react/src-tauri/target/release/bundle/macos/"
echo ""
echo "To install the app:"
echo "   1. Navigate to the location above"
echo "   2. Copy FoldDB.app to your Applications folder"
echo "   3. Open FoldDB from Applications"
echo ""
echo "To create a DMG installer:"
echo "   The .dmg file is also in the same directory"
echo ""

