#!/usr/bin/env python3
"""Check if the native index tree has any data"""
import subprocess
import json

# Query a common word
result = subprocess.run(
    ["curl", "-s", "http://localhost:9001/api/native-index/search?term=the"],
    capture_output=True,
    text=True
)

print("Search for 'the':")
print(result.stdout)
print()

# Try to list all schemas to see what's there
result = subprocess.run(
    ["curl", "-s", "http://localhost:9001/api/schemas"],
    capture_output=True,
    text=True
)

schemas = json.loads(result.stdout)
print(f"Found {len(schemas)} schemas:")
for schema in schemas:
    if schema.get('state') == 'Approved':
        print(f"  - {schema.get('descriptive_name', schema.get('name'))} (Approved)")
        # Show a few fields
        if 'field_topologies' in schema:
            fields = list(schema['field_topologies'].keys())[:3]
            print(f"    Fields: {', '.join(fields)}")

