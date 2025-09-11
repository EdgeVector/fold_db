#!/usr/bin/env python3
"""
Comprehensive DataFold User Activity Management Script

This script handles the complete workflow:
1. Uses the existing HTTP server (if running)
2. Adds user activity data to the database via HTTP API using curl
3. Queries and displays the user activity data

Usage: python scripts/manage_user_activity.py
"""

import requests
import json
import random
import time
import sys
import os
import subprocess
from datetime import datetime, timedelta

def check_http_server():
    """Check if the HTTP server is running."""
    try:
        response = requests.get("http://localhost:9001", timeout=5)
        if response.status_code == 200:
            print("✅ HTTP server is running on localhost:9001")
            return True
        else:
            print(f"⚠️  HTTP server responded with status: {response.status_code}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Could not connect to HTTP server: {e}")
        print("💡 Make sure the HTTP server is running: ./run_http_server.sh")
        return False

def create_user_activity_via_curl(user_id, action, resource, timestamp, metadata):
    """Create a single user activity via curl command."""
    mutation_data = {
        "type": "mutation",
        "schema": "UserActivity",
        "mutation_type": "create",
        "data": {
            "hash_key": user_id,
            "range_key": timestamp,
            "user_id": user_id,
            "action": action,
            "resource": resource,
            "timestamp": timestamp,
            "metadata": metadata
        }
    }
    
    # Convert to JSON string
    json_data = json.dumps(mutation_data)
    
    # Use curl to send the mutation
    curl_cmd = [
        "curl", "-X", "POST",
        "http://localhost:9001/api/mutation",
        "-H", "Content-Type: application/json",
        "-d", json_data
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            print(f"✅ Created activity: {user_id} - {action} at {timestamp}")
            return True
        else:
            print(f"❌ Failed to create activity: {result.stderr}")
            return False
    except subprocess.TimeoutExpired:
        print("❌ Request timed out")
        return False
    except Exception as e:
        print(f"❌ Error creating activity: {e}")
        return False

def query_user_activities_via_curl(user_id=None, action=None):
    """Query user activities via curl command."""
    query_data = {
        "type": "query",
        "schema": "UserActivity",
        "query_type": "get_all",
        "fields": ["user_id", "action", "resource", "timestamp", "metadata"]
    }
    
    # Add hash filter if user_id is specified (HashRange schemas use hash_filter format)
    if user_id:
        query_data["filter"] = {
            "hash_filter": {
                "Key": user_id
            }
        }
    
    # Convert to JSON string
    json_data = json.dumps(query_data)
    
    # Use curl to send the query
    curl_cmd = [
        "curl", "-X", "POST",
        "http://localhost:9001/api/query",
        "-H", "Content-Type: application/json",
        "-d", json_data
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            try:
                response_data = json.loads(result.stdout)
                return response_data
            except json.JSONDecodeError:
                print(f"❌ Invalid JSON response: {result.stdout}")
                return None
        else:
            print(f"❌ Query failed: {result.stderr}")
            return None
    except subprocess.TimeoutExpired:
        print("❌ Query timed out")
        return None
    except Exception as e:
        print(f"❌ Error querying activities: {e}")
        return None

def load_and_approve_schema():
    """Load and approve the UserActivity schema."""
    print("📚 Loading UserActivity schema...")
    
    # Check if schema file exists
    schema_path = "available_schemas/UserActivity.json"
    if not os.path.exists(schema_path):
        print(f"❌ Schema file not found: {schema_path}")
        return False
    
    # Load schema file
    try:
        with open(schema_path, 'r') as f:
            schema_data = json.load(f)
        print(f"✅ Loaded schema: {schema_data['name']}")
    except Exception as e:
        print(f"❌ Error loading schema: {e}")
        return False
    
    # Send schema to server
    curl_cmd = [
        "curl", "-X", "POST",
        "http://localhost:9001/api/schema",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(schema_data)
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            print("✅ Schema loaded successfully")
            return True
        else:
            print(f"❌ Failed to load schema: {result.stderr}")
            return False
    except Exception as e:
        print(f"❌ Error loading schema: {e}")
        return False

def create_sample_activities():
    """Create sample user activities."""
    print("🎯 Creating sample user activities...")
    
    # Sample activities data
    activities = [
        # User 1 activities
        {
            "user_id": "user_001",
            "action": "login",
            "resource": "web_app",
            "timestamp": "2025-01-15T09:00:00Z",
            "metadata": {
                "ip_address": "192.168.1.100",
                "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                "session_id": "sess_abc123"
            }
        },
        {
            "user_id": "user_001",
            "action": "view_page",
            "resource": "/dashboard",
            "timestamp": "2025-01-15T09:05:00Z",
            "metadata": {
                "page_title": "Dashboard",
                "duration": 45,
                "session_id": "sess_abc123"
            }
        },
        {
            "user_id": "user_001",
            "action": "create_post",
            "resource": "/posts/create",
            "timestamp": "2025-01-15T09:15:00Z",
            "metadata": {
                "post_title": "My First Post",
                "post_length": 150,
                "session_id": "sess_abc123"
            }
        },
        {
            "user_id": "user_001",
            "action": "logout",
            "resource": "web_app",
            "timestamp": "2025-01-15T10:30:00Z",
            "metadata": {
                "session_duration": 5400,
                "session_id": "sess_abc123"
            }
        },
        
        # User 2 activities
        {
            "user_id": "user_002",
            "action": "login",
            "resource": "mobile_app",
            "timestamp": "2025-01-15T11:00:00Z",
            "metadata": {
                "device_type": "mobile",
                "app_version": "2.1.0",
                "session_id": "sess_def456"
            }
        },
        {
            "user_id": "user_002",
            "action": "view_page",
            "resource": "/profile",
            "timestamp": "2025-01-15T11:05:00Z",
            "metadata": {
                "page_title": "User Profile",
                "duration": 120,
                "session_id": "sess_def456"
            }
        },
        {
            "user_id": "user_002",
            "action": "update_profile",
            "resource": "/profile/edit",
            "timestamp": "2025-01-15T11:10:00Z",
            "metadata": {
                "fields_updated": ["bio", "avatar"],
                "session_id": "sess_def456"
            }
        },
        
        # User 3 activities
        {
            "user_id": "user_003",
            "action": "login",
            "resource": "web_app",
            "timestamp": "2025-01-15T14:00:00Z",
            "metadata": {
                "ip_address": "10.0.0.50",
                "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
                "session_id": "sess_ghi789"
            }
        },
        {
            "user_id": "user_003",
            "action": "view_page",
            "resource": "/posts",
            "timestamp": "2025-01-15T14:05:00Z",
            "metadata": {
                "page_title": "All Posts",
                "duration": 30,
                "session_id": "sess_ghi789"
            }
        },
        {
            "user_id": "user_003",
            "action": "like_post",
            "resource": "/posts/123/like",
            "timestamp": "2025-01-15T14:10:00Z",
            "metadata": {
                "post_id": "123",
                "post_author": "user_001",
                "session_id": "sess_ghi789"
            }
        }
    ]
    
    success_count = 0
    for activity in activities:
        if create_user_activity_via_curl(
            activity["user_id"],
            activity["action"],
            activity["resource"],
            activity["timestamp"],
            activity["metadata"]
        ):
            success_count += 1
        time.sleep(0.1)  # Small delay between requests
    
    print(f"✅ Created {success_count}/{len(activities)} user activities")
    return success_count > 0

def display_activities(activities_data, title="User Activities"):
    """Display user activities in a formatted way."""
    if not activities_data or not activities_data.get("data"):
        print(f"❌ No {title.lower()} found")
        return
    
    activities = activities_data["data"]
    print(f"\n📊 {title} ({len(activities)} found):")
    print("=" * 80)
    
    for activity in activities:
        # Handle HashRange data format where fields contain arrays of {range_key, value} objects
        def extract_field_value(field_data):
            if isinstance(field_data, list) and len(field_data) > 0:
                # HashRange format: array of {range_key, value} objects
                return [item.get('value', 'N/A') for item in field_data]
            elif isinstance(field_data, dict):
                # Regular format: direct value
                return field_data
            else:
                return field_data
        
        user_id = extract_field_value(activity.get('user_id', 'N/A'))
        action = extract_field_value(activity.get('action', 'N/A'))
        resource = extract_field_value(activity.get('resource', 'N/A'))
        timestamp = extract_field_value(activity.get('timestamp', 'N/A'))
        
        print(f"👤 User: {user_id}")
        print(f"🎯 Action: {action}")
        print(f"📍 Resource: {resource}")
        print(f"⏰ Timestamp: {timestamp}")
        
        metadata = activity.get('metadata', {})
        if metadata:
            print("📋 Metadata:")
            if isinstance(metadata, list) and len(metadata) > 0:
                # HashRange format for metadata
                for i, meta_item in enumerate(metadata):
                    if isinstance(meta_item, dict) and 'value' in meta_item:
                        print(f"   Entry {i+1}: {meta_item['value']}")
            elif isinstance(metadata, dict):
                # Regular format for metadata
                for key, value in metadata.items():
                    print(f"   {key}: {value}")
        
        print("-" * 40)

def main():
    """Main function to run the user activity management workflow."""
    print("🎯 DataFold User Activity Management Script")
    print("=" * 60)
    
    # Check if HTTP server is running
    if not check_http_server():
        print("\n💡 To start the HTTP server, run:")
        print("   ./run_http_server.sh")
        return 1
    
    # Load and approve schema
    if not load_and_approve_schema():
        print("❌ Failed to load schema")
        return 1
    
    # Create sample activities
    if not create_sample_activities():
        print("❌ Failed to create sample activities")
        return 1
    
    # Query and display all activities
    print("\n🔍 Querying all user activities...")
    all_activities = query_user_activities_via_curl()
    display_activities(all_activities, "All User Activities")
    
    # Query activities for specific user
    print("\n🔍 Querying activities for user_001...")
    user1_activities = query_user_activities_via_curl(user_id="user_001")
    display_activities(user1_activities, "User 001 Activities")
    
    # Query activities by action
    print("\n🔍 Querying all login activities...")
    login_activities = query_user_activities_via_curl(action="login")
    display_activities(login_activities, "Login Activities")
    
    print("\n💡 Next steps:")
    print("   - Check the HTTP server UI at: http://localhost:9001")
    print("   - Try querying different users or actions")
    print("   - Experiment with range filters for time-based queries")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
