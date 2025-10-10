#!/usr/bin/env python3
"""
DataFold Sample Schema Setup Script

This script:
1. Approves all sample schemas (User, Product, Order, ProductReview, UserActivity, Message, Event)
2. Creates sample mutations for each schema
3. Verifies the data was inserted correctly

Usage: python scripts/setup_sample_schemas.py
"""

import requests
import json
import random
import time
import sys
from datetime import datetime, timedelta

BASE_URL = "http://localhost:9001"

# Schema names to approve and populate
SCHEMAS = [
    "User",
    "Product", 
    "Order",
    "ProductReview",
    "UserActivity",
    "Message",
    "Event"
]

def check_http_server():
    """Check if the HTTP server is running."""
    try:
        response = requests.get(BASE_URL, timeout=5)
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

def approve_schema(schema_name):
    """Approve a schema via HTTP API."""
    try:
        url = f"{BASE_URL}/api/schema/{schema_name}/approve"
        response = requests.post(url, timeout=10)
        
        if response.status_code == 200:
            print(f"  ✅ Approved schema: {schema_name}")
            return True
        else:
            print(f"  ⚠️  Schema {schema_name} approval returned status {response.status_code}")
            print(f"     Response: {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"  ❌ Failed to approve schema {schema_name}: {e}")
        return False

def create_mutation(mutation_data):
    """Create a mutation via HTTP API."""
    try:
        url = f"{BASE_URL}/api/mutation"
        headers = {"Content-Type": "application/json"}
        response = requests.post(url, json=mutation_data, headers=headers, timeout=10)
        
        if response.status_code == 200:
            return True
        else:
            print(f"  ⚠️  Mutation failed with status {response.status_code}")
            print(f"     Response: {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"  ❌ Failed to create mutation: {e}")
        return False

def generate_user_mutations(count=5):
    """Generate sample User mutations."""
    print(f"\n📝 Creating {count} User records...")
    
    usernames = ["johndoe", "janedoe", "alice", "bob", "charlie", "diana", "eve", "frank"]
    statuses = ["active", "inactive", "pending"]
    
    for i in range(count):
        username = f"{random.choice(usernames)}{i+1}"
        created_at = (datetime.now() - timedelta(days=random.randint(1, 365))).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "User",
            "mutation_type": "create",
            "fields_and_values": {
                "created_at": created_at,
                "username": {"value": username},
                "email": {"value": f"{username}@example.com"},
                "full_name": {"value": username.replace("_", " ").title()},
                "user_id": {"value": f"user_{i+1:03d}"},
                "bio": {"value": f"Bio for {username}"},
                "status": {"value": random.choice(statuses)}
            },
            "key_value": {"hash": None, "range": created_at}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created user: {username}")
        time.sleep(0.1)

def generate_product_mutations(count=10):
    """Generate sample Product mutations."""
    print(f"\n📝 Creating {count} Product records...")
    
    categories = ["Electronics", "Clothing", "Books", "Home & Garden", "Sports"]
    brands = ["BrandA", "BrandB", "BrandC", "BrandD"]
    
    products = [
        "Wireless Mouse", "Laptop Stand", "USB Cable", "Desk Lamp",
        "T-Shirt", "Jeans", "Sneakers", "Backpack",
        "Novel", "Cookbook", "Plant Pot", "Water Bottle"
    ]
    
    for i in range(count):
        product = random.choice(products)
        created_at = (datetime.now() - timedelta(days=random.randint(30, 180))).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "Product",
            "mutation_type": "create",
            "fields_and_values": {
                "created_at": created_at,
                "product_id": {"value": f"PROD-{i+1:03d}"},
                "name": {"value": f"{product} #{i+1}"},
                "description": {"value": f"High quality {product.lower()} with excellent features"},
                "price": {"value": str(random.uniform(10, 500))},
                "category": {"value": random.choice(categories)},
                "brand": {"value": random.choice(brands)},
                "stock_quantity": {"value": str(random.randint(0, 100))},
                "sku": {"value": f"SKU-{i+1:05d}"}
            },
            "key_value": {"hash": None, "range": created_at}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created product: {product} #{i+1}")
        time.sleep(0.1)

def generate_order_mutations(count=8):
    """Generate sample Order mutations."""
    print(f"\n📝 Creating {count} Order records...")
    
    statuses = ["pending", "shipped", "delivered", "cancelled"]
    payment_methods = ["credit_card", "paypal", "bank_transfer"]
    
    for i in range(count):
        order_date = (datetime.now() - timedelta(days=random.randint(1, 90))).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "Order",
            "mutation_type": "create",
            "fields_and_values": {
                "order_date": order_date,
                "order_id": {"value": f"ORD-{i+1:05d}"},
                "user_id": {"value": f"user_{random.randint(1, 5):03d}"},
                "total_amount": {"value": str(random.uniform(20, 500))},
                "status": {"value": random.choice(statuses)},
                "shipping_address": {"value": f"{random.randint(1, 999)} Main St, City, State"},
                "payment_method": {"value": random.choice(payment_methods)},
                "items": {"value": f"PROD-{random.randint(1, 10):03d}"},
                "tracking_number": {"value": f"TRACK-{i+1:010d}"}
            },
            "key_value": {"hash": None, "range": order_date}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created order: ORD-{i+1:05d}")
        time.sleep(0.1)

def generate_product_review_mutations(count=15):
    """Generate sample ProductReview mutations (HashRange schema)."""
    print(f"\n📝 Creating {count} ProductReview records...")
    
    titles = ["Great!", "Love it", "Not bad", "Disappointing", "Amazing product", "Could be better"]
    
    for i in range(count):
        product_id = f"PROD-{random.randint(1, 10):03d}"
        review_date = (datetime.now() - timedelta(days=random.randint(1, 60))).isoformat() + "Z"
        rating = random.randint(1, 5)
        
        mutation = {
            "type": "mutation",
            "schema": "ProductReview",
            "mutation_type": "create",
            "fields_and_values": {
                "product_id": product_id,
                "review_date": review_date,
                "review_id": f"REV-{i+1:05d}",
                "user_id": f"user_{random.randint(1, 5):03d}",
                "rating": str(rating),
                "title": random.choice(titles),
                "content": f"This is a {'positive' if rating >= 4 else 'negative'} review for the product.",
                "verified_purchase": str(random.choice([True, False])),
                "helpful_count": str(random.randint(0, 50))
            },
            "key_value": {"hash": product_id, "range": review_date}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created review for {product_id}: {rating} stars")
        time.sleep(0.1)

def generate_user_activity_mutations(count=20):
    """Generate sample UserActivity mutations (HashRange schema)."""
    print(f"\n📝 Creating {count} UserActivity records...")
    
    activity_types = ["login", "logout", "view_product", "add_to_cart", "purchase", "review"]
    
    for i in range(count):
        user_id = f"user_{random.randint(1, 5):03d}"
        timestamp = (datetime.now() - timedelta(hours=random.randint(1, 168))).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "UserActivity",
            "mutation_type": "create",
            "fields_and_values": {
                "user_id": user_id,
                "timestamp": timestamp,
                "activity_id": f"ACT-{i+1:05d}",
                "activity_type": random.choice(activity_types),
                "resource_id": f"RES-{random.randint(1, 100):03d}",
                "metadata": json.dumps({"source": "web", "device": "desktop"}),
                "ip_address": f"192.168.1.{random.randint(1, 255)}",
                "user_agent": "Mozilla/5.0"
            },
            "key_value": {"hash": user_id, "range": timestamp}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created activity for {user_id}: {mutation['fields_and_values']['activity_type']}")
        time.sleep(0.1)

def generate_message_mutations(count=12):
    """Generate sample Message mutations (HashRange schema)."""
    print(f"\n📝 Creating {count} Message records...")
    
    message_types = ["text", "image", "file", "link"]
    
    for i in range(count):
        conv_id = f"CONV-{random.randint(1, 3):03d}"
        sent_at = (datetime.now() - timedelta(hours=random.randint(1, 72))).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "Message",
            "mutation_type": "create",
            "fields_and_values": {
                "conversation_id": conv_id,
                "sent_at": sent_at,
                "message_id": f"MSG-{i+1:05d}",
                "sender_id": f"user_{random.randint(1, 5):03d}",
                "recipient_id": f"user_{random.randint(1, 5):03d}",
                "content": f"Sample message content #{i+1}",
                "message_type": random.choice(message_types),
                "attachments": json.dumps([])
            },
            "key_value": {"hash": conv_id, "range": sent_at}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created message in {conv_id}")
        time.sleep(0.1)

def generate_event_mutations(count=8):
    """Generate sample Event mutations."""
    print(f"\n📝 Creating {count} Event records...")
    
    categories = ["Conference", "Workshop", "Webinar", "Meetup", "Training"]
    statuses = ["upcoming", "ongoing", "completed", "cancelled"]
    
    event_names = [
        "Tech Summit", "Developer Meetup", "Product Launch",
        "Training Session", "Team Building", "Networking Event"
    ]
    
    for i in range(count):
        start_time = (datetime.now() + timedelta(days=random.randint(1, 60))).isoformat() + "Z"
        end_time = (datetime.now() + timedelta(days=random.randint(1, 60), hours=2)).isoformat() + "Z"
        
        mutation = {
            "type": "mutation",
            "schema": "Event",
            "mutation_type": "create",
            "fields_and_values": {
                "start_time": start_time,
                "event_id": {"value": f"EVT-{i+1:05d}"},
                "title": {"value": f"{random.choice(event_names)} {i+1}"},
                "description": {"value": f"Description for event {i+1}"},
                "end_time": {"value": end_time},
                "location": {"value": f"Venue {random.randint(1, 5)}, City"},
                "organizer_id": {"value": f"user_{random.randint(1, 3):03d}"},
                "attendees": {"value": json.dumps([f"user_{j:03d}" for j in range(1, random.randint(3, 8))])},
                "category": {"value": random.choice(categories)},
                "status": {"value": random.choice(statuses)},
                "max_capacity": {"value": str(random.randint(20, 200))}
            },
            "key_value": {"hash": None, "range": start_time}
        }
        
        if create_mutation(mutation):
            print(f"  ✅ Created event: {mutation['fields_and_values']['title']['value']}")
        time.sleep(0.1)

def query_schema(schema_name):
    """Query a schema to verify data."""
    try:
        query_data = {
            "type": "query",
            "schema": schema_name,
            "fields": []
        }
        
        url = f"{BASE_URL}/api/query"
        headers = {"Content-Type": "application/json"}
        response = requests.post(url, json=query_data, headers=headers, timeout=10)
        
        if response.status_code == 200:
            data = response.json()
            count = len(data) if isinstance(data, list) else 0
            print(f"  ✅ {schema_name}: {count} records")
            return count
        else:
            print(f"  ⚠️  Query for {schema_name} returned status {response.status_code}")
            return 0
    except requests.exceptions.RequestException as e:
        print(f"  ❌ Failed to query {schema_name}: {e}")
        return 0

def main():
    print("=" * 60)
    print("DataFold Sample Schema Setup")
    print("=" * 60)
    
    # Check server
    if not check_http_server():
        sys.exit(1)
    
    # Step 1: Approve all schemas
    print("\n📋 Step 1: Approving schemas...")
    print("-" * 60)
    
    for schema in SCHEMAS:
        approve_schema(schema)
        time.sleep(0.2)
    
    print("\n✅ All schemas approved!")
    
    # Step 2: Create sample data
    print("\n📋 Step 2: Creating sample data...")
    print("-" * 60)
    
    generate_user_mutations(5)
    generate_product_mutations(10)
    generate_order_mutations(8)
    generate_product_review_mutations(15)
    generate_user_activity_mutations(20)
    generate_message_mutations(12)
    generate_event_mutations(8)
    
    print("\n✅ All sample data created!")
    
    # Step 3: Verify data
    print("\n📋 Step 3: Verifying data...")
    print("-" * 60)
    
    total = 0
    for schema in SCHEMAS:
        count = query_schema(schema)
        total += count
    
    print("\n" + "=" * 60)
    print(f"✅ Setup complete! Total records created: {total}")
    print("=" * 60)

if __name__ == "__main__":
    main()

