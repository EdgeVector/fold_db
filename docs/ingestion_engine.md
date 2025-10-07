# Ingestion Engine Documentation

## Overview

The ingestion engine is an AI-powered system that automatically processes arbitrary JSON data, determines appropriate schemas, and generates database mutations. It supports both schema selection from existing schemas and automatic schema creation.

## Architecture

```mermaid
flowchart TB
    subgraph HTTP["HTTP Layer"]
        API["/api/ingestion/process"]
        Routes["routes.rs<br/>HTTP Handler"]
    end
    
    subgraph Service["Ingestion Service"]
        SIS["SimpleIngestionService"]
        Config["Configuration<br/>• Provider: OpenRouter/Ollama<br/>• API Keys<br/>• Timeouts<br/>• Auto-execute"]
    end
    
    subgraph AI["AI Services"]
        OpenRouter["OpenRouterService<br/>Claude via API"]
        Ollama["OllamaService<br/>Local Models"]
    end
    
    subgraph Components["Core Components"]
        Stripper["SchemaStripper<br/>Remove sensitive data"]
        Generator["MutationGenerator<br/>JSON → Mutations"]
        Processor["OperationProcessor<br/>Execute mutations"]
    end
    
    subgraph Storage["Storage Layer"]
        Node["DataFoldNode"]
        SchemaManager["SchemaManager"]
        DB["Sled Database"]
    end
    
    API --> Routes
    Routes --> SIS
    SIS --> Config
    SIS -.->|uses| OpenRouter
    SIS -.->|uses| Ollama
    SIS --> Stripper
    SIS --> Generator
    SIS --> Processor
    Processor --> Node
    Node --> SchemaManager
    SchemaManager --> DB
```

## Processing Flow

```mermaid
sequenceDiagram
    participant Client
    participant Routes
    participant Service as SimpleIngestionService
    participant AI as AI Service
    participant Schema as SchemaManager
    participant DB as Database
    
    Client->>Routes: POST /api/ingestion/process<br/>{data, auto_execute}
    Routes->>Service: process_json_with_node()
    
    rect rgb(200, 230, 255)
        Note over Service: Step 1: Validate Input
        Service->>Service: validate_input()<br/>Check not null, object/array
    end
    
    rect rgb(200, 255, 230)
        Note over Service,Schema: Step 2: Get Available Schemas
        Service->>Schema: get_schema_states()
        Schema-->>Service: All schemas
        Service->>Service: strip_schemas()<br/>Remove payment/permissions
    end
    
    rect rgb(255, 230, 200)
        Note over Service,AI: Step 3: AI Recommendation
        Service->>AI: get_schema_recommendation()<br/>{user_data, available_schemas}
        AI-->>Service: AISchemaResponse<br/>{existing_schemas, new_schemas, mappers}
    end
    
    rect rgb(230, 200, 255)
        Note over Service,DB: Step 4: Determine Schema
        alt Existing Schema Recommended
            Service->>Service: Use existing_schemas[0]
        else New Schema Needed
            Service->>Schema: load_schema_from_json()
            Schema->>DB: store_schema()
            Schema->>DB: store_schema_state(APPROVED)
            DB-->>Schema: Schema created
        end
    end
    
    rect rgb(255, 255, 200)
        Note over Service: Step 5: Generate Mutations
        Service->>Service: generate_mutations()<br/>Apply mappers, add metadata
    end
    
    rect rgb(200, 255, 255)
        Note over Service,DB: Step 6: Execute (if auto_execute)
        loop Each Mutation
            Service->>DB: execute_mutation()
            DB-->>Service: Result
        end
    end
    
    Service-->>Routes: IngestionResponse
    Routes-->>Client: JSON Response
```

## Detailed Step Flow

```mermaid
stateDiagram-v2
    [*] --> ValidateInput: Receive Request
    
    ValidateInput --> GetSchemas: Valid
    ValidateInput --> Error: Invalid
    
    GetSchemas --> StripSchemas: Schemas Retrieved
    StripSchemas --> AIRecommendation: Stripped
    
    AIRecommendation --> DetermineSchema: Response Received
    
    state DetermineSchema {
        [*] --> CheckExisting
        CheckExisting --> UseExisting: Has existing_schemas
        CheckExisting --> CreateNew: Has new_schemas
        
        CreateNew --> ParseJSON
        ParseJSON --> LoadToDB
        LoadToDB --> SetApproved
        SetApproved --> [*]
        
        UseExisting --> [*]
    }
    
    DetermineSchema --> GenerateMutations: Schema Ready
    
    state GenerateMutations {
        [*] --> CheckDataType
        CheckDataType --> ProcessArray: Is Array
        CheckDataType --> ProcessObject: Is Object
        
        ProcessArray --> IterateItems
        IterateItems --> ExtractFields
        ExtractFields --> ApplyMappers
        ApplyMappers --> CreateMutation
        CreateMutation --> IterateItems: More items
        CreateMutation --> CollectAll: Done
        
        ProcessObject --> ExtractFields
        
        CollectAll --> [*]
    }
    
    GenerateMutations --> CheckAutoExecute: Mutations Generated
    
    CheckAutoExecute --> ExecuteMutations: auto_execute=true
    CheckAutoExecute --> ReturnResponse: auto_execute=false
    
    state ExecuteMutations {
        [*] --> ForEachMutation
        ForEachMutation --> ConvertToOperation
        ConvertToOperation --> Execute
        Execute --> TrackResult: Success
        Execute --> LogError: Failure
        TrackResult --> ForEachMutation: More
        LogError --> ForEachMutation: Continue
        ForEachMutation --> [*]: Done
    }
    
    ExecuteMutations --> ReturnResponse
    ReturnResponse --> [*]
    Error --> [*]
```

## AI Schema Recommendation

```mermaid
flowchart LR
    subgraph Input
        UserData["User JSON Data"]
        AvailableSchemas["Available Schemas<br/>(stripped)"]
    end
    
    subgraph AIService["AI Service"]
        direction TB
        Provider{AI Provider}
        OpenRouter["OpenRouter API<br/>Claude 3.5 Sonnet"]
        Ollama["Ollama Local<br/>Llama3 etc."]
        
        Provider -->|openrouter| OpenRouter
        Provider -->|ollama| Ollama
    end
    
    subgraph Response["AI Response"]
        direction TB
        ExistingSchemas["existing_schemas: Vec&lt;String&gt;<br/>Match found in available"]
        NewSchema["new_schemas: Option&lt;Value&gt;<br/>Generated schema definition"]
        Mappers["mutation_mappers: HashMap<br/>Field name mappings"]
    end
    
    UserData --> AIService
    AvailableSchemas --> AIService
    
    OpenRouter --> Response
    Ollama --> Response
    
    Response --> Decision{Decision}
    Decision -->|Has existing| UseExisting["Use First Match"]
    Decision -->|No match| CreateNew["Create New Schema"]
```

## Schema Creation Process

```mermaid
flowchart TD
    Start([AI Recommends New Schema]) --> Parse[Parse JSON to<br/>DeclarativeSchemaDefinition]
    Parse --> Convert[Convert to<br/>Schema Object]
    Convert --> StoreDB[(Store in<br/>Sled Database)]
    StoreDB --> UpdateCache[Update In-Memory<br/>Schema Cache]
    UpdateCache --> SetAvailable[Set State:<br/>AVAILABLE]
    SetAvailable --> SetApproved[Override State:<br/>APPROVED]
    SetApproved --> Ready([Schema Ready for Use])
    
    style StoreDB fill:#e1f5fe
    style SetApproved fill:#fff9c4
    
    Note1[Note: Schema stored in DB only<br/>NOT written to available_schemas/ folder]
    SetApproved -.-> Note1
```

## Data Structures

```mermaid
classDiagram
    class IngestionRequest {
        +Value data
        +Option~bool~ auto_execute
        +Option~u32~ trust_distance
        +Option~String~ pub_key
    }
    
    class AISchemaResponse {
        +Vec~String~ existing_schemas
        +Option~Value~ new_schemas
        +HashMap mutation_mappers
    }
    
    class Mutation {
        +String schema_name
        +HashMap fields_and_values
        +HashMap key_value
        +MutationType mutation_type
        +u32 trust_distance
        +String pub_key
    }
    
    class IngestionResponse {
        +bool success
        +String schema_name
        +bool new_schema_created
        +usize mutations_generated
        +usize mutations_executed
        +Vec~String~ errors
    }
    
    class IngestionConfig {
        +AIProvider provider
        +OpenRouterConfig openrouter
        +OllamaConfig ollama
        +bool enabled
        +u32 max_retries
        +u64 timeout_seconds
        +bool auto_execute_mutations
        +u32 default_trust_distance
    }
    
    IngestionRequest --> AISchemaResponse : AI Processing
    AISchemaResponse --> Mutation : Generation
    Mutation --> IngestionResponse : Execution
    IngestionConfig ..> IngestionRequest : Configures
```

## Component Responsibilities

```mermaid
mindmap
    root((Ingestion Engine))
        HTTP Layer
            routes.rs
                Receive requests
                Create service
                Return responses
        Configuration
            Environment vars
            Config file
            Provider selection
            API keys & models
        AI Services
            OpenRouterService
                Claude API calls
                Structured prompts
                Response parsing
            OllamaService
                Local model calls
                Same interface
        Schema Management
            SchemaStripper
                Remove payments
                Remove permissions
                Simplify for AI
            Schema Creation
                Parse definitions
                Store in DB
                Set APPROVED state
        Mutation Generation
            MutationGenerator
                JSON to mutations
                Apply field mappers
                Add metadata
        Execution
            OperationProcessor
                Execute mutations
                Transaction handling
                Error recovery
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_PROVIDER` | `openrouter` | AI provider: `openrouter` or `ollama` |
| `FOLD_OPENROUTER_API_KEY` | - | OpenRouter API key (required for OpenRouter) |
| `OPENROUTER_MODEL` | `anthropic/claude-3.5-sonnet` | Model to use with OpenRouter |
| `OPENROUTER_BASE_URL` | `https://openrouter.ai/api/v1` | OpenRouter API endpoint |
| `OLLAMA_MODEL` | `llama3` | Ollama model name |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama service endpoint |
| `INGESTION_ENABLED` | `true` | Enable/disable ingestion |
| `INGESTION_AUTO_EXECUTE` | `true` | Auto-execute mutations |
| `INGESTION_DEFAULT_TRUST_DISTANCE` | `0` | Default trust distance for mutations |
| `INGESTION_MAX_RETRIES` | `3` | Max retries for AI API calls |
| `INGESTION_TIMEOUT_SECONDS` | `60` | Timeout for AI API calls |

### Config File

Location: `./config/ingestion_config.json`

```json
{
  "provider": "openrouter",
  "openrouter": {
    "api_key": "sk-...",
    "model": "anthropic/claude-3.5-sonnet",
    "base_url": "https://openrouter.ai/api/v1"
  },
  "ollama": {
    "model": "llama3",
    "base_url": "http://localhost:11434"
  }
}
```

## API Endpoints

### Process Ingestion

**POST** `/api/ingestion/process`

Request:
```json
{
  "data": {
    "title": "Example",
    "content": "Data to ingest"
  },
  "auto_execute": true,
  "trust_distance": 0,
  "pub_key": "optional-key"
}
```

Response:
```json
{
  "success": true,
  "schema_name": "ExampleSchema",
  "new_schema_created": false,
  "mutations_generated": 1,
  "mutations_executed": 1,
  "errors": []
}
```

### Get Status

**GET** `/api/ingestion/status`

Response:
```json
{
  "enabled": true,
  "configured": true,
  "provider": "OpenRouter",
  "model": "anthropic/claude-3.5-sonnet",
  "auto_execute_mutations": true,
  "default_trust_distance": 0
}
```

### Health Check

**GET** `/api/ingestion/health`

Response:
```json
{
  "status": "healthy",
  "service": "ingestion",
  "details": { ... }
}
```

### Get/Save Config

**GET** `/api/ingestion/config`  
**POST** `/api/ingestion/config`

### Validate JSON

**POST** `/api/ingestion/validate`

## Error Handling

```mermaid
flowchart TD
    Error{Error Type}
    
    Error -->|Input Validation| FailImmediate[Return 400<br/>Bad Request]
    Error -->|Configuration| FailConfig[Return 503<br/>Service Unavailable]
    Error -->|AI Service| FailAI[Return 500<br/>with error details]
    Error -->|Schema Creation| FailSchema[Return 500<br/>with error details]
    Error -->|Mutation Execution| ContinueNext[Log error<br/>Continue with next]
    
    ContinueNext --> FinalResponse[Return partial success<br/>with error list]
    
    style FailImmediate fill:#ffcdd2
    style FailConfig fill:#ffcdd2
    style FailAI fill:#ffcdd2
    style FailSchema fill:#ffcdd2
    style ContinueNext fill:#fff9c4
    style FinalResponse fill:#c8e6c9
```

### Error Recovery

- **Input validation errors**: Fail immediately, return error to client
- **AI service errors**: Retry with exponential backoff (configurable), then fail
- **Schema creation errors**: Fail the request, return detailed error
- **Mutation execution errors**: Log error, continue with remaining mutations
- **Partial success**: Return success=true with error list for failed mutations

## Implementation Notes

### Schema Storage

- Schemas created by ingestion are stored in the **Sled database**
- They are **NOT** written to the `available_schemas/` folder
- Schemas persist across restarts (database is durable)
- To export a schema to file, use the schema management APIs

### Schema State Lifecycle

1. AI creates new schema → Stored as `AVAILABLE`
2. Immediately changed to `APPROVED` (auto-approval for ingestion)
3. Schema is now usable for mutations
4. On restart: Schema loads from database with `APPROVED` state

### Mutation Generation

- Single objects → 1 mutation
- Arrays → 1 mutation per item
- Field mappers from AI applied to handle field name variations
- Trust distance and pub_key added for security/provenance

### AI Integration

The AI is given:
- User's JSON data
- Stripped versions of available schemas (no payment/permission info)

The AI returns:
- List of matching existing schemas (empty if no match)
- New schema definition (if no existing schema fits)
- Field name mappings (AI data field → schema field)

## Testing

Example test flow:

```bash
# 1. Configure ingestion
curl -X POST http://localhost:8080/api/ingestion/config \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "openrouter",
    "openrouter": {
      "api_key": "sk-...",
      "model": "anthropic/claude-3.5-sonnet"
    }
  }'

# 2. Process data
curl -X POST http://localhost:8080/api/ingestion/process \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "title": "My Blog Post",
      "body": "Content here",
      "tags": ["tech", "ai"]
    },
    "auto_execute": true
  }'

# 3. Check status
curl http://localhost:8080/api/ingestion/status
```

## Future Enhancements

- [ ] Export ingestion-created schemas to `available_schemas/` folder
- [ ] Batch ingestion for multiple JSON objects
- [ ] Schema versioning for AI-created schemas
- [ ] Custom AI prompts/templates
- [ ] Schema suggestion feedback loop
- [ ] Multi-schema mutations (for related data)
- [ ] Async ingestion with job queue
- [ ] Ingestion audit log

