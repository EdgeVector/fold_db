#!/usr/bin/env python3
"""
Twitter Data Processing Script for DataFold Ingestion

This script processes Twitter archive data and feeds it into the DataFold ingestion system.
It handles the JavaScript format of Twitter exports and converts them to proper JSON for ingestion.

Usage: python scripts/process_twitter_data.py [options]

Examples:
  # Process all Twitter data files
  python scripts/process_twitter_data.py --all

  # Process specific data types
  python scripts/process_twitter_data.py --tweets --likes --following

  # Process with custom settings
  python scripts/process_twitter_data.py --tweets --auto-execute --trust-distance 1
"""

import requests
import json
import os
import sys
import argparse
import time
from pathlib import Path
from typing import Dict, List, Any, Optional
import subprocess

# Configuration
TWITTER_DATA_DIR = "sample_data/twitter/data"
SERVER_URL = "http://localhost:9001"
INGESTION_ENDPOINT = f"{SERVER_URL}/api/ingestion/process"

# Twitter data files and their descriptions
TWITTER_FILES = {
    "tweets.js": "Twitter posts and replies",
    "like.js": "Liked tweets",
    "following.js": "Accounts being followed", 
    "follower.js": "Followers",
    "account.js": "Account information",
    "profile.js": "Profile details",
    "direct-messages.js": "Direct messages",
    "direct-messages-group.js": "Group direct messages",
    "block.js": "Blocked accounts",
    "mute.js": "Muted accounts",
    "lists-created.js": "Created lists",
    "lists-member.js": "Lists the user is a member of",
    "lists-subscribed.js": "Subscribed lists",
    "moment.js": "Moments created",
    "article.js": "Articles published",
    "periscope-broadcast-metadata.js": "Periscope broadcasts",
    "community-note.js": "Community notes written",
    "community-note-rating.js": "Community note ratings",
    "grok-chat-item.js": "Grok chat conversations"
}

def check_server_running() -> bool:
    """Check if the HTTP server is running."""
    try:
        response = requests.get(SERVER_URL, timeout=5)
        return response.status_code == 200
    except requests.exceptions.RequestException:
        return False

def start_server() -> bool:
    """Start the HTTP server if not running."""
    print("🚀 Starting HTTP server...")
    try:
        # Start server in background
        subprocess.Popen(["./run_http_server.sh"], 
                        stdout=subprocess.DEVNULL, 
                        stderr=subprocess.DEVNULL)
        
        # Wait for server to start
        for i in range(30):  # Wait up to 30 seconds
            if check_server_running():
                print("✅ HTTP server started successfully")
                return True
            time.sleep(1)
            print(f"⏳ Waiting for server to start... ({i+1}/30)")
        
        print("❌ Server failed to start within 30 seconds")
        return False
    except Exception as e:
        print(f"❌ Failed to start server: {e}")
        return False

def load_twitter_js_file(file_path: Path) -> Optional[Dict[str, Any]]:
    """
    Load a Twitter JavaScript file and extract the JSON data.
    
    Twitter exports are in JavaScript format like:
    window.YTD.tweets.part0 = [{...}]
    """
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Extract the JSON data from the JavaScript wrapper
        # Look for the pattern: window.YTD.{name}.part0 = [...]
        if 'window.YTD' in content:
            # Find the start of the array
            start_idx = content.find('[')
            end_idx = content.rfind(']') + 1
            
            if start_idx != -1 and end_idx != -1:
                json_str = content[start_idx:end_idx]
                data = json.loads(json_str)
                return data
            else:
                print(f"⚠️  Could not extract JSON from {file_path.name}")
                return None
        else:
            print(f"⚠️  File {file_path.name} doesn't appear to be a Twitter export")
            return None
            
    except json.JSONDecodeError as e:
        print(f"❌ JSON decode error in {file_path.name}: {e}")
        return None
    except Exception as e:
        print(f"❌ Error reading {file_path.name}: {e}")
        return None

def process_data_with_ingestion(data: Any, auto_execute: bool = True, 
                               trust_distance: int = 0, pub_key: str = "default") -> bool:
    """Send data to the ingestion endpoint and wait for completion."""
    try:
        payload = {
            "data": data,
            "auto_execute": auto_execute,
            "trust_distance": trust_distance,
            "pub_key": pub_key
        }
        
        # Send the request with a longer timeout to allow for processing time
        response = requests.post(INGESTION_ENDPOINT, json=payload, timeout=120)
        
        if response.status_code == 200:
            result = response.json()
            if result.get("success", False):
                print(f"✅ Successfully processed data")
                if "schema_name" in result:
                    print(f"   📋 Schema: {result['schema_name']}")
                if "mutations_generated" in result:
                    print(f"   📝 Mutations: {result['mutations_generated']}")
                if "mutations_executed" in result:
                    print(f"   ⚡ Executed: {result['mutations_executed']}")
                
                # Add a small delay to ensure database operations complete
                time.sleep(1)
                return True
            else:
                print(f"❌ Ingestion failed: {result.get('error', 'Unknown error')}")
                return False
        else:
            print(f"❌ HTTP error {response.status_code}: {response.text}")
            return False
            
    except requests.exceptions.RequestException as e:
        print(f"❌ Request failed: {e}")
        return False

def process_twitter_file(file_name: str, auto_execute: bool = True, 
                        trust_distance: int = 0, pub_key: str = "default") -> bool:
    """Process a single Twitter data file."""
    file_path = Path(TWITTER_DATA_DIR) / file_name
    
    if not file_path.exists():
        print(f"❌ File not found: {file_path}")
        return False
    
    print(f"\n📁 Processing {file_name}...")
    print(f"   📖 Description: {TWITTER_FILES.get(file_name, 'Unknown data type')}")
    
    # Load the data
    data = load_twitter_js_file(file_path)
    if data is None:
        return False
    
    # Show data summary
    if isinstance(data, list):
        print(f"   📊 Records: {len(data)}")
        if len(data) > 0:
            print(f"   🔍 Sample keys: {list(data[0].keys()) if isinstance(data[0], dict) else 'Not a dict'}")
    else:
        print(f"   📊 Data type: {type(data).__name__}")
    
    # Process with ingestion
    return process_data_with_ingestion(data, auto_execute, trust_distance, pub_key)

def main():
    parser = argparse.ArgumentParser(description="Process Twitter archive data with DataFold ingestion")
    parser.add_argument("--all", action="store_true", help="Process all available Twitter data files")
    parser.add_argument("--auto-execute", action="store_true", default=True, help="Auto-execute mutations")
    parser.add_argument("--trust-distance", type=int, default=0, help="Trust distance for mutations")
    parser.add_argument("--pub-key", default="default", help="Public key for mutations")
    
    # Add arguments for specific file types
    for file_name in TWITTER_FILES.keys():
        arg_name = file_name.replace('.js', '').replace('-', '_')
        parser.add_argument(f"--{arg_name}", action="store_true", 
                          help=f"Process {file_name}")
    
    args = parser.parse_args()
    
    # Check if server is running
    if not check_server_running():
        print("❌ HTTP server is not running")
        print("💡 Starting server...")
        if not start_server():
            print("❌ Could not start server. Please run: ./run_http_server.sh")
            sys.exit(1)
    else:
        print("✅ HTTP server is running")
    
    # Determine which files to process
    files_to_process = []
    
    if args.all:
        files_to_process = list(TWITTER_FILES.keys())
    else:
        # Process specific files based on arguments
        for file_name in TWITTER_FILES.keys():
            arg_name = file_name.replace('.js', '').replace('-', '_')
            if getattr(args, arg_name, False):
                files_to_process.append(file_name)
    
    if not files_to_process:
        print("❌ No files specified to process")
        print("💡 Use --all to process all files, or specify individual files like --tweets --likes")
        sys.exit(1)
    
    print(f"\n🎯 Processing {len(files_to_process)} Twitter data files...")
    print(f"⚙️  Auto-execute: {args.auto_execute}")
    print(f"🔑 Trust distance: {args.trust_distance}")
    print(f"🔐 Public key: {args.pub_key}")
    
    # Process each file
    successful = 0
    failed = 0
    
    for i, file_name in enumerate(files_to_process):
        if process_twitter_file(file_name, args.auto_execute, args.trust_distance, args.pub_key):
            successful += 1
        else:
            failed += 1
        
        # Add a delay between requests to prevent database lock contention
        if i < len(files_to_process) - 1:  # Don't delay after the last file
            print("⏳ Waiting for database operations to complete...")
            time.sleep(5)  # 5 second delay between requests
    
    # Summary
    print(f"\n📊 Processing Summary:")
    print(f"   ✅ Successful: {successful}")
    print(f"   ❌ Failed: {failed}")
    print(f"   📁 Total: {len(files_to_process)}")
    
    if successful > 0:
        print(f"\n🎉 Successfully processed {successful} Twitter data files!")
        print("💡 You can now query your data through the web interface at http://localhost:9001")
    
    if failed > 0:
        print(f"\n⚠️  {failed} files failed to process. Check the error messages above.")

if __name__ == "__main__":
    main()
