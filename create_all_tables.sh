#!/bin/bash
echo "Creating all required DynamoDB tables..."

for table in orchestrator_state metadata schemas schema_states transforms transform_queue_tree public_keys node_id_schema_permissions native_index; do
    full_name="DataFoldStorage-${table}"
    echo "Creating ${full_name}..."
    aws dynamodb create-table \
        --table-name "${full_name}" \
        --attribute-definitions \
            AttributeName=PK,AttributeType=S \
            AttributeName=SK,AttributeType=S \
        --key-schema \
            AttributeName=PK,KeyType=HASH \
            AttributeName=SK,KeyType=RANGE \
        --billing-mode PAY_PER_REQUEST \
        --region us-west-2 2>/dev/null && echo "✅ ${full_name}" || echo "⚠️  ${full_name} (may already exist)"
done

echo "✅ All tables created/verified"
