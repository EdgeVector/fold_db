# LLM Query Workflow - Implementation Summary

## Overview
Complete implementation of the natural language query workflow using LLM to analyze queries, create indexes, and provide interactive results exploration.

## Backend Implementation

### 1. Module Structure (`src/datafold_node/llm_query/`)
- **mod.rs** - Module exports
- **types.rs** - Request/Response types, session context
- **session.rs** - Session management with TTL
- **service.rs** - LLM service integration
- **routes.rs** - HTTP route handlers

### 2. API Endpoints

#### POST `/api/llm-query/analyze`
- Analyzes natural language queries using LLM
- Returns query plan with optional index schema
- Creates/reuses session for context

**Request:**
```json
{
  "query": "Find all blog posts about AI from last month",
  "session_id": "optional-uuid"
}
```

**Response:**
```json
{
  "session_id": "uuid",
  "query_plan": {
    "query": { "schema_name": "...", "fields": [...] },
    "index_schema": null | { ... },
    "reasoning": "Analysis explanation"
  }
}
```

#### POST `/api/llm-query/execute`
- Executes query plan
- Creates index if needed and monitors backfill
- Returns results with LLM summary

**Request:**
```json
{
  "session_id": "uuid",
  "query_plan": { ... }
}
```

**Response:**
```json
{
  "status": "pending" | "running" | "complete",
  "backfill_progress": 0.75,
  "results": [...],
  "summary": "LLM-generated summary"
}
```

#### POST `/api/llm-query/chat`
- Ask follow-up questions about results
- Uses conversation history and cached results

**Request:**
```json
{
  "session_id": "uuid",
  "question": "What's the average word count?"
}
```

**Response:**
```json
{
  "answer": "Based on the results...",
  "context_used": true
}
```

#### GET `/api/llm-query/backfill/{hash}`
- Get backfill status and progress

**Response:**
```json
{
  "status": "in_progress",
  "progress": 0.65,
  "total_records": 1000,
  "processed_records": 650
}
```

### 3. Key Features
- ✅ Session management with 1-hour TTL
- ✅ LLM integration (OpenRouter/Ollama)
- ✅ Smart indexing recommendations (>10k threshold)
- ✅ Backfill progress monitoring
- ✅ Result formatting (hash→range→fields)
- ✅ Conversation history for interactive Q&A
- ✅ No silent failures - all errors thrown

### 4. Integration Points
- Uses existing ingestion LLM services
- Leverages schema loading and approval flow
- Monitors backfill via BackfillTracker
- Integrated with http_server.rs routes

## Frontend Implementation

### 1. Components (`src/datafold_node/static-react/src/`)

#### LlmQueryTab Component
- **Location:** `components/tabs/LlmQueryTab.jsx`
- **Features:**
  - Natural language query input
  - Real-time query analysis display
  - Query plan visualization
  - Backfill progress monitoring
  - Results summary display
  - Interactive follow-up Q&A
  - Chat history with context

#### LLM Query API Client
- **Location:** `api/clients/llmQueryClient.ts`
- **Exports:**
  - `analyzeQuery()`
  - `executeQueryPlan()`
  - `chat()`
  - `getBackfillStatus()`

### 2. UI Features

#### Query Input
- Large textarea for natural language queries
- Example placeholder text
- "Analyze Query" button with loading state
- "New Query" button to reset session

#### Query Plan Display
- Shows LLM reasoning/analysis
- Displays target schema
- Lists fields to retrieve
- Highlights index creation if needed
- "Execute Query" button

#### Backfill Progress
- Visual progress bar
- Percentage display
- Real-time polling updates

#### Results Summary
- LLM-generated summary of results
- Clean prose formatting

#### Follow-up Questions
- Chat-style interface
- Conversation history display
- Real-time question answering
- Context-aware responses

### 3. Tab Navigation
- New "AI Query" tab with 🤖 icon
- Positioned after "Query" tab
- Integrated with existing tab system

## Testing

### Backend Tests
```bash
cargo test --lib llm_query
# ✅ 3 tests passed (session management)
```

### Build Verification
```bash
cargo build --release
# ✅ Success

cargo clippy
# ✅ No warnings

cd src/datafold_node/static-react && npm run build
# ✅ Success (773 modules)
```

## Usage Example

### 1. Start the server
```bash
./run_http_server.sh
```

### 2. Navigate to the UI
Open `http://localhost:9001` and click the "AI Query" tab

### 3. Example Query Flow
1. **Enter query:** "Find all blog posts from last month with more than 100 views"
2. **Analyze:** Click "Analyze Query" - LLM determines optimal query plan
3. **Review Plan:** See which schema, fields, and filters will be used
4. **Execute:** Click "Execute Query" - Index created if needed, results returned
5. **View Summary:** Read LLM-generated insights about the results
6. **Ask Follow-ups:** "What's the most common topic?" - Get contextual answers

## Architecture Benefits

1. **DRY Principles:**
   - Reuses existing LLM services
   - Leverages schema and backfill infrastructure
   - Shared session management pattern

2. **No Silent Failures:**
   - All errors properly thrown and handled
   - User feedback for all operations

3. **Type Safety:**
   - TypeScript for frontend
   - Rust type system for backend
   - Full API type definitions

4. **Performance:**
   - In-memory session caching
   - Background backfill polling
   - Optimistic UI updates

5. **Extensibility:**
   - Easy to add new LLM providers
   - Session storage can be persisted
   - Additional analysis features simple to add

## Configuration

### Backend
Set environment variables for LLM provider:
```bash
export AI_PROVIDER=openrouter  # or ollama
export FOLD_OPENROUTER_API_KEY=your_key
export OPENROUTER_MODEL=anthropic/claude-3.5-sonnet
```

### Frontend
No configuration needed - uses API constants from `constants/api.ts`

## Files Created/Modified

### Backend (New Files)
- `src/datafold_node/llm_query/mod.rs`
- `src/datafold_node/llm_query/types.rs`
- `src/datafold_node/llm_query/session.rs`
- `src/datafold_node/llm_query/service.rs`
- `src/datafold_node/llm_query/routes.rs`

### Backend (Modified)
- `src/datafold_node/mod.rs` - Export llm_query module
- `src/datafold_node/http_server.rs` - Add LLM query routes
- `src/ingestion/openrouter_service.rs` - Make call_openrouter_api public
- `src/ingestion/ollama_service.rs` - Make call_ollama_api public

### Frontend (New Files)
- `src/datafold_node/static-react/src/api/clients/llmQueryClient.ts`
- `src/datafold_node/static-react/src/components/tabs/LlmQueryTab.jsx`

### Frontend (Modified)
- `src/datafold_node/static-react/src/App.jsx` - Add LlmQueryTab
- `src/datafold_node/static-react/src/constants/ui.js` - Add AI Query tab
- `src/datafold_node/static-react/src/api/clients/index.ts` - Export llmQueryClient

## Next Steps (Optional Enhancements)

1. **Streaming Results:** WebSocket for real-time backfill updates
2. **Query Caching:** Cache common query patterns
3. **Multi-step Queries:** Chain multiple queries together
4. **Visualization:** Generate charts/graphs from results
5. **Query History:** Persist and reuse past successful queries
6. **Session Persistence:** Store sessions in database for recovery
7. **Rate Limiting:** Prevent LLM API abuse
8. **Cost Tracking:** Monitor LLM API usage and costs

## Documentation
Full workflow documentation available in: `docs/llm_query_workflow.md`

