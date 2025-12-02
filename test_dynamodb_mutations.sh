#!/bin/bash
# Script to test DynamoDB mutations
# Usage: ./test_dynamodb_mutations.sh

set -e

echo "🧪 Testing DynamoDB Mutations"
echo "================================"
echo ""

# Check if LocalStack is running (if using LocalStack)
if [ -n "$AWS_ENDPOINT_URL" ]; then
    echo "📡 Using LocalStack endpoint: $AWS_ENDPOINT_URL"
    # Test if LocalStack is accessible
    if curl -s "$AWS_ENDPOINT_URL/_localstack/health" > /dev/null 2>&1; then
        echo "✅ LocalStack is running"
    else
        echo "⚠️  Warning: LocalStack may not be running at $AWS_ENDPOINT_URL"
        echo "   Start it with: docker run -d -p 4566:4566 localstack/localstack"
    fi
else
    echo "📡 Using real AWS DynamoDB"
    echo "   Make sure AWS credentials are configured"
fi

echo ""
echo "Running mutation tests..."
echo ""

# Run the tests
cargo test --test dynamodb_mutation_test -- --ignored --nocapture

echo ""
echo "✅ Test complete!"
