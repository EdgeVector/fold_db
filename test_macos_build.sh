#!/bin/bash

# Local macOS Build Test Script
# This script mimics what the CI workflow does to build the macOS app
# Run this to test the build locally before pushing a release tag

set -euo pipefail

echo "========================================"
echo "DataFold macOS Build Test"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    ARCH="aarch64"
fi

echo -e "${YELLOW}Building for architecture: $ARCH${NC}"
echo ""

# Step 1: Check Cargo.toml for proper version format
echo "Step 1: Checking Cargo.toml plugin versions..."
cd src/datafold_node/static-react/src-tauri
if grep -q 'tauri-plugin-log = "2.0"' Cargo.toml || grep -q 'tauri-plugin-shell = "2.0"' Cargo.toml; then
    echo -e "${RED}ERROR: Found improperly formatted plugin versions in Cargo.toml${NC}"
    echo "Plugin versions must be in full semver format (e.g., '2.0.0', not '2.0')"
    exit 1
fi
echo -e "${GREEN}✓ Plugin versions look good${NC}"
echo ""

# Return to repo root
cd ../../../..

# Step 2: Verify Cargo dependencies
echo "Step 2: Verifying Cargo dependencies..."
cd src/datafold_node/static-react/src-tauri
if ! cargo check 2>&1; then
    echo -e "${RED}ERROR: Cargo check failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Cargo dependencies verified${NC}"
echo ""

# Return to frontend directory
cd ..

# Step 3: Install npm dependencies
echo "Step 3: Installing npm dependencies..."
if ! npm ci; then
    echo -e "${RED}ERROR: npm ci failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ npm dependencies installed${NC}"
echo ""

# Step 4: Build frontend
echo "Step 4: Building frontend..."
if ! npm run build; then
    echo -e "${RED}ERROR: Frontend build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Frontend built successfully${NC}"
echo ""

# Step 5: Build Tauri app
echo "Step 5: Building Tauri app (this may take several minutes)..."
if ! npm run tauri:build 2>&1 | tee /tmp/tauri_build.log; then
    echo -e "${RED}ERROR: Tauri build failed${NC}"
    echo "Check /tmp/tauri_build.log for details"
    exit 1
fi
echo -e "${GREEN}✓ Tauri build completed${NC}"
echo ""

# Step 6: Check for version parsing errors in output
echo "Step 6: Checking build output for errors..."
if grep -q "Failed to parse version" /tmp/tauri_build.log; then
    echo -e "${RED}ERROR: Found version parsing errors in build output${NC}"
    grep "Failed to parse version" /tmp/tauri_build.log
    exit 1
fi
echo -e "${GREEN}✓ No version parsing errors found${NC}"
echo ""

# Step 7: Check for build artifacts
echo "Step 7: Verifying build artifacts..."
BUNDLE_DIR="src-tauri/target/release/bundle"

echo "Bundle directory contents:"
ls -la "$BUNDLE_DIR" || true
echo ""

if [ -d "$BUNDLE_DIR/dmg" ]; then
    echo "DMG directory contents:"
    ls -la "$BUNDLE_DIR/dmg"
    echo ""
fi

if [ -d "$BUNDLE_DIR/macos" ]; then
    echo "macOS directory contents:"
    ls -la "$BUNDLE_DIR/macos"
    echo ""
fi

# Check for DMG
DMG_FOUND=false
if [ -d "$BUNDLE_DIR/dmg" ]; then
    for f in "$BUNDLE_DIR/dmg"/DataFold_*.dmg; do
        if [ -f "$f" ]; then
            echo -e "${GREEN}✓ Found DMG: $f${NC}"
            DMG_FOUND=true
            DMG_PATH="$f"
            DMG_SIZE=$(du -h "$f" | cut -f1)
            echo "  Size: $DMG_SIZE"
        fi
    done
fi

if [ "$DMG_FOUND" = false ]; then
    echo -e "${RED}✗ No DMG file found${NC}"
else
    echo ""
fi

# Check for .app
APP_FOUND=false
if [ -d "$BUNDLE_DIR/macos/DataFold.app" ]; then
    echo -e "${GREEN}✓ Found .app bundle: $BUNDLE_DIR/macos/DataFold.app${NC}"
    APP_FOUND=true
    APP_SIZE=$(du -sh "$BUNDLE_DIR/macos/DataFold.app" | cut -f1)
    echo "  Size: $APP_SIZE"
else
    echo -e "${RED}✗ No .app bundle found${NC}"
fi

echo ""

# Step 8: Test artifact preparation (mimics CI workflow)
echo "Step 8: Testing artifact preparation (CI workflow simulation)..."

# Go back to repo root
cd ../../..

TEST_VERSION="0.1.10-test"
WORKSPACE_ROOT="$(pwd)"
mkdir -p "$WORKSPACE_ROOT/dist-test"

cd "$BUNDLE_DIR"

# Test DMG copy
for f in dmg/DataFold_*.dmg; do
    if [ -f "$f" ]; then
        echo "Simulating DMG copy: $f -> dist-test/DataFold-$TEST_VERSION-$ARCH.dmg"
        cp "$f" "$WORKSPACE_ROOT/dist-test/DataFold-$TEST_VERSION-$ARCH.dmg"
        echo -e "${GREEN}✓ DMG copy successful${NC}"
    else
        echo -e "${YELLOW}⚠ No DMG to copy${NC}"
    fi
done

# Test .app zip
if [ -d macos/DataFold.app ]; then
    echo "Simulating .app zip: macos/DataFold.app -> dist-test/DataFold-$TEST_VERSION-$ARCH.app.zip"
    cd macos
    zip -r -q "$WORKSPACE_ROOT/dist-test/DataFold-$TEST_VERSION-$ARCH.app.zip" DataFold.app
    cd ..
    echo -e "${GREEN}✓ .app zip successful${NC}"
else
    echo -e "${YELLOW}⚠ No .app bundle to zip${NC}"
fi

cd "$WORKSPACE_ROOT"

echo ""
echo "Test artifacts in dist-test/:"
ls -lh dist-test/

echo ""
echo "========================================"
echo "Build Test Summary"
echo "========================================"
echo -e "Architecture: ${YELLOW}$ARCH${NC}"
echo -e "DMG produced: $([ "$DMG_FOUND" = true ] && echo -e "${GREEN}YES${NC}" || echo -e "${RED}NO${NC}")"
echo -e "APP produced: $([ "$APP_FOUND" = true ] && echo -e "${GREEN}YES${NC}" || echo -e "${RED}NO${NC}")"
echo ""

if [ "$DMG_FOUND" = true ] && [ "$APP_FOUND" = true ]; then
    echo -e "${GREEN}✓✓✓ Build test PASSED! The macOS release workflow should work.${NC}"
    echo ""
    echo "Test artifacts created in dist-test/:"
    ls -1 dist-test/
    echo ""
    echo "You can test the DMG by opening it:"
    echo "  open dist-test/DataFold-$TEST_VERSION-$ARCH.dmg"
    echo ""
    echo -e "${GREEN}The workflow is ready for deployment!${NC}"
    exit 0
else
    echo -e "${RED}✗✗✗ Build test FAILED! Issues must be fixed before deploying.${NC}"
    echo ""
    if [ "$DMG_FOUND" = false ]; then
        echo "- DMG was not created"
    fi
    if [ "$APP_FOUND" = false ]; then
        echo "- .app bundle was not created"
    fi
    exit 1
fi

