# Iterator Stack Quick Reference

## Quick Start

### Basic Usage
```rust
use crate::transform::iterator_stack::{ChainParser, ExecutionEngine, IteratorStack};

// 1. Parse expression
let parser = ChainParser::new();
let chain = parser.parse("blogpost.map().content.split_by_word().map()")?;

// 2. Build iterator stack
let stack = IteratorStack::from_chain(&chain)?;

// 3. Execute
let mut engine = ExecutionEngine::new();
let result = engine.execute_fields(&[chain], &alignment_result, input_data)?;
```

### Common Patterns

#### Simple Field Access
```rust
// Expression: "input.value"
// No iteration, direct value access
```

#### Schema Iteration
```rust
// Expression: "blogpost.map()"
// One row per blogpost
```

#### Nested Iteration
```rust
// Expression: "blogpost.map().content.split_by_word().map()"
// One row per word in each blogpost
```

#### Array Processing
```rust
// Expression: "tags.split_array().map()"
// One row per tag
```

## Key Types

### Core Types
```rust
pub struct IteratorStack {
    pub scopes: Vec<ActiveScope>,           // Active iterator scopes
    pub current_depth: usize,               // Current stack depth
    pub max_depth: usize,                   // Maximum allowed depth
    pub scope_contexts: HashMap<usize, ScopeContext>, // Context per depth
}

pub struct ActiveScope {
    pub depth: usize,                       // Depth level
    pub iterator_type: IteratorType,        // Type of iterator
    pub position: usize,                    // Current position
    pub total_items: usize,                 // Total items to iterate
    pub branch_path: String,                // Branch path
    pub parent_depth: Option<usize>,        // Parent scope depth
}

pub enum IteratorType {
    Schema { field_name: String },          // Schema-level iterator
    ArraySplit { field_name: String },     // Array split iterator
    WordSplit { field_name: String },      // Word split iterator
    Custom { name: String, config: IteratorConfig }, // Custom iterator
}
```

### Execution Types
```rust
pub struct ExecutionResult {
    pub index_entries: Vec<IndexEntry>,     // Generated index entries
    pub warnings: Vec<ExecutionWarning>,    // Execution warnings
    pub statistics: ExecutionStatistics,    // Performance statistics
}

pub struct IndexEntry {
    pub depth: usize,                       // Emission depth
    pub hash_value: JsonValue,              // Hash field value
    pub range_value: JsonValue,             // Range field value
    pub field_values: HashMap<String, JsonValue>, // Field values
}
```

## Error Handling

### Common Errors
```rust
pub enum IteratorStackError {
    ParseError(String),                     // Expression parsing failed
    AlignmentError(String),                 // Field alignment violation
    ExecutionError(String),                 // Runtime execution failure
    ValidationError(String),                // Data validation failure
}
```

### Error Recovery
```rust
match result {
    Ok(execution_result) => {
        // Process successful result
        process_results(execution_result);
    }
    Err(IteratorStackError::ParseError(msg)) => {
        // Handle parsing error
        log::error!("Parse error: {}", msg);
        return Err(SchemaError::InvalidField(msg));
    }
    Err(IteratorStackError::AlignmentError(msg)) => {
        // Handle alignment error
        log::warn!("Alignment warning: {}", msg);
        // Continue with partial results if possible
    }
    Err(e) => {
        // Handle other errors
        return Err(SchemaError::InvalidTransform(format!("Execution failed: {}", e)));
    }
}
```

## Performance Tips

### Optimization Strategies
1. **Use Expression Deduplication**: Identical expressions are executed only once
2. **Leverage Scope Caching**: Iterator states are cached when possible
3. **Choose Appropriate Iterator Types**: 
   - `Streaming` for large datasets
   - `Buffered` for moderate data
   - `InMemory` for small datasets
4. **Monitor Statistics**: Use `ExecutionStatistics` for performance insights

### Memory Management
```rust
// For large datasets
let stack = IteratorStack::with_max_depth(5); // Limit depth

// Monitor memory usage
let stats = execution_result.statistics;
log::info!("Memory usage: {} bytes", stats.memory_usage_bytes);
```

## Debugging

### Enable Debug Logging
```rust
// Set log level to debug
env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

### Common Debug Scenarios
1. **Parse Errors**: Check expression syntax
2. **Alignment Errors**: Verify field depths match
3. **Execution Errors**: Check input data structure
4. **Performance Issues**: Monitor statistics and warnings

### Debug Information
```rust
// Log stack state
log::debug!("Stack depth: {}, Scopes: {}", stack.current_depth, stack.len());

// Log execution statistics
log::debug!("Generated {} entries in {:?}", 
    result.statistics.total_entries, 
    result.statistics.execution_duration);
```

## Integration Examples

### With Transform Executor
```rust
use crate::transform::executor::TransformExecutor;

// Transform executor uses iterator stack internally
let result = TransformExecutor::execute_transform(&transform, input_values)?;
```

### With Field Alignment
```rust
use crate::transform::iterator_stack::field_alignment::FieldAlignmentValidator;

// Validate alignment before execution
let validator = FieldAlignmentValidator::new();
let alignment_result = validator.validate_alignment(&parsed_chains)?;
```

### With Aggregation
```rust
use crate::transform::aggregation::aggregate_results_unified;

// Aggregate results from iterator stack execution
let final_result = aggregate_results_unified(
    &schema,
    &parsed_chains,
    &execution_result,
    &input_values,
    &all_expressions,
)?;
```

#### Universal key workflow
- `aggregate_results_unified` expects a `DeclarativeSchemaDefinition` whose
  [`KeyConfig`](../schema-management.md#key-configuration) describes the
  schema's universal key fields.
- The aggregator hydrates key metadata via
  `schema_operations::shape_unified_result`, which always returns a
  `{ "hash": <value>, "range": <value>, "fields": { ... } }` envelope.
- HashRange schemas also receive compatibility arrays for `hash_key` and
  `range_key` so existing consumers keep working while migrating to the
  normalized output.

```rust
let shaped = aggregate_results_unified(
    &schema,
    &parsed_chains,
    &execution_result,
    &input_values,
    &all_expressions,
)?;

assert_eq!(shaped["fields"]["value"], json!(42));
assert_eq!(shaped["hash"], json!("hash-1"));
```

- Legacy range-only schemas continue to surface a `range_key` property alongside
  the universal `range` field to preserve response compatibility.
- Universal key adoption policy is documented in
  [SCHEMA-KEY-004](../project_logic.md#logic-table) and cross-referenced by the
  [SKC-7 PBI delivery notes](../delivery/SKC-7/prd.md#notes).

#### Troubleshooting universal key aggregation
- **Missing range field**: HashRange schemas must configure both
  `key.hash_field` and `key.range_field`. The aggregator surfaces the underlying
  `SchemaError` message from `shape_unified_result`, e.g. `HashRange schema
  requires key.hash_field and key.range_field`.
- **Mismatched dotted paths**: Ensure iterator expressions populate the same
  dotted field paths defined in the schema's `KeyConfig`. When execution output
  omits a dotted segment the aggregator falls back to
  `resolve_dotted_path`, which returns `null` instead of failing hard.
- **Unexpected null hash/range**: Check that universal key tests in
  [SKC-7-2](../delivery/SKC-7/SKC-7-2.md) cover the scenario and that the
  iterator stack emits `_hash_field`/`_range_field` chains for the schema.

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expression() {
        let parser = ChainParser::new();
        let chain = parser.parse("input.value").unwrap();
        assert_eq!(chain.depth, 0);
    }

    #[test]
    fn test_nested_expression() {
        let parser = ChainParser::new();
        let chain = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        assert!(chain.depth > 0);
    }
}
```

### Integration Tests
```rust
#[test]
fn test_execution_flow() {
    let parser = ChainParser::new();
    let chain = parser.parse("blogpost.map()").unwrap();
    
    let mut engine = ExecutionEngine::new();
    let result = engine.execute_fields(&[chain], &alignment_result, input_data).unwrap();
    
    assert!(!result.index_entries.is_empty());
}
```

## Common Pitfalls

1. **Incorrect Expression Syntax**: Use proper chain syntax
2. **Misaligned Fields**: Ensure field depths are consistent
3. **Memory Issues**: Monitor memory usage for large datasets
4. **Performance Bottlenecks**: Use appropriate iterator types
5. **Error Handling**: Always handle errors gracefully

## Resources

- [Full Architecture Documentation](./iterator_stack_architecture.md)
- [Execution Flow Diagram](./iterator_stack_flow_diagram.md)
- [Transform System Documentation](../transform.md)
- [API Reference](../../api-reference.md)
