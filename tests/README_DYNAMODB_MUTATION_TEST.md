# DynamoDB Mutation Test

This test verifies that mutations work correctly with DynamoDB backend after the storage abstraction refactoring.

## What It Tests

1. ✅ **Single mutation** - Basic create operation
2. ✅ **Batch mutations** - Multiple mutations in one batch
3. ✅ **Update mutation** - Update existing data
4. ✅ **Data persistence** - Verify data is stored correctly
5. ✅ **Rapid sequential mutations** - Stress test for deadlocks (7 mutations)
6. ✅ **Large batch mutation** - 10 mutations in one batch
7. ✅ **Node API integration** - Test through DataFoldNode

## Prerequisites

### Option 1: LocalStack (Recommended for Testing)

1. Start LocalStack:
```bash
docker run -d -p 4566:4566 localstack/localstack
```

2. Set environment variable:
```bash
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1
```

### Option 2: Real AWS DynamoDB

1. Configure AWS credentials:
```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_DEFAULT_REGION=us-east-1
```

## Running the Test

```bash
# Run the test (it's marked as ignored by default)
cargo test --test dynamodb_mutation_test -- --ignored --nocapture

# Or run specific test
cargo test --test dynamodb_mutation_test test_dynamodb_mutations_no_deadlock -- --ignored --nocapture
cargo test --test dynamodb_mutation_test test_dynamodb_mutations_with_node -- --ignored --nocapture
```

## Expected Results

✅ **All tests should pass**
- No deadlocks (mutations complete in < 10 seconds each)
- All mutations succeed
- Data is persisted correctly
- Async path working correctly

## What to Look For

### Success Indicators:
- ✅ Mutations complete quickly (< 10s for single, < 20s for batch of 10)
- ✅ No timeout errors
- ✅ All mutation IDs returned
- ✅ No "deadlock" or "timeout" messages in logs

### Failure Indicators:
- ❌ Mutations hang indefinitely
- ❌ Timeout errors after 60 seconds
- ❌ "deadlock" messages in logs
- ❌ Mutations take > 30 seconds

## Troubleshooting

### Test fails with "ResourceNotFoundException"
- **Cause**: Tables don't exist
- **Fix**: The test creates tables automatically, but you may need to wait a few seconds after LocalStack starts

### Test hangs
- **Cause**: Possible deadlock (this is what we're testing for!)
- **Fix**: Check logs for timeout messages. If it hangs, the refactoring may not have fully resolved the issue.

### AWS credentials error
- **Cause**: Missing or invalid AWS credentials
- **Fix**: Set AWS credentials or use LocalStack with test credentials

## Test Output

The test will print:
- 🧪 Test progress indicators
- ✅ Success messages for each test
- ⏱️ Timing information
- 📊 Performance metrics

Example output:
```
🧪 Starting DynamoDB mutation test...
📋 Storing test schema: test_users
✅ Schema loaded successfully

🧪 Test 1: Single mutation
✅ Single mutation completed in 234ms

🧪 Test 2: Batch mutations
✅ Batch mutations (2) completed in 456ms

...

✅ All DynamoDB mutation tests passed!
   - No deadlocks detected
   - All mutations completed successfully
   - Async path working correctly
```
