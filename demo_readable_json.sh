#!/bin/bash
set -e

echo "==============================================="
echo "📖 DynamoDB Readable JSON Storage Demo"
echo "==============================================="
echo ""

# Wait for tables to be fully deleted
echo "⏳ Ensuring DataFoldStorage-main table exists..."
aws dynamodb describe-table --table-name DataFoldStorage-main --region us-west-2 >/dev/null 2>&1 && echo "✅ Table exists" || {
    echo "❌ Table doesn't exist, creating it..."
    aws dynamodb create-table \
        --table-name DataFoldStorage-main \
        --attribute-definitions \
            AttributeName=PK,AttributeType=S \
            AttributeName=SK,AttributeType=S \
        --key-schema \
            AttributeName=PK,KeyType=HASH \
            AttributeName=SK,KeyType=RANGE \
        --billing-mode PAY_PER_REQUEST \
        --region us-west-2 >/dev/null
    
    echo "⏳ Waiting for table to become active..."
    sleep 10
    echo "✅ Table created"
}

echo ""
echo "==============================================="
echo "📝 Writing Test Data as JSON String"
echo "==============================================="
echo ""

# Write an atom
ATOM_UUID="demo-atom-$(date +%s)"
ATOM_KEY="atom:${ATOM_UUID}"

aws dynamodb put-item \
  --region us-west-2 \
  --table-name DataFoldStorage-main \
  --item '{
    "PK": {"S": "default:'${ATOM_KEY}'"},
    "SK": {"S": "'${ATOM_KEY}'"},
    "Value": {"S": "{\"uuid\":\"'${ATOM_UUID}'\",\"content\":{\"message\":\"This JSON is HUMAN READABLE in DynamoDB console!\",\"author\":\"Demo User\",\"timestamp\":\"2024-12-02T22:00:00Z\"},\"pub_key\":\"demo-key-123\",\"source_file_name\":\"demo.json\",\"schema_name\":\"demo_schema\"}"}
  }'

echo "✅ Written atom with key: ${ATOM_KEY}"
echo ""

# Write a molecule
MOL_UUID="demo-mol-$(date +%s)"
MOL_KEY="ref:${MOL_UUID}"

aws dynamodb put-item \
  --region us-west-2 \
  --table-name DataFoldStorage-main \
  --item '{
    "PK": {"S": "default:'${MOL_KEY}'"},
    "SK": {"S": "'${MOL_KEY}'"},
    "Value": {"S": "{\"uuid\":\"'${MOL_UUID}'\",\"atom_uuids\":{\"user1\":\"'${ATOM_UUID}'\",\"user2\":\"atom-xyz\"},\"pub_key\":\"demo-key-123\"}"}
  }'

echo "✅ Written molecule with key: ${MOL_KEY}"
echo ""

echo "==============================================="
echo "📖 Reading Data from DynamoDB"
echo "==============================================="
echo ""

echo "🔍 Atom Data (Pretty Printed JSON):"
echo "---"
aws dynamodb get-item \
  --region us-west-2 \
  --table-name DataFoldStorage-main \
  --key '{"PK":{"S":"default:'${ATOM_KEY}'"},"SK":{"S":"'${ATOM_KEY}'"}}' \
  --output json | jq -r '.Item.Value.S' | jq '.'

echo ""
echo "🔍 Molecule Data (Pretty Printed JSON):"
echo "---"
aws dynamodb get-item \
  --region us-west-2 \
  --table-name DataFoldStorage-main \
  --key '{"PK":{"S":"default:'${MOL_KEY}'"},"SK":{"S":"'${MOL_KEY}'"}}' \
  --output json | jq -r '.Item.Value.S' | jq '.'

echo ""
echo "==============================================="
echo "✅ Demo Complete!"
echo "==============================================="
echo ""
echo "You can now go to the AWS DynamoDB console and:"
echo "1. Navigate to the 'DataFoldStorage-main' table"
echo "2. Click 'Explore table items'"
echo "3. Find these items:"
echo "   - PK: default:${ATOM_KEY}"
echo "   - PK: default:${MOL_KEY}"
echo "4. View the 'Value' field - it's READABLE JSON!"
echo ""
echo "🎉 No more binary blobs - all data is human-readable!"
