#!/usr/bin/env python3
import boto3
import json

# Initialize DynamoDB client
dynamodb = boto3.client('dynamodb', region_name='us-west-2')

# Get an item from the table
response = dynamodb.get_item(
    TableName='DataFoldStorage-main',
    Key={
        'PK': {'S': 'default:atom:test-readable-1764713555'},
        'SK': {'S': 'atom:test-readable-1764713555'}
    }
)

print("✅ DynamoDB Console View:")
print("=" * 60)
print(f"PK: {response['Item']['PK']['S']}")
print(f"SK: {response['Item']['SK']['S']}")
print(f"Value Type: String (S) - Human Readable!")
print("")
print("📖 Value Content (formatted):")
print("=" * 60)
value_json = json.loads(response['Item']['Value']['S'])
print(json.dumps(value_json, indent=2))
