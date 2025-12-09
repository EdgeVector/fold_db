#!/usr/bin/env python3
"""
Simple Twitter Ingestion Test Script

This script tests the Twitter ingestion fix with a minimal set of data
to verify that mutations are being generated correctly.
"""

import requests
import json
import os
import sys
from pathlib import Path

# Configuration
SERVER_URL = "http://localhost:9001"
INGESTION_ENDPOINT = f"{SERVER_URL}/api/ingestion/process"

def check_server_running() -> bool:
    """Check if the HTTP server is running."""
    try:
        response = requests.get(SERVER_URL, timeout=5)
        return response.status_code == 200
    except requests.exceptions.RequestException:
        return False

def load_twitter_sample(file_name: str, max_records: int = 3):
    """Load a small sample of Twitter data for testing."""
    file_path = Path("sample_data/twitter/data") / file_name
    
    if not file_path.exists():
        print(f"❌ File not found: {file_path}")
        return None
    
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Extract JSON from JavaScript wrapper
        start_idx = content.find('[')
        end_idx = content.rfind(']') + 1
        
        if start_idx != -1 and end_idx != -1:
            json_str = content[start_idx:end_idx]
            data = json.loads(json_str)
            
            # Take only first few records for testing
            if isinstance(data, list) and len(data) > max_records:
                data = data[:max_records]
            
            return data
        else:
            print(f"⚠️  Could not extract JSON from {file_name}")
            return None
            
    except Exception as e:
        print(f"❌ Error reading {file_name}: {e}")
        return None

def test_ingestion(data, data_type: str):
    """Test ingestion with a small data sample."""
    print(f"\n🧪 Testing {data_type} ingestion...")
    
    payload = {
        "data": data,
        "auto_execute": True,
        "trust_distance": 0,
        "pub_key": "default"
    }
    
    try:
        response = requests.post(INGESTION_ENDPOINT, json=payload, timeout=30)
        
        # Handle async 202 response
        if response.status_code == 202:
            result = response.json()
            progress_id = result.get("progress_id")
            print(f"   ⏳ Ingestion started async (ID: {progress_id}). Polling for status...")
            
            # Poll for completion
            import time
            max_retries = 30
            for i in range(max_retries):
                time.sleep(1)
                status_url = f"{SERVER_URL}/api/ingestion/progress/{progress_id}"
                status_resp = requests.get(status_url, timeout=5)
                
                if status_resp.status_code == 200:
                    status = status_resp.json()
                    if status.get("is_complete") or status.get("is_failed"):
                        if status.get("is_failed"):
                            print(f"❌ {data_type}: Ingestion failed async - {status.get('error_message')}")
                            return False
                        
                        # Success
                        results = status.get("results", {})
                        mutations_generated = results.get("mutations_generated", 0)
                        mutations_executed = results.get("mutations_executed", 0)
                        
                        print(f"✅ {data_type}: {mutations_generated} mutations generated, {mutations_executed} executed")
                        
                        if mutations_generated > 0:
                            print(f"   🎉 SUCCESS: Mutations are being generated!")
                            return True
                        else:
                            print(f"   ⚠️  WARNING: No mutations generated")
                            return False
                else:
                    print(f"   ⚠️  Status poll failed: {status_resp.status_code}")
            
            print(f"❌ {data_type}: Timed out waiting for ingestion")
            return False

        # Handle legacy synchronous 200 response (just in case)
        elif response.status_code == 200:
            result = response.json()
            if result.get("success", False):
                mutations_generated = result.get("mutations_generated", 0)
                mutations_executed = result.get("mutations_executed", 0)
                
                print(f"✅ {data_type}: {mutations_generated} mutations generated, {mutations_executed} executed")
                
                if mutations_generated > 0:
                    print(f"   🎉 SUCCESS: Mutations are being generated!")
                    return True
                else:
                    print(f"   ⚠️  WARNING: No mutations generated")
                    return False
            else:
                print(f"❌ {data_type}: Ingestion failed - {result.get('error', 'Unknown error')}")
                return False
        else:
            print(f"❌ {data_type}: HTTP error {response.status_code}: {response.text}")
            return False
            
    except requests.exceptions.RequestException as e:
        print(f"❌ {data_type}: Request failed: {e}")
        return False

def main():
    print("🚀 Twitter Ingestion Fix Test")
    print("=" * 50)
    
    # Check if server is running
    if not check_server_running():
        print("❌ HTTP server is not running")
        print("💡 Please start the server: ./run_http_server.sh")
        sys.exit(1)
    
    print("✅ HTTP server is running")
    
    # Test cases - small samples of different Twitter data types
    test_cases = [
        ("tweets.js", "Tweets"),
        ("following.js", "Following"),
        ("like.js", "Likes"),
        ("account.js", "Account"),
        ("direct-messages.js", "Direct Messages")
    ]
    
    successful_tests = 0
    total_tests = 0
    
    for file_name, data_type in test_cases:
        data = load_twitter_sample(file_name, max_records=2)  # Only 2 records for speed
        
        if data is not None:
            total_tests += 1
            
            # Show data structure
            if isinstance(data, list) and len(data) > 0:
                print(f"📊 {data_type}: {len(data)} records")
                if isinstance(data[0], dict):
                    print(f"   🔍 Sample keys: {list(data[0].keys())}")
            
            # Test ingestion
            if test_ingestion(data, data_type):
                successful_tests += 1
        else:
            print(f"⚠️  Skipping {data_type} - could not load data")
    
    # Summary
    print(f"\n📊 Test Summary:")
    print(f"   ✅ Successful: {successful_tests}")
    print(f"   📁 Total tests: {total_tests}")
    
    if successful_tests == total_tests and total_tests > 0:
        print(f"\n🎉 ALL TESTS PASSED! Twitter ingestion fix is working correctly!")
        print("💡 You can now process your full Twitter data with confidence.")
    elif successful_tests > 0:
        print(f"\n⚠️  PARTIAL SUCCESS: {successful_tests}/{total_tests} tests passed")
        print("💡 Some Twitter data types are working, but there may be issues with others.")
    else:
        print(f"\n❌ ALL TESTS FAILED: Twitter ingestion fix needs more work")
        print("💡 Check the error messages above for details.")

if __name__ == "__main__":
    main()
