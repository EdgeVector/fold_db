# AI Query Examples with Declarative Schemas

This document demonstrates how the AI query feature intelligently selects the appropriate declarative schemas for various natural language queries.

## Overview

The LLM query system analyzes natural language queries and automatically:
1. Selects the most efficient schema (base or declarative)
2. Chooses appropriate filters (HashKey, RangeKey, etc.)
3. Selects relevant fields to return
4. Provides reasoning for the choices

## Successful Query Examples

### 1. Product Tag Search

**Natural Language Query:**
```
"Show me all products tagged with electronics"
```

**AI Analysis Result:**
- **Schema Selected**: `ProductTagIndex`
- **Filter**: `HashKey: "electronics"`
- **Reasoning**: "Using ProductTagIndex since it's optimized for querying products by tags with a hash key on 'tag'. This is more efficient than scanning the main Product table and filtering."

**Why This Works:**
The AI recognized that `ProductTagIndex` (our declarative schema) provides a hash-indexed lookup on product tags, making tag-based searches O(1) instead of O(n) table scan.

---

### 2. Author-Specific Blog Posts

**Natural Language Query:**
```
"Find all blog posts by Alice Smith"
```

**AI Analysis Result:**
- **Schema Selected**: `BlogPostAuthorIndex`
- **Filter**: `HashKey: "Alice Smith"`
- **Fields**: title, content, publish_date, tags
- **Reasoning**: "Using BlogPostAuthorIndex since we're querying by author name. This schema has author as the hash key, making it efficient for retrieving all posts by a specific author."

**Results**: Successfully returned multiple blog posts by Alice Smith with dates and tags

**Why This Works:**
The declarative `BlogPostAuthorIndex` schema indexes blog posts by author, enabling instant lookup of all posts by any author.

---

### 3. User Message History

**Natural Language Query:**
```
"Show me all messages from user_005"
```

**AI Analysis Result:**
- **Schema Selected**: `MessageSenderIndex`
- **Filter**: `HashKey: "user_005"`
- **Fields**: conversation_id, content, message_id, recipient_id, sent_at, message_type
- **Reasoning**: "MessageSenderIndex is the most efficient schema for this query since it has sender_id as the hash key. This allows direct lookup of all messages from a specific sender without scanning the entire message table."

**Why This Works:**
The `MessageSenderIndex` declarative schema creates a reverse index on sender_id, perfect for user message history queries.

---

### 4. Event Management by Organizer

**Natural Language Query:**
```
"What events are organized by user_001?"
```

**AI Analysis Result:**
- **Schema Selected**: `EventOrganizerIndex`
- **Filter**: `HashKey: "user_001"`
- **Fields**: event_id, title, description, start_time, end_time, location, category, status, max_capacity
- **Reasoning**: "EventOrganizerIndex is the most efficient schema for this query since it has organizer_id as the hash key. This allows direct lookup of all events for a specific organizer without scanning the entire events table."

**Why This Works:**
The `EventOrganizerIndex` enables organizers to quickly view their event portfolio without scanning all events.

---

### 5. Activity Type Analytics

**Natural Language Query:**
```
"Show me all login activities"
```

**AI Analysis Result:**
- **Schema Selected**: `UserActivityTypeIndex`
- **Filter**: `HashKey: "login"`
- **Fields**: user_id, timestamp, ip_address, metadata, activity_id
- **Reasoning**: "For login activities, UserActivityTypeIndex is most efficient since it has activity_type as hash key. Using HashKey filter on 'login' will retrieve all login records."

**Why This Works:**
The `UserActivityTypeIndex` schema groups activities by type, perfect for security monitoring and analytics queries.

---

### 6. Order Status Management

**Natural Language Query:**
```
"Show me all pending orders"
```

**AI Analysis Result:**
- **Schema Selected**: `OrderStatusIndex`
- **Filter**: `HashKey: "pending"`
- **Fields**: order_id, user_id, order_date, total_amount, payment_method, tracking_number
- **Reasoning**: "OrderStatusIndex is the most efficient schema for querying orders by status since it has status as the hash key. This allows direct lookup of all pending orders without scanning the entire orders table."

**Why This Works:**
The `OrderStatusIndex` enables warehouse and fulfillment teams to quickly view orders by status (pending, shipped, delivered, etc.).

---

### 7. Full-Text Message Search

**Natural Language Query:**
```
"Find all messages containing the word meeting"
```

**AI Analysis Result:**
- **Schema Selected**: `MessageWordIndex`
- **Filter**: `HashKey: "meeting"`
- **Fields**: message_id, conversation_id, sender_id, content, sent_at, message_type
- **Reasoning**: "MessageWordIndex is the optimal schema since it has a hash key on 'word' field which allows efficient lookup of messages containing specific words. Using MessageWordIndex is more efficient than scanning the full Message table with a Value filter."

**Why This Works:**
The `MessageWordIndex` provides full-text search capabilities by indexing individual words from message content using the `split_by_word()` transform function.

---

## Key Insights

### 1. Schema Selection Intelligence

The AI consistently chooses declarative schemas over base schemas when:
- A hash key matches the query predicate
- The declarative schema reduces search space
- Specific access patterns are needed (by author, by tag, by status, etc.)

### 2. Filter Strategy

The AI intelligently uses:
- **HashKey filters** for exact matches (author name, tag, status)
- **RangeKey filters** for temporal queries (future implementation)
- **No filter** when all records are needed

### 3. Field Selection

The AI selects relevant fields based on:
- Query intent (what information the user likely wants)
- Schema structure (available fields)
- Data relationships (foreign keys, metadata)

### 4. Performance Optimization

Each query demonstrates understanding of:
- O(1) hash lookups vs O(n) table scans
- Index utilization
- Data locality
- Query planning

---

## Testing AI Queries

### Using curl

```bash
# Step 1: Analyze query
curl -X POST http://localhost:9001/api/llm-query/analyze \
  -H "Content-Type: application/json" \
  -d '{"query": "YOUR NATURAL LANGUAGE QUERY"}'

# Step 2: Execute the query plan (use session_id and query_plan from Step 1)
curl -X POST http://localhost:9001/api/llm-query/execute \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "SESSION_ID_FROM_STEP_1",
    "query_plan": QUERY_PLAN_FROM_STEP_1
  }'

# Step 3: Ask follow-up questions (optional)
curl -X POST http://localhost:9001/api/llm-query/chat \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "SESSION_ID",
    "question": "YOUR FOLLOW-UP QUESTION"
  }'
```

### Using the UI

1. Navigate to http://localhost:9001
2. Go to the LLM Query tab
3. Enter your natural language query
4. Review the AI's schema selection and reasoning
5. Execute the query
6. Ask follow-up questions about the results

---

## Declarative Schemas That Enable These Queries

All these intelligent queries are possible because of our 17 declarative schemas:

1. **ProductTagIndex** - Products by tag
2. **ProductCategoryIndex** - Products by category
3. **ProductBrandIndex** - Products by brand
4. **ProductReviewStats** - Review aggregations
5. **ProductReviewUserIndex** - Reviews by user
6. **UserOrderStats** - Orders by user
7. **OrderStatusIndex** - Orders by status
8. **EventCategoryIndex** - Events by category
9. **EventOrganizerIndex** - Events by organizer
10. **MessageWordIndex** - Messages by word (full-text)
11. **MessageSenderIndex** - Messages by sender
12. **ConversationMessageStats** - Conversation statistics
13. **BlogPostWordIndex** - Blog posts by word (full-text)
14. **BlogPostTagIndex** - Blog posts by tag
15. **BlogPostAuthorIndex** - Blog posts by author
16. **UserByStatus** - Users by status
17. **UserActivityTypeIndex** - Activities by type

---

## Conclusion

The AI query feature demonstrates remarkable intelligence in:
- Understanding natural language intent
- Selecting optimal data structures
- Explaining its reasoning
- Balancing performance and functionality

Our declarative schemas provide the foundation for this intelligence, creating specialized indexes that transform complex queries into simple, efficient lookups.

---

## Next Steps

Try these advanced queries:
- "Show me products in the Electronics category priced under $100"
- "What are the most recent blog posts about Rust?"
- "Find all shipped orders from the last week"
- "Show me conversation statistics for CONV-001"
- "List all Workshop events in September"

