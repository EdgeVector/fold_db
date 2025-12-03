#!/bin/bash
# Direct DynamoDB write test to verify JSON string storage

ATOM_UUID="test-atom-$(date +%s)"
ATOM_KEY="atom:${ATOM_UUID}"

aws dynamodb put-item \
  --region us-west-2 \
  --table-name DataFoldStorage \
  --item '{
    "PK": {"S": "default:'${ATOM_KEY}'"},
    "SK": {"S": "'${ATOM_KEY}'"},
    "Value": {"S": "{\"uuid\":\"'${ATOM_UUID}'\",\"content\":{\"message\":\"This is readable JSON!\"},\"pub_key\":\"test-key\",\"source_file_name\":null,\"schema_name\":\"test\"}"}
  }'

echo ""
echo "Written atom with key: ${ATOM_KEY}"
echo "Now checking if it's readable in DynamoDB..."
echo ""

aws dynamodb get-item \
  --region us-west-2 \
  --table-name DataFoldStorage \
  --key '{"PK":{"S":"default:'${ATOM_KEY}'"},"SK":{"S":"'${ATOM_KEY}'"}}' \
  --output json | jq -r '.Item.Value.S' | jq
