# Iterator Stack Execution Flow

## High-Level Flow Diagram

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Expression    │───▶│   Chain Parser   │───▶│   Parsed Chain  │
│   String        │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Final Output   │◀───│   Aggregation    │◀───│ Execution Result│
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         ▲
                                                         │
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Alignment Result│───▶│ Execution Engine │◀───│  Iterator Stack │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         ▲                       │                       ▲
         │                       ▼                       │
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Field Alignment │    │   Input Data     │    │   Scope Context │
│   Validator     │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Detailed Execution Flow

### Phase 1: Parsing and Validation
```
Expression: "blogpost.map().content.split_by_word().map()"
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Chain Parser                                               │
│ ├─ Parse schema reference (blogpost)                       │
│ ├─ Parse map operation                                     │
│ ├─ Parse field access (content)                            │
│ ├─ Parse split operation (split_by_word)                   │
│ └─ Parse final map operation                               │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ ParsedChain                                                │
│ ├─ operations: [Schema, Map, Field, Split, Map]           │
│ ├─ depth: 3                                               │
│ ├─ expression: "blogpost.map().content.split_by_word()..." │
│ └─ branch_path: "root"                                    │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Field Alignment Validator                                  │
│ ├─ Validate field alignment rules                          │
│ ├─ Check depth consistency                                 │
│ ├─ Verify broadcast semantics                              │
│ └─ Generate alignment result                               │
└─────────────────────────────────────────────────────────────┘
```

### Phase 2: Iterator Stack Construction
```
┌─────────────────────────────────────────────────────────────┐
│ Iterator Stack Construction                                │
│                                                             │
│ Depth 0: Root Scope                                        │
│ ├─ context: input_data                                     │
│ ├─ iterator_type: None                                     │
│ └─ position: 0                                             │
│                                                             │
│ Depth 1: Schema Iterator (blogpost.map())                  │
│ ├─ context: blogpost_data                                  │
│ ├─ iterator_type: Schema { field_name: "blogpost" }       │
│ ├─ position: 0                                             │
│ └─ total_items: 3                                          │
│                                                             │
│ Depth 2: Field Access (content)                            │
│ ├─ context: content_data                                   │
│ ├─ iterator_type: None                                     │
│ └─ position: 0                                             │
│                                                             │
│ Depth 3: Word Split Iterator (split_by_word())             │
│ ├─ context: word_data                                      │
│ ├─ iterator_type: WordSplit { field_name: "content" }     │
│ ├─ position: 0                                             │
│ └─ total_items: 15 (words across all blogposts)           │
└─────────────────────────────────────────────────────────────┘
```

### Phase 3: Execution Engine Processing
```
┌─────────────────────────────────────────────────────────────┐
│ Execution Engine                                           │
│                                                             │
│ 1. Create Execution Context                                │
│    ├─ input_data: original input                           │
│    ├─ field_alignments: alignment rules                    │
│    ├─ emission_depth: 3 (deepest iterator)                 │
│    └─ variables: {}                                        │
│                                                             │
│ 2. Execute Field Expressions                               │
│    ├─ Process each field at correct depth                  │
│    ├─ Apply broadcasting rules                             │
│    ├─ Generate index entries                               │
│    └─ Collect execution statistics                         │
│                                                             │
│ 3. Generate Execution Result                               │
│    ├─ index_entries: Vec<IndexEntry>                      │
│    ├─ warnings: Vec<ExecutionWarning>                      │
│    └─ statistics: ExecutionStatistics                      │
└─────────────────────────────────────────────────────────────┘
```

### Phase 4: Result Aggregation
```
┌─────────────────────────────────────────────────────────────┐
│ Result Aggregation                                         │
│                                                             │
│ Input: ExecutionResult                                     │
│ ├─ index_entries: 15 entries (one per word)               │
│ ├─ warnings: []                                            │
│ └─ statistics: {...}                                       │
│                                                             │
│ Process:                                                   │
│ ├─ Group entries by depth                                  │
│ ├─ Apply aggregation rules                                 │
│ ├─ Format output structure                                 │
│ └─ Handle error cases                                      │
│                                                             │
│ Output: Final Result Object                                │
│ ├─ field_values: HashMap<String, JsonValue>               │
│ └─ metadata: execution_info                                │
└─────────────────────────────────────────────────────────────┘
```

## Broadcasting Example

### Scenario: Mixed Field Depths
```
Field A: "blogpost.map()"           (depth 1, 3 items)
Field B: "blogpost.map().content.split_by_word().map()"  (depth 3, 15 items)

Execution:
├─ Deepest iterator: depth 3 (15 items)
├─ Field A: broadcast 3 values → 15 values
│  ├─ Value 1 → broadcast to iterations 1-5
│  ├─ Value 2 → broadcast to iterations 6-10
│  └─ Value 3 → broadcast to iterations 11-15
└─ Field B: direct 1:1 alignment → 15 values
   ├─ Value 1 → iteration 1
   ├─ Value 2 → iteration 2
   └─ ... → iterations 3-15
```

## Error Handling Flow

```
┌─────────────────┐
│   Error Occurs  │
└─────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ Error Classification                                       │
│ ├─ ParseError: Invalid expression syntax                   │
│ ├─ AlignmentError: Field alignment violation               │
│ ├─ ExecutionError: Runtime execution failure               │
│ └─ ValidationError: Data validation failure                │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ Error Recovery Strategy                                    │
│ ├─ Graceful Degradation: Continue with partial results     │
│ ├─ Error Propagation: Stop execution and report error      │
│ ├─ Retry Logic: Attempt alternative execution path         │
│ └─ Fallback Values: Use default values for failed fields   │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ Error Reporting                                            │
│ ├─ Detailed error messages with context                    │
│ ├─ Execution statistics and diagnostics                    │
│ ├─ Suggested remediation steps                             │
│ └─ Partial results when possible                           │
└─────────────────────────────────────────────────────────────┘
```

## Performance Monitoring Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Performance Monitoring                                     │
│                                                             │
│ Execution Statistics:                                      │
│ ├─ total_entries: Number of index entries generated       │
│ ├─ items_per_depth: Distribution of items across depths   │
│ ├─ memory_usage_bytes: Estimated memory consumption       │
│ ├─ cache_hits: Number of cache hits                       │
│ └─ cache_misses: Number of cache misses                   │
│                                                             │
│ Scope Information:                                         │
│ ├─ total_scopes: Number of active scopes                  │
│ ├─ current_depth: Current stack depth                     │
│ ├─ active_iterators: Types of active iterators            │
│ └─ completion_status: Completion status per depth         │
│                                                             │
│ Warnings:                                                  │
│ ├─ Performance warnings (slow operations)                 │
│ ├─ Memory warnings (high memory usage)                    │
│ ├─ Alignment warnings (potential alignment issues)        │
│ └─ Optimization suggestions                               │
└─────────────────────────────────────────────────────────────┘
```
