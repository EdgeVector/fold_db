# Twitter Data Ingestion Guide

This guide explains how to use your downloaded Twitter data with the DataFold ingestion system.

## Overview

The DataFold ingestion system can automatically process your Twitter archive data, analyze it with AI, create appropriate database schemas, and store the data for querying. This is particularly useful for:

- Analyzing your Twitter activity over time
- Creating searchable archives of your posts, likes, and interactions
- Building applications that work with your social media data
- Research and personal data analysis

## Twitter Data Structure

Your Twitter archive contains various types of data in JavaScript format:

### Key Data Files

- **`tweets.js`** - Your tweets and replies
- **`like.js`** - Tweets you've liked
- **`following.js`** - Accounts you follow
- **`follower.js`** - Your followers
- **`account.js`** - Your account information
- **`profile.js`** - Profile details
- **`direct-messages.js`** - Direct messages
- **`block.js`** - Blocked accounts
- **`mute.js`** - Muted accounts

### Data Format

Twitter exports are in JavaScript format like this:
```javascript
window.YTD.tweets.part0 = [
  {
    "tweet": {
      "id": "1234567890",
      "full_text": "Your tweet content here",
      "created_at": "Mon Oct 20 17:11:38 +0000 2025",
      // ... more fields
    }
  }
]
```

## Processing Methods

### Method 1: Using the Web Interface

1. **Start the server:**
   ```bash
   ./run_http_server.sh
   ```

2. **Open the web interface:**
   Navigate to http://localhost:9001

3. **Go to the Ingestion tab**

4. **Load sample Twitter data:**
   Click the "Twitter" button to load sample data, or paste your own Twitter data

5. **Configure settings:**
   - Auto-execute: Check to automatically store data
   - Trust distance: Set to 0 for immediate processing
   - Public key: Use "default"

6. **Process the data:**
   Click "Process Ingestion" to analyze and store your data

### Method 2: Using the Processing Scripts

#### Quick Example

```bash
# Run the example script to process tweets
python scripts/example_twitter_ingestion.py
```

#### Process All Twitter Data

```bash
# Process all available Twitter data files
python scripts/process_twitter_data.py --all
```

#### Process Specific Data Types

```bash
# Process only tweets and likes
python scripts/process_twitter_data.py --tweets --likes

# Process with custom settings
python scripts/process_twitter_data.py --tweets --auto-execute --trust-distance 1
```

### Method 3: Manual Processing

You can also manually process Twitter data by:

1. **Extract JSON from JavaScript files:**
   ```python
   import json
   
   # Load Twitter file
   with open('sample_data/twitter/data/tweets.js', 'r') as f:
       content = f.read()
   
   # Extract JSON (skip JavaScript wrapper)
   start_idx = content.find('[')
   end_idx = content.rfind(']') + 1
   json_str = content[start_idx:end_idx]
   tweets_data = json.loads(json_str)
   ```

2. **Send to ingestion API:**
   ```bash
   curl -X POST http://localhost:9001/api/ingestion/process \
     -H "Content-Type: application/json" \
     -d '{
       "data": [your_json_data_here],
       "auto_execute": true,
       "trust_distance": 0,
       "pub_key": "default"
     }'
   ```

## How It Works

### 1. Data Analysis
The ingestion system uses AI to analyze your Twitter data and determine:
- What type of data it is (tweets, likes, followers, etc.)
- What schema structure would be appropriate
- How to map the data fields

### 2. Schema Creation
Based on the analysis, the system either:
- Uses an existing schema if one matches
- Creates a new schema automatically

### 3. Data Storage
The data is stored in the database using the appropriate schema, making it queryable through the DataFold interface.

## Example Schemas Created

### Tweets Schema
```json
{
  "name": "Tweet",
  "descriptive_name": "Twitter Posts and Replies",
  "key": {"range_field": "id"},
  "fields": ["id", "full_text", "created_at", "user_id"],
  "field_topologies": {
    "id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "full_text": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "created_at": {"root": {"type": "Primitive", "value": "String", "classifications": ["date"]}},
    "user_id": {"root": {"type": "Primitive", "value": "String", "classifications": ["username"]}}
  }
}
```

### Likes Schema
```json
{
  "name": "Like",
  "descriptive_name": "Liked Tweets",
  "key": {"range_field": "tweet_id"},
  "fields": ["tweet_id", "full_text", "expanded_url"],
  "field_topologies": {
    "tweet_id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "full_text": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "expanded_url": {"root": {"type": "Primitive", "value": "String", "classifications": ["url"]}}
  }
}
```

## Querying Your Data

After ingestion, you can query your Twitter data through the web interface:

1. **Go to the Query tab** in the web interface
2. **Select your schema** (e.g., "Tweet", "Like", "Following")
3. **Choose fields** to retrieve
4. **Add filters** if needed
5. **Execute the query**

### Example Queries

**Get all tweets:**
```json
{
  "type": "query",
  "schema": "Tweet",
  "fields": ["id", "full_text", "created_at"]
}
```

**Search tweets by text:**
```json
{
  "type": "query",
  "schema": "Tweet",
  "fields": ["id", "full_text", "created_at"],
  "filter": {
    "range": {"contains": "your_search_term"}
  }
}
```

**Get recent tweets:**
```json
{
  "type": "query",
  "schema": "Tweet",
  "fields": ["id", "full_text", "created_at"],
  "filter": {
    "range": {"gte": "2024-01-01"}
  }
}
```

## Troubleshooting

### Server Not Running
```bash
# Start the server
./run_http_server.sh
```

### Ingestion Fails
- Check that the server is running on port 9001
- Verify your Twitter data files are in the correct format
- Check the server logs in `server.log`

### Data Not Appearing
- Ensure "Auto-execute" is enabled
- Check that the schema was created and approved
- Verify the ingestion was successful (check the response)

### Large Files
For large Twitter archives:
- Process files individually rather than all at once
- Consider processing in smaller batches
- Monitor server memory usage

## Best Practices

1. **Start Small:** Process a few tweets first to test the setup
2. **Backup Data:** Keep your original Twitter archive files
3. **Monitor Resources:** Large datasets may require significant memory
4. **Use Filters:** Query with filters to avoid overwhelming results
5. **Schema Review:** Review auto-generated schemas and adjust if needed

## Advanced Usage

### Custom Data Processing
You can modify the processing scripts to:
- Filter specific types of tweets
- Extract specific fields
- Combine multiple data sources
- Apply custom transformations

### Integration with Other Tools
The ingested data can be used with:
- Data analysis tools
- Visualization libraries
- Machine learning frameworks
- Custom applications

## Support

If you encounter issues:
1. Check the server logs (`server.log`)
2. Verify your Twitter data format
3. Test with the example scripts first
4. Check the DataFold documentation for schema types and query syntax
