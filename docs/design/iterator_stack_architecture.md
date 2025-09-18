# Iterator Stack Architecture

## Overview

The Iterator Stack is a sophisticated execution model for handling complex data transformations with nested iterations, fan-out operations, and multi-dimensional data processing. It provides a unified approach to executing declarative transforms across different schema types (Single, Range, HashRange) while maintaining proper data alignment and efficient execution.

## Core Concept

The Iterator Stack model handles **fan-out** using a stack of iterators (scopes). Each field expression is evaluated within this stacked scope, with the field containing the deepest active iterator determining the number of output rows.

### Key Principles

1. **Stack-Based Execution**: Nested iterations are managed as a stack of scopes
2. **Depth-Determined Output**: The deepest iterator determines the output cardinality
3. **Alignment Validation**: Fields must be properly aligned relative to the deepest iterator
4. **Broadcast Semantics**: Values are broadcast across iterations when appropriate
5. **Efficient Execution**: Deduplication and optimization prevent redundant computation

## Architecture Components

### 1. Chain Parser (`chain_parser/`)

**Purpose**: Parses complex expressions into executable chains

**Key Features**:
- Parses expressions like `blogpost.map().content.split_by_word().map()`
- Tracks iterator depths and branch structures
- Validates chain syntax and structure
- Creates `ParsedChain` objects with operation sequences

**Example**:
```rust
// Input: "blogpost.map().content.split_by_word().map()"
// Output: ParsedChain with operations:
//   - Schema iterator (blogpost.map())
//   - Field access (content)
//   - Word split iterator (split_by_word())
//   - Final mapping (map())
```

### 2. Iterator Stack (`stack.rs`)

**Purpose**: Manages the runtime stack of active iterators

**Key Features**:
- Maintains stack of `ActiveScope` objects
- Tracks current depth and scope contexts
- Manages iterator state and progression
- Handles scope creation and cleanup

**Data Structures**:
```rust
pub struct IteratorStack {
    pub scopes: Vec<ActiveScope>,           // Active iterator scopes
    pub current_depth: usize,               // Current stack depth
    pub max_depth: usize,                   // Maximum allowed depth
    pub scope_contexts: HashMap<usize, ScopeContext>, // Context per depth
}
```

### 3. Field Alignment (`field_alignment/`)

**Purpose**: Validates and manages field alignment rules

**Key Features**:
- Enforces 1:1, broadcast, and reduced alignment rules
- Validates that all fields align properly relative to the deepest iterator
- Optimizes alignment for performance
- Prevents misaligned field execution

**Alignment Types**:
- **1:1 Alignment**: Field matches the deepest iterator depth
- **Broadcast Alignment**: Field has shallower depth, values are broadcast
- **Reduced Alignment**: Field has deeper depth, values are aggregated

### 4. Execution Engine (`execution_engine/`)

**Purpose**: Runtime execution engine for iterator stack operations

**Key Features**:
- Coordinates execution of multiple field expressions
- Manages broadcasting and emission of index entries
- Handles deduplication and optimization
- Provides execution statistics and monitoring

**Core Components**:
- **Core Engine**: Main execution coordination
- **Field Execution**: Handles individual field processing
- **Field Evaluation**: Evaluates field expressions within scopes
- **Iterator Management**: Manages iterator lifecycle and state

### 5. Types and Errors (`types.rs`, `errors.rs`)

**Purpose**: Common data structures and error handling

**Key Types**:
- `IteratorType`: Schema, ArraySplit, WordSplit, Custom
- `ActiveScope`: Runtime scope information
- `ScopeContext`: Context data for each scope level
- `IteratorState`: Current iterator state and progress

## Data Flow

### 1. Parsing Phase
```
Expression String → ChainParser → ParsedChain
```

### 2. Validation Phase
```
ParsedChain → FieldAlignmentValidator → AlignmentValidationResult
```

### 3. Execution Phase
```
ParsedChain + InputData → ExecutionEngine → ExecutionResult
```

### 4. Result Aggregation
```
ExecutionResult → Aggregation → Final Output
```

## Execution Model

### Stack Depth Management

The iterator stack maintains a stack of active scopes, where each scope represents a level of iteration:

```
Depth 0: Root scope (input data)
Depth 1: blogpost.map() iterator
Depth 2: content.split_by_word() iterator
Depth 3: (deepest iterator determines output cardinality)
```

### Broadcasting Semantics

When a field has shallower depth than the deepest iterator, its values are broadcast across all iterations:

```
Deepest Iterator: 3 items
Field A (depth 1): 1 value → broadcast to 3 iterations
Field B (depth 3): 3 values → 1:1 alignment
```

### Index Entry Emission

Index entries are emitted at the correct depth, ensuring proper data structure:

```rust
pub struct IndexEntry {
    pub depth: usize,           // Emission depth
    pub hash_value: JsonValue,  // Hash field value
    pub range_value: JsonValue, // Range field value (if applicable)
    pub field_values: HashMap<String, JsonValue>, // Field values
}
```

## Performance Optimizations

### 1. Expression Deduplication
- Identical expressions are executed only once
- Results are shared across multiple fields
- Reduces redundant computation

### 2. Scope Caching
- Iterator states are cached when possible
- Context data is reused across iterations
- Memory usage is optimized

### 3. Lazy Evaluation
- Expressions are evaluated only when needed
- Iterators are advanced on-demand
- Early termination when possible

### 4. Memory Management
- Streaming iterators for large datasets
- Buffered iterators for moderate data
- In-memory iterators for small datasets

## Error Handling

### Error Types
- `IteratorStackError`: General iterator stack errors
- `FieldEvaluationError`: Field evaluation specific errors
- `AlignmentError`: Field alignment validation errors
- `ExecutionError`: Runtime execution errors

### Error Recovery
- Graceful degradation when possible
- Detailed error messages with context
- Partial results when some fields fail
- Comprehensive error logging

## Usage Patterns

### Simple Field Access
```rust
// Expression: "input.value"
// Result: Single value, no iteration
```

### Schema Iteration
```rust
// Expression: "blogpost.map()"
// Result: One row per blogpost
```

### Nested Iteration
```rust
// Expression: "blogpost.map().content.split_by_word().map()"
// Result: One row per word in each blogpost
```

### Mixed Alignment
```rust
// Field A: "blogpost.map()" (depth 1)
// Field B: "blogpost.map().content.split_by_word().map()" (depth 3)
// Result: Field A values broadcast to match Field B cardinality
```

## Integration with Transform System

The Iterator Stack integrates seamlessly with the broader transform system:

1. **Transform Executor** uses the Iterator Stack for complex field expressions
2. **Validation** ensures proper alignment before execution
3. **Aggregation** combines results from multiple iterator stack executions
4. **Coordination** manages multi-chain execution for HashRange schemas

## Monitoring and Statistics

The Iterator Stack provides comprehensive monitoring:

- **Execution Statistics**: Timing, memory usage, cache performance
- **Scope Information**: Depth, iterator types, completion status
- **Performance Metrics**: Items per depth, memory estimates
- **Warning System**: Non-fatal issues and optimization suggestions

## Future Enhancements

### Planned Improvements
1. **Parallel Execution**: Multi-threaded iterator processing
2. **Advanced Caching**: Intelligent result caching strategies
3. **Streaming Support**: Better support for large dataset processing
4. **Dynamic Optimization**: Runtime optimization based on data characteristics

### Extension Points
1. **Custom Iterators**: Plugin system for domain-specific iterators
2. **Alignment Rules**: Configurable alignment validation
3. **Execution Strategies**: Different execution modes for different use cases
4. **Monitoring Hooks**: Custom monitoring and profiling integration

## Conclusion

The Iterator Stack architecture provides a robust, efficient, and extensible foundation for complex data transformations. Its stack-based model elegantly handles nested iterations while maintaining proper data alignment and execution efficiency. The modular design allows for easy extension and optimization while providing comprehensive monitoring and error handling capabilities.
