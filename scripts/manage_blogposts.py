#!/usr/bin/env python3
"""
Comprehensive DataFold Blog Post Management Script

This script handles the complete workflow:
1. Uses the existing HTTP server (if running)
2. Adds dummy blog posts to the database via HTTP API using curl
3. Queries and displays the blog posts

Usage: python scripts/manage_blogposts.py
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

def create_blog_post_via_curl(title, content, author, publish_date, tags):
    """Create a single blog post via curl command."""
    mutation_data = {
        "type": "mutation",
        "schema": "BlogPost",
        "mutation_type": "create",
        "data": {
            "title": title,
            "content": content,
            "author": author,
            "publish_date": publish_date,
            "tags": tags
        }
    }
    
    print(f"📝 Creating via curl: {title}")
    
    # Execute curl command
    curl_cmd = [
        "curl", "-X", "POST", "http://localhost:9001/api/mutation",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(mutation_data),
        "-s"  # Silent mode
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode == 0:
            response = json.loads(result.stdout)
            if response.get("success"):
                print(f"✅ Created: {title}")
                return True
            else:
                print(f"❌ Failed to create '{title}': {response.get('error', 'Unknown error')}")
                return False
        else:
            print(f"❌ Curl command failed for '{title}': {result.stderr}")
            return False
            
    except subprocess.TimeoutExpired:
        print(f"❌ Timeout creating '{title}'")
        return False
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON response for '{title}': {result.stdout}")
        return False
    except Exception as e:
        print(f"❌ Error creating '{title}': {e}")
        return False

def add_dummy_blog_posts():
    """Add dummy blog posts to the database."""
    print("\n📚 Adding dummy blog posts...")
    
    # Sample data for blog posts
    sample_titles = [
        "Getting Started with DataFold",
        "Understanding Range Schemas",
        "Best Practices for Data Ingestion",
        "Advanced Query Patterns",
        "Security and Permissions in DataFold",
        "Building Scalable Data Applications",
        "Real-time Data Processing",
        "Data Transformation Techniques",
        "Performance Optimization Tips",
        "Deployment Strategies"
    ]
    
    sample_contents = [
        "DataFold is a powerful distributed database system that enables efficient data storage and retrieval across multiple nodes. This post will guide you through the basics of getting started with DataFold, including installation, configuration, and your first data operations.",
        
        "Range schemas are a key feature of DataFold that allow you to organize data based on a specific field. This post explores how range schemas work, their benefits, and how to implement them effectively in your applications.",
        
        "Data ingestion is a critical component of any data system. This post covers best practices for ingesting data into DataFold, including error handling, validation, and performance optimization techniques.",
        
        "DataFold supports various query patterns that can help you retrieve data efficiently. This post demonstrates advanced query patterns including filtering, sorting, and aggregation operations.",
        
        "Security is paramount in any data system. This post explains how DataFold handles permissions, authentication, and data access control to ensure your data remains secure.",
        
        "Building scalable data applications requires careful planning and implementation. This post provides insights into designing and building applications that can handle large-scale data operations with DataFold.",
        
        "Real-time data processing is essential for many modern applications. This post explores how DataFold supports real-time data processing and streaming operations.",
        
        "Data transformation is a common requirement in data applications. This post covers various techniques for transforming data within DataFold, including custom transforms and data mapping.",
        
        "Performance is crucial for data applications. This post provides tips and techniques for optimizing the performance of your DataFold applications.",
        
        "Deploying DataFold applications requires careful consideration of infrastructure and configuration. This post covers various deployment strategies and best practices."
    ]
    
    sample_authors = [
        "Alice Johnson",
        "Bob Smith", 
        "Carol Davis",
        "David Wilson",
        "Eva Brown"
    ]
    
    sample_tags = [
        ["tutorial", "beginners", "datafold"],
        ["schemas", "range", "advanced"],
        ["ingestion", "best-practices", "performance"],
        ["queries", "patterns", "advanced"],
        ["security", "permissions", "authentication"],
        ["scalability", "architecture", "design"],
        ["real-time", "streaming", "processing"],
        ["transforms", "data-processing", "mapping"],
        ["performance", "optimization", "tips"],
        ["deployment", "infrastructure", "configuration"]
    ]
    
    # Generate publish dates (last 30 days)
    base_date = datetime.now() - timedelta(days=30)
    
    # Create blog posts
    successful_posts = 0
    total_posts = len(sample_titles)
    
    for i in range(total_posts):
        # Generate a random publish date within the last 30 days
        days_ago = random.randint(0, 30)
        publish_date = (base_date + timedelta(days=days_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")
        
        # Create the blog post
        success = create_blog_post_via_curl(
            sample_titles[i],
            sample_contents[i],
            random.choice(sample_authors),
            publish_date,
            sample_tags[i]
        )
        
        if success:
            successful_posts += 1
        
        # Small delay to avoid overwhelming the server
        time.sleep(0.5)
    
    print(f"\n📊 Summary: Created {successful_posts} out of {total_posts} blog posts")
    return successful_posts

def query_blog_posts_via_curl():
    """Query all blog posts via curl command."""
    print("\n🔍 Querying blog posts via curl...")
    
    query_data = {
        "type": "query",
        "schema": "BlogPost",
        "fields": ["title", "author", "publish_date", "tags"]
    }
    
    curl_cmd = [
        "curl", "-X", "POST", "http://localhost:9001/api/query",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(query_data),
        "-s"  # Silent mode
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode == 0:
            response = json.loads(result.stdout)
            # Check for both 'success' and 'data' fields
            if response.get("success") or response.get("data"):
                return response.get("data", response.get("results", []))
            else:
                print(f"❌ Query failed: {response.get('error', 'Unknown error')}")
                return None
        else:
            print(f"❌ Curl query command failed: {result.stderr}")
            return None
            
    except subprocess.TimeoutExpired:
        print("❌ Query timeout")
        return None
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON response: {result.stdout}")
        return None
    except Exception as e:
        print(f"❌ Error querying blog posts: {e}")
        return None

def display_blog_posts(posts):
    """Display blog posts in a formatted way."""
    if not posts:
        print("❌ No blog posts found.")
        return
    
    print(f"\n📖 Found {len(posts)} blog posts:\n")
    
    # Handle the specific format where posts is a list with one item containing field dictionaries
    if isinstance(posts, list) and len(posts) > 0:
        post_data = posts[0]  # Get the first (and only) item
        
        if isinstance(post_data, dict) and 'title' in post_data:
            # Extract all the publish dates from the title field
            publish_dates = list(post_data['title'].keys())
            publish_dates.sort()  # Sort by date
            
            print("=" * 80)
            for i, publish_date in enumerate(publish_dates, 1):
                title = post_data['title'].get(publish_date, 'No title')
                author = post_data['author'].get(publish_date, 'Unknown') if 'author' in post_data else 'Unknown'
                tags = post_data['tags'].get(publish_date, []) if 'tags' in post_data else []
                
                print(f"{i:2d}. {title}")
                print(f"    👤 Author: {author}")
                print(f"    📅 Published: {publish_date}")
                print(f"    🏷️  Tags: {', '.join(tags) if isinstance(tags, list) else str(tags)}")
                print("-" * 80)
        else:
            print("❌ Unexpected data structure")
            print(json.dumps(posts, indent=2))
    else:
        print("❌ No blog posts found or unexpected format")
        print(json.dumps(posts, indent=2))

def show_curl_example():
    """Show what the actual curl command looks like for creating blog posts."""
    print("\n🔍 Example curl command for creating blog posts:")
    print("=" * 60)
    
    # Example mutation payload
    mutation_data = {
        "type": "mutation",
        "schema": "BlogPost",
        "mutation_type": "create",
        "data": {
            "title": "Getting Started with DataFold",
            "content": "DataFold is a powerful distributed database system...",
            "author": "Alice Johnson",
            "publish_date": "2025-08-26T12:05:08Z",
            "tags": ["tutorial", "beginners", "datafold"]
        }
    }
    
    print("📝 The mutation payload:")
    print(json.dumps(mutation_data, indent=2))
    print()
    
    print("💡 The curl command:")
    print("```bash")
    print("curl -X POST http://localhost:9001/api/mutation \\")
    print("  -H 'Content-Type: application/json' \\")
    print(f"  -d '{json.dumps(mutation_data)}'")
    print("```")
    print()
    
    print("✅ Why curl works now:")
    print("- No authentication required")
    print("- Simple JSON payload")
    print("- Direct HTTP POST")
    print("- No signing or key management")

def main():
    """Main function to manage blog posts."""
    print("🎯 DataFold Blog Post Management Script (curl Version)")
    print("=" * 60)
    
    # Check if HTTP server is running
    if not check_http_server():
        print("❌ HTTP server is not running. Exiting.")
        print("💡 Start the HTTP server with: ./run_http_server.sh")
        sys.exit(1)
    
    try:
        # Show curl example
        show_curl_example()
        
        # Add blog posts
        successful_posts = add_dummy_blog_posts()
        
        if successful_posts > 0:
            # Query and display blog posts
            posts = query_blog_posts_via_curl()
            if posts is not None:
                display_blog_posts(posts)
            else:
                print("❌ Failed to query blog posts.")
        
        print(f"\n🎉 Blog post management completed!")
        print(f"📝 {successful_posts} blog posts were created and are now available in the database.")
        print(f"🌐 You can also view them at http://localhost:9001")
        
    except Exception as e:
        print(f"❌ Error during blog post management: {e}")

if __name__ == "__main__":
    main()
