#!/bin/bash

# Generate comprehensive code coverage for the datafold project
# Covers both Rust backend and React frontend components

set -e

echo "🧪 Generating comprehensive coverage report for datafold project"
echo "================================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Generate Rust coverage
print_status "Generating Rust coverage..."

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    print_warning "cargo-llvm-cov not found. Installing..."
    cargo install cargo-llvm-cov
fi

# Run coverage for the entire workspace and output HTML report
print_status "Running Rust tests with coverage..."
cargo llvm-cov --workspace --html --output-dir target/coverage-html

# Also generate LCOV format for CI integration
cargo llvm-cov --workspace --lcov --output-path target/coverage.lcov

print_success "Rust coverage generated in target/coverage-html/"

# Generate frontend coverage
print_status "Generating frontend coverage..."

if [[ ! -d "src/datafold_node/static-react" ]]; then
    print_warning "Frontend directory not found. Skipping frontend coverage."
else
    cd src/datafold_node/static-react
    
    # Check if node_modules exists
    if [[ ! -d "node_modules" ]]; then
        print_status "Installing frontend dependencies..."
        npm install
    fi
    
    # Run frontend tests with coverage
    print_status "Running frontend tests with coverage..."
    npm run test:coverage
    
    print_success "Frontend coverage generated in src/datafold_node/static-react/coverage/"
    
    cd - > /dev/null
fi

# Generate combined report summary
print_status "Generating coverage summary..."

echo ""
echo "📊 Coverage Report Summary"
echo "=========================="

if [[ -f "target/coverage-html/index.html" ]]; then
    echo "🦀 Rust Coverage: target/coverage-html/index.html"
fi

if [[ -f "src/datafold_node/static-react/coverage/index.html" ]]; then
    echo "⚛️  Frontend Coverage: src/datafold_node/static-react/coverage/index.html"
fi

echo ""
echo "🚀 Quick Actions:"
echo "  • Open Rust coverage:     open target/coverage-html/index.html"
echo "  • Open frontend coverage: open src/datafold_node/static-react/coverage/index.html"
echo "  • Run coverage check:     npm run ci:coverage-check --prefix src/datafold_node/static-react"
echo ""

print_success "Coverage generation complete!"

# Optional: automatically open coverage reports
if command -v open >/dev/null 2>&1; then
    read -p "Open coverage reports in browser? (y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [[ -f "target/coverage-html/index.html" ]]; then
            open target/coverage-html/index.html
        fi
        if [[ -f "src/datafold_node/static-react/coverage/index.html" ]]; then
            open src/datafold_node/static-react/coverage/index.html
        fi
    fi
fi

