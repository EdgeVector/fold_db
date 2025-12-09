
import os
import json
from pathlib import Path

# Setup directory
base_dir = Path("sample_data/twitter/data")
base_dir.mkdir(parents=True, exist_ok=True)

# Define dummy data
files = {
    "tweets.js": {
        "prefix": "window.YTD.tweets.part0 = ",
        "data": [
            {
                "tweet": {
                    "edit_info": {
                        "initial": {
                            "editTweetIds": ["1234567890"],
                            "editableUntil": "2020-01-01T00:00:00.000Z",
                            "editsRemaining": "5",
                            "isEditEligible": True
                        }
                    },
                    "retweeted": False,
                    "source": "<a href=\"http://twitter.com/download/iphone\" rel=\"nofollow\">Twitter for iPhone</a>",
                    "entities": {
                        "hashtags": [],
                        "symbols": [],
                        "user_mentions": [],
                        "urls": []
                    },
                    "display_text_range": ["0", "5"],
                    "favorite_count": "0",
                    "id_str": "1234567890",
                    "truncated": False,
                    "retweet_count": "0",
                    "id": "1234567890",
                    "possibly_sensitive": False,
                    "created_at": "Mon Jan 01 12:00:00 +0000 2020",
                    "favorited": False,
                    "full_text": "Hello world",
                    "lang": "en"
                }
            }
        ]
    },
    "following.js": {
        "prefix": "window.YTD.following.part0 = ",
        "data": [
            {
                "following": {
                    "accountId": "111111",
                    "userLink": "https://twitter.com/intent/user?user_id=111111"
                }
            }
        ]
    },
    "like.js": {
        "prefix": "window.YTD.like.part0 = ",
        "data": [
            {
                "like": {
                    "tweetId": "999999",
                    "fullText": "Liked tweet text",
                    "expandedUrl": "https://twitter.com/i/web/status/999999"
                }
            }
        ]
    },
    "account.js": {
        "prefix": "window.YTD.account.part0 = ",
        "data": [
            {
                "account": {
                    "email": "test@example.com",
                    "createdVia": "web",
                    "username": "testuser",
                    "accountId": "12345",
                    "createdAt": "2010-01-01T00:00:00.000Z",
                    "accountDisplayName": "Test User"
                }
            }
        ]
    },
    "direct-messages.js": {
        "prefix": "window.YTD.direct_message.part0 = ",
        "data": [
            {
                "dmConversation": {
                    "conversationId": "111-222",
                    "messages": [
                        {
                            "messageCreate": {
                                "recipientId": "222",
                                "senderId": "111",
                                "text": "Hello DM",
                                "createdAt": "2020-01-01T12:00:00.000Z"
                            }
                        }
                    ]
                }
            }
        ]
    }
}

for filename, content in files.items():
    file_path = base_dir / filename
    with open(file_path, "w") as f:
        json_str = json.dumps(content["data"], indent=2)
        f.write(f"{content['prefix']}{json_str}")
    print(f"Created {file_path}")

print("Done creating dummy data.")
