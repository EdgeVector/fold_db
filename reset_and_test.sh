#!/bin/bash
set -e

echo "🗑️  Deleting all DynamoDB tables..."
for table in DataFoldStorage-main DataFoldStorage-metadata DataFoldStorage-native_index \
             DataFoldStorage-node_id_schema_permissions DataFoldStorage-orchestrator_state \
             DataFoldStorage-public_keys DataFoldStorage-schema_states DataFoldStorage-schemas \
             DataFoldStorage-transform_queue_tree DataFoldStorage-transforms; do
    echo "  Deleting $table..."
    aws dynamodb delete-table --table-name $table --region us-west-2 2>/dev/null || echo "    (table doesn't exist or already deleted)"
done

echo ""
echo "⏳ Waiting for tables to be deleted..."
sleep 5

echo ""
echo "✅ Database reset complete!"
echo ""
echo "🔄 Restarting server..."
