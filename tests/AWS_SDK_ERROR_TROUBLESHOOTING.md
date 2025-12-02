# AWS SDK "Service Error" Troubleshooting Guide

## The Problem

The test is failing with a generic "service error" from the AWS SDK. This is a very generic error that can have multiple causes.

## Diagnostic Steps

### 1. Verify AWS Credentials Are Accessible

```bash
# Test with AWS CLI (this works, so credentials are valid)
aws dynamodb list-tables --region us-east-1

# Test with explicit credentials
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_DEFAULT_REGION=us-east-1
```

### 2. Check Network Connectivity

The Rust AWS SDK might be using a different network path than the AWS CLI:

```bash
# Test connectivity to DynamoDB endpoint
curl -v https://dynamodb.us-east-1.amazonaws.com/

# Check if you're behind a proxy
echo $HTTP_PROXY
echo $HTTPS_PROXY
echo $NO_PROXY
```

### 3. Verify Region Configuration

Make sure the region matches where your tables are:

```bash
# List your tables and their regions
aws dynamodb list-tables --region us-east-1

# Check table details
aws dynamodb describe-table --table-name TestMutationStorage-main --region us-east-1
```

### 4. Test with Minimal Example

Create a simple test to isolate the issue:

```rust
use aws_sdk_dynamodb::Client;
use aws_config::defaults;

#[tokio::test]
async fn test_minimal_dynamodb() {
    let config = defaults(aws_config::BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;
    let client = Client::new(&config);
    
    // Test 1: List tables
    match client.list_tables().send().await {
        Ok(res) => println!("✅ List tables works: {:?}", res.table_names()),
        Err(e) => println!("❌ List tables failed: {:?}", e),
    }
    
    // Test 2: Get item
    match client
        .get_item()
        .table_name("TestMutationStorage-main")
        .key("PK", aws_sdk_dynamodb::types::AttributeValue::S("test_user_mutations:test_key".to_string()))
        .key("SK", aws_sdk_dynamodb::types::AttributeValue::S("test_key".to_string()))
        .send()
        .await
    {
        Ok(res) => println!("✅ Get item works: {:?}", res.item()),
        Err(e) => println!("❌ Get item failed: {:?}", e),
    }
}
```

### 5. Common Causes and Solutions

#### A. Credential Provider Chain Issue

The Rust SDK might not be finding credentials the same way AWS CLI does:

**Solution:** Set explicit credentials:
```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_SESSION_TOKEN=your_token  # If using temporary credentials
```

#### B. HTTP Client Configuration

The SDK might need explicit HTTP client configuration:

**Solution:** Add to your test:
```rust
use aws_config::defaults;
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

let config = defaults(aws_config::BehaviorVersion::latest())
    .region("us-east-1")
    .http_client(HyperClientBuilder::new().build())
    .load()
    .await;
```

#### C. Retry/Timeout Configuration

The default retry might not be sufficient:

**Solution:** Already added in the test, but you can increase:
```rust
.retry_config(
    aws_config::retry::RetryConfig::standard()
        .with_max_attempts(5)  // Increase from 3
)
```

#### D. Endpoint URL Override

If you have `AWS_ENDPOINT_URL` set, it might be interfering:

**Solution:** Make sure it's unset for real AWS:
```bash
unset AWS_ENDPOINT_URL
```

### 6. Enable Detailed Logging

Run with detailed AWS SDK logging:

```bash
RUST_LOG=aws_smithy_runtime=debug,aws_sdk_dynamodb=debug \
cargo test --test dynamodb_mutation_test test_dynamodb_mutations_no_deadlock -- --ignored --nocapture
```

### 7. Check for Proxy/Firewall Issues

If you're behind a corporate firewall or proxy:

```bash
# Set proxy if needed
export HTTP_PROXY=http://proxy:port
export HTTPS_PROXY=http://proxy:port
export NO_PROXY=localhost,127.0.0.1

# Or configure in AWS SDK
```

### 8. Verify IAM Permissions

Make sure your AWS credentials have the necessary permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:DeleteItem",
        "dynamodb:Query",
        "dynamodb:Scan",
        "dynamodb:DescribeTable",
        "dynamodb:CreateTable",
        "dynamodb:ListTables"
      ],
      "Resource": "*"
    }
  ]
}
```

### 9. Test with LocalStack (Alternative)

If real AWS continues to have issues, test with LocalStack:

```bash
# Start LocalStack
docker run -d -p 4566:4566 localstack/localstack

# Set endpoint
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1

# Run test
cargo test --test dynamodb_mutation_test -- --ignored --nocapture
```

## Next Steps

1. Run the minimal test above to isolate the issue
2. Check the detailed logs with `RUST_LOG=debug`
3. Compare AWS CLI behavior vs Rust SDK behavior
4. If the issue persists, the error might be in how the SDK is being used in the codebase

## Getting More Error Details

The current error message "service error" is too generic. To get more details, you can:

1. Add `eprintln!("Full error: {:?}", e)` in the retry macro
2. Check if the error has an inner error that's being masked
3. Look at the AWS SDK error types to extract more information

The improved error logging in `dynamodb_utils.rs` should help, but you may need to run with `RUST_LOG=debug` to see it.
