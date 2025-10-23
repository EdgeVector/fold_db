#!/usr/bin/env python3
"""
Simple example of processing Twitter data with DataFold ingestion

This script demonstrates how to:
1. Load Twitter archive data
2. Convert it to proper JSON format
3. Send it to the DataFold ingestion system

Usage: python scripts/example_twitter_ingestion.py
"""

import requests
import json
import os
from pathlib import Path

def load_twitter_data(file_path):
    """Load and parse a Twitter JavaScript export file."""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Extract JSON from JavaScript wrapper
    start_idx = content.find('[')
    end_idx = content.rfind(']') + 1
    
    if start_idx != -1 and end_idx != -1:
        json_str = content[start_idx:end_idx]
        return json.loads(json_str)
    
    return None

def send_to_ingestion(data, server_url="http://localhost:9001"):
    """Send data to the DataFold ingestion endpoint."""
    endpoint = f"{server_url}/api/ingestion/process"
    
    payload = {
        "data": data,
        "auto_execute": True,
        "trust_distance": 0,
        "pub_key": "default"
    }
    
    response = requests.post(endpoint, json=payload, timeout=60)
    return response.json()

def main():
    # Example: Process tweets data
    tweets_file = Path("sample_data/twitter/data/tweets.js")
    
    if not tweets_file.exists():
        print(f"❌ File not found: {tweets_file}")
        print("💡 Make sure you have Twitter data in the sample_data/twitter/data/ directory")
        return
    
    print("📁 Loading Twitter tweets data...")
    tweets_data = load_twitter_data(tweets_file)
    
    if tweets_data is None:
        print("❌ Failed to load tweets data")
        return
    
    print(f"✅ Loaded {len(tweets_data)} tweets")
    
    # Show sample data structure
    if len(tweets_data) > 0:
        print("\n🔍 Sample tweet structure:")
        sample_tweet = tweets_data[0]
        print(json.dumps(sample_tweet, indent=2)[:500] + "...")
    
    # Send to ingestion (only first 5 tweets for demo)
    print(f"\n🚀 Sending first 5 tweets to ingestion system...")
    sample_data = tweets_data[:5]
    
    try:
        result = send_to_ingestion(sample_data)
        
        if result.get("success", False):
            print("✅ Successfully processed tweets!")
            print(f"📋 Schema: {result.get('schema_name', 'Unknown')}")
            print(f"📝 Mutations generated: {result.get('mutations_generated', 0)}")
            print(f"⚡ Mutations executed: {result.get('mutations_executed', 0)}")
        else:
            print(f"❌ Ingestion failed: {result.get('error', 'Unknown error')}")
    
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to connect to server: {e}")
        print("💡 Make sure the server is running: ./run_http_server.sh")

if __name__ == "__main__":
    main()
