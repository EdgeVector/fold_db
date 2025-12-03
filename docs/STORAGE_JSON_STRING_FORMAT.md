# DynamoDB Storage Format Change: Binary to JSON String

## Summary

Changed DynamoDB `Value` field from **Binary** (type `B`) to **JSON String** (type `S`) for human readability.

## Before vs After

### Before (Binary Storage - Not Readable)
```
PK: default:atom:abc123
SK: atom:abc123
Value: <Binary> (base64 blob - unreadable in console)
```

### After (JSON String Storage - Readable!)
```
PK: default:atom:abc123
SK: atom:abc123
Value: {"uuid":"abc123","content":{"message":"Hello"},"pub_key":"key123",...}
```

## Benefits

✅ **Human-readable** in AWS DynamoDB console  
✅ **Debuggable** - can inspect data directly  
✅ **No performance penalty** - DynamoDB treats strings and binary the same  
✅ **Still JSON** - same serialization format, just stored as string  
✅ **Easy troubleshooting** - can query and verify data without decoding  

## Technical Changes

### Files Modified
- `src/storage/dynamodb_backend.rs`

### Changes Made
1. **PUT operations**: `AttributeValue::B(bytes)` → `AttributeValue::S(json_string)`
2. **GET operations**: Read `AttributeValue::S` and convert to bytes
3. **SCAN operations**: Read `AttributeValue::S` from results
4. **BATCH operations**: Convert to JSON string before batching

### Code Example

```rust
// BEFORE (Binary)
.item("Value", AttributeValue::B(value.clone().into()))

// AFTER (JSON String)
let json_str = String::from_utf8(value.clone())?;
.item("Value", AttributeValue::S(json_str))
```

## Viewing Data

### AWS CLI
```bash
aws dynamodb get-item \
  --table-name DataFoldStorage-main \
  --key '{"PK":{"S":"default:atom:xyz"},"SK":{"S":"atom:xyz"}}' \
  | jq -r '.Item.Value.S' | jq
```

### AWS Console
1. Navigate to DynamoDB
2. Select table (e.g., `DataFoldStorage-main`)
3. Click "Explore table items"
4. View any item - the `Value` field shows readable JSON!

## Compatibility

- ✅ Backward compatible with existing code
- ✅ Same JSON serialization (serde_json)
- ✅ Same data structures (Atom, Molecule, etc.)
- ⚠️ **Not compatible with old binary data** - requires migration or dual-read logic

## Migration Notes

Old binary data will need to be migrated or the code needs dual-read logic:

```rust
// Read either String or Binary
if let Some(AttributeValue::S(json_str)) = item.get("Value") {
    return Ok(Some(json_str.as_bytes().to_vec()));
} else if let Some(AttributeValue::B(data)) = item.get("Value") {
    return Ok(Some(data.as_ref().to_vec()));
}
```

## Example Data in DynamoDB

```json
{
  "PK": {"S": "default:atom:test-readable-1764713555"},
  "SK": {"S": "atom:test-readable-1764713555"},
  "Value": {
    "S": "{\"uuid\":\"test-readable-1764713555\",\"content\":{\"message\":\"This is READABLE JSON in DynamoDB!\",\"author\":\"Test User\"},\"pub_key\":\"test-key-123\",\"source_file_name\":null,\"schema_name\":\"test_schema\"}"
  }
}
```

When you view this in the console, you can see and read the entire content structure!
