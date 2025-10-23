# Twitter Data Ingestion - Quick Start

This guide shows you how to use your downloaded Twitter data with the DataFold ingestion system.

## Quick Start

### 1. Start the Server
```bash
./run_http_server.sh
```

### 2. Process Twitter Data
```bash
# Process all Twitter data files
python3 scripts/process_twitter_data.py --all

# Or process specific files
python3 scripts/process_twitter_data.py --tweets --likes --following
```

### 3. Query Your Data
- Open http://localhost:9001 in your browser
- Go to the Query tab
- Select your Twitter schemas (Tweet, Like, Following, etc.)
- Query your data!

## What Gets Created

The ingestion system automatically creates schemas for your Twitter data:

- **Tweet** - Your tweets and replies
- **Like** - Tweets you've liked  
- **Following** - Accounts you follow
- **Follower** - Your followers
- **Account** - Your account information
- **Profile** - Profile details
- **DirectMessage** - Direct messages
- **Block** - Blocked accounts
- **Mute** - Muted accounts

## Example Queries

**Get all your tweets:**
```json
{
  "type": "query",
  "schema": "Tweet",
  "fields": ["id", "full_text", "created_at"]
}
```

**Search tweets by content:**
```json
{
  "type": "query", 
  "schema": "Tweet",
  "fields": ["id", "full_text", "created_at"],
  "filter": {
    "range": {"contains": "search_term"}
  }
}
```

## Files Created

- `scripts/process_twitter_data.py` - Main processing script
- `scripts/example_twitter_ingestion.py` - Simple example
- `docs/TWITTER_DATA_INGESTION.md` - Detailed documentation

## Need Help?

1. Check the detailed guide: `docs/TWITTER_DATA_INGESTION.md`
2. Run the example script: `python3 scripts/example_twitter_ingestion.py`
3. Check server logs: `tail -f server.log`

## Your Twitter Data Location

Your Twitter archive should be in: `sample_data/twitter/data/`

The system will automatically process files like:
- `tweets.js` - Your tweets
- `like.js` - Liked tweets
- `following.js` - Accounts you follow
- `account.js` - Account info
- And many more!

Happy querying! 🐦✨
