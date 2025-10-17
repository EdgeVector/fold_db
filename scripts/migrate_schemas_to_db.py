#!/usr/bin/env python3
"""
Migration utility to migrate schema JSON files to sled database.

This script reads schema JSON files from the available_schemas directory
and imports them into the schema service's sled database via HTTP API.

Usage:
    python3 scripts/migrate_schemas_to_db.py [--schemas-dir DIR] [--service-url URL]
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import List, Dict, Any

try:
    import requests
except ImportError:
    print("Error: requests library not found. Install it with: pip install requests")
    sys.exit(1)


def load_schema_files(schemas_dir: str) -> List[Dict[str, Any]]:
    """Load all schema JSON files from the specified directory."""
    schemas = []
    schemas_path = Path(schemas_dir)
    
    if not schemas_path.exists():
        print(f"Warning: Schema directory '{schemas_dir}' does not exist")
        return schemas
    
    for file_path in schemas_path.glob("*.json"):
        try:
            with open(file_path, 'r') as f:
                schema = json.load(f)
                if 'name' in schema:
                    schemas.append(schema)
                    print(f"Loaded schema '{schema['name']}' from {file_path.name}")
                else:
                    print(f"Warning: Schema file '{file_path.name}' missing 'name' field, skipping")
        except json.JSONDecodeError as e:
            print(f"Error: Failed to parse {file_path.name}: {e}")
        except Exception as e:
            print(f"Error: Failed to read {file_path.name}: {e}")
    
    return schemas


def migrate_schemas(schemas: List[Dict[str, Any]], service_url: str) -> None:
    """Migrate schemas to the schema service database via HTTP API."""
    if not schemas:
        print("No schemas to migrate")
        return
    
    success_count = 0
    error_count = 0
    similar_count = 0
    
    for schema in schemas:
        schema_name = schema.get('name', 'unknown')
        try:
            response = requests.post(
                f"{service_url}/api/schemas",
                json=schema,
                timeout=10
            )
            
            if response.status_code == 201:
                print(f"✓ Successfully added schema '{schema_name}'")
                success_count += 1
            elif response.status_code == 409:
                print(f"~ Schema '{schema_name}' is too similar to an existing schema")
                similar_count += 1
            else:
                print(f"✗ Failed to add schema '{schema_name}': {response.status_code}")
                print(f"  Response: {response.text}")
                error_count += 1
        except requests.RequestException as e:
            print(f"✗ Error adding schema '{schema_name}': {e}")
            error_count += 1
    
    print("\n" + "="*60)
    print(f"Migration summary:")
    print(f"  Successfully added: {success_count}")
    print(f"  Similar (skipped):  {similar_count}")
    print(f"  Errors:             {error_count}")
    print(f"  Total processed:    {len(schemas)}")
    print("="*60)


def verify_service_available(service_url: str) -> bool:
    """Verify that the schema service is running and accessible."""
    try:
        response = requests.get(f"{service_url}/api/health", timeout=5)
        if response.status_code == 200:
            print(f"✓ Schema service is running at {service_url}")
            return True
        else:
            print(f"✗ Schema service returned unexpected status: {response.status_code}")
            return False
    except requests.RequestException as e:
        print(f"✗ Cannot connect to schema service at {service_url}: {e}")
        print("  Make sure the schema service is running first.")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Migrate schema JSON files to schema service database"
    )
    parser.add_argument(
        "--schemas-dir",
        default="available_schemas",
        help="Directory containing schema JSON files (default: available_schemas)"
    )
    parser.add_argument(
        "--service-url",
        default="http://127.0.0.1:9002",
        help="Schema service URL (default: http://127.0.0.1:9002)"
    )
    
    args = parser.parse_args()
    
    print("Schema Migration Utility")
    print("="*60)
    
    # Verify service is available
    if not verify_service_available(args.service_url):
        sys.exit(1)
    
    print()
    
    # Load schemas from JSON files
    print(f"Loading schemas from '{args.schemas_dir}'...")
    schemas = load_schema_files(args.schemas_dir)
    print(f"Found {len(schemas)} schema files\n")
    
    if not schemas:
        print("No schemas found to migrate")
        return
    
    # Migrate schemas
    print("Migrating schemas to database...")
    migrate_schemas(schemas, args.service_url)


if __name__ == "__main__":
    main()

