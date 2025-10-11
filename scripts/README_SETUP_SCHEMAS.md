# Schema Setup Script

## Overview

The `setup_sample_schemas.py` script is a comprehensive tool for:
1. Approving all base schemas
2. Populating base schemas with sample data
3. Approving declarative/transform schemas (which auto-populate via backfill)

## Usage

```bash
# Make sure the HTTP server is running first
./run_http_server.sh

# In another terminal, run the setup script
python3 scripts/setup_sample_schemas.py
```

## What It Does

### Step 1: Approve and Populate Base Schemas (8 schemas)

For each base schema, the script:
1. Approves the schema
2. Immediately creates sample data for that schema

Schemas processed in order:
- User
- Product
- Order
- ProductReview
- UserActivity
- Message
- Event
- BlogPost

Sample data created per schema:
- **5 Users** - with various statuses (active, inactive, pending)
- **10 Products** - across multiple categories and brands
- **8 Orders** - with different statuses and payment methods
- **15 Product Reviews** - with ratings and verified purchases
- **20 User Activities** - various activity types (login, purchase, etc.)
- **12 Messages** - across 3 conversations
- **8 Events** - conferences, workshops, meetups
- **10 Blog Posts** - with multiple authors and tags

**Total: ~88 base records**

### Step 2: Approve Declarative Schemas (17 schemas)

These schemas auto-populate via transform backfills:

**Product Domain:**
- ProductTagIndex
- ProductCategoryIndex
- ProductBrandIndex
- ProductReviewStats
- ProductReviewUserIndex

**Order Domain:**
- UserOrderStats
- OrderStatusIndex

**Event Domain:**
- EventCategoryIndex
- EventOrganizerIndex

**Message Domain:**
- MessageWordIndex
- MessageSenderIndex
- ConversationMessageStats

**Blog Domain:**
- BlogPostWordIndex
- BlogPostTagIndex
- BlogPostAuthorIndex

**User Domain:**
- UserByStatus
- UserActivityTypeIndex

### Step 3: Verify Data
Queries all base schemas to confirm data was created successfully.

## Schema Files

All schema definitions are located in `/available_schemas/`:

### Base Schema Files
- BlogPost.json
- User.json
- Product.json
- ProductReview.json
- Order.json
- Event.json
- Message.json
- UserActivity.json

### Declarative Schema Files
- ProductTagIndex.json
- ProductCategoryIndex.json
- ProductBrandIndex.json
- ProductReviewStats.json
- ProductReviewUserIndex.json
- UserOrderStats.json
- OrderStatusIndex.json
- EventCategoryIndex.json
- EventOrganizerIndex.json
- MessageWordIndex.json
- MessageSenderIndex.json
- ConversationMessageStats.json
- BlogPostTagIndex.json
- BlogPostAuthorIndex.json
- BlogPostWordIndex.json
- UserByStatus.json
- UserActivityTypeIndex.json

## Documentation

See the `/docs/` directory for detailed documentation:

- **README_DECLARATIVE_SCHEMAS.md** - Detailed descriptions of each declarative schema
- **SCHEMA_SUMMARY.md** - Quick reference guide with tables
- **transform_functions.md** - Complete transform function reference

## Requirements

- HTTP server must be running on port 9001
- Python 3 with `requests` library installed:
  ```bash
  pip install requests
  ```

## Expected Output

```
============================================================
DataFold Sample Schema Setup
============================================================
✅ HTTP server is running on localhost:9001

📋 Step 1: Approving and populating base schemas...
------------------------------------------------------------

👤 User Schema
  ✅ Approved schema: User

📝 Creating 5 User records...
  ✅ Created user: johndoe1
  ... (etc)

📦 Product Schema
  ✅ Approved schema: Product

📝 Creating 10 Product records...
  ✅ Created product: Wireless Mouse #1
  ... (etc)

📋 Step 2: Approving declarative/transform schemas...
------------------------------------------------------------
ℹ️  These schemas will auto-populate via backfill transforms
  ✅ Approved schema: BlogPostWordIndex
  ... (etc)

📋 Step 3: Verifying data in base schemas...
------------------------------------------------------------
  ✅ User: 5 records
  ✅ Product: 10 records
  ... (etc)

============================================================
✅ Setup complete!
📊 Base schema records: 88
📊 Base schemas: 8
📊 Declarative schemas: 17
📊 Total schemas: 25
============================================================

ℹ️  Declarative schemas are populating in the background via transforms.
   Use the UI or query API to check their status.
```

## Troubleshooting

### Server Not Running
```
❌ Could not connect to HTTP server
💡 Make sure the HTTP server is running: ./run_http_server.sh
```

**Solution**: Run `./run_http_server.sh` in another terminal first.

### Schema Already Approved
If schemas are already approved, the script will continue without error.

### Transform Backfill Status
Declarative schemas populate asynchronously via transforms. To check their status:
- Open the UI at http://localhost:9001
- Navigate to the Schema Management page
- Check the backfill status for each declarative schema

## Related Scripts

- **manage_blogposts.py** - Specific management for BlogPost schema
- **manage_user_activity.py** - Specific management for UserActivity schema
- **setup_sample_schemas.py** - This comprehensive setup script (recommended)

## Notes

- The script is idempotent - safe to run multiple times
- Declarative schemas automatically update when base data changes
- All timestamps use ISO 8601 format with 'Z' suffix
- Sample data includes realistic variety for testing queries

