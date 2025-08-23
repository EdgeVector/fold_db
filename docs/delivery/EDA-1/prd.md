# PBI-EDA-1: Event-Driven Mutation Completion Tracking

[View in Backlog](../backlog.md#user-content-EDA-1)

## Overview

This PBI addresses a critical race condition in the DataFold system where asynchronous mutation processing causes "Atom not found" errors during query operations. The system currently uses an event-driven architecture for mutations but lacks proper completion tracking, leading to queries attempting to access data before it's fully persisted.

## Problem Statement

**Current Issue:**
- Mutations are processed asynchronously through message bus events
- Queries are executed synchronously and immediately attempt to access data
- Race condition: Query tries to find atoms before mutation processing completes
- Result: "Atom 'xyz' not found" errors, even though system is working correctly

**Impact:**
- Confusing error messages in logs during normal operation
- Potential query failures under high concurrency
- Poor developer experience with misleading error output
- Difficulty distinguishing real errors from timing issues

## User Stories

**As a developer:**
- I want to eliminate "Atom not found" errors in query operations
- I need reliable mutation completion tracking
- I want clear async/await patterns for mutation-query sequences
- I need to distinguish between real errors and timing issues

**As a system:**
- I need proper event-driven mutation completion handling
- I want to maintain async architecture while providing sync interfaces
- I need to handle concurrent operations gracefully

## Technical Approach

### Core Solution

**1. MutationCompletionHandler**
- **Location**: New file `src/fold_db_core/mutation_completion_handler.rs`
- **Purpose**: Track pending mutations and signal completion
- **Key Features**: Event-driven completion, timeout support, thread-safe

**2. FoldDB Integration**
- **Location**: `src/fold_db_core/mod.rs`
- **Changes**: Add completion handler field, modify `write_schema()` to return mutation IDs, add `wait_for_mutation()` method

**3. Mutation Enhancement**
- **Location**: `src/schema/types/operations.rs`
- **Changes**: Add optional `synchronous` field for testing

**4. Test Updates**
- **Location**: `tests/comprehensive_filter_test.rs`
- **Changes**: Use completion tracking to eliminate race conditions

### Key Architecture Changes

```rust
// New completion handler
pub struct MutationCompletionHandler {
    pending_mutations: Arc<RwLock<HashMap<String, oneshot::Sender<()>>>>,
    message_bus: Arc<MessageBus>,
}

// Enhanced FoldDB API
impl FoldDB {
    pub fn write_schema(&mut self, mutation: Mutation) -> Result<String, SchemaError>
    pub async fn wait_for_mutation(&self, mutation_id: &str) -> Result<(), SchemaError>
}
```

## UX/UI Considerations

**Developer Experience:**
- Clean, error-free logs during normal operation
- Clear async/await patterns for mutation-query sequences
- Comprehensive error messages for actual failures
- Test utilities for synchronous operation mode

**System Behavior:**
- No functional changes to existing APIs
- Backward compatibility maintained
- Improved reliability under concurrent load
- Better error reporting and debugging

## Acceptance Criteria

### Core Functionality
- [ ] MutationCompletionHandler implemented with event-driven completion tracking
- [ ] FoldDB integrated with mutation completion handler
- [ ] `write_schema()` returns mutation IDs for tracking
- [ ] `wait_for_mutation()` method provides completion waiting
- [ ] Comprehensive filter tests pass without "Atom not found" errors
- [ ] System remains stable under concurrent load

### Technical Quality
- [ ] Thread-safe implementation using Arc<RwLock<>>
- [ ] Proper error handling with timeout support
- [ ] Efficient resource management and cleanup
- [ ] Backward compatibility maintained
- [ ] Clear error messages for debugging

### Performance & Integration
- [ ] Minimal overhead for mutation tracking
- [ ] Seamless integration with existing event system
- [ ] No impact on existing synchronous operations
- [ ] Proper logging and observability

## Dependencies

**Core Dependencies:**
- Existing `MessageBus` for event communication
- `MutationExecuted` event type (already implemented)
- `FoldDB` and `DbOperations` infrastructure
- Current mutation and query processing systems

**External Crates:**
- `tokio::sync::oneshot` for completion channels
- Standard library concurrency primitives (`Arc`, `RwLock`)

## Resolved Design Decisions

**1. Timeout Configuration**: System-wide default of 5 seconds
- Single `Duration::from_secs(5)` timeout for all mutations
- No per-operation configuration needed initially
- Covers 99% of mutation completion scenarios

**2. Error Handling**: Simple unified error approach
- All completion errors return `SchemaError::InvalidData("Mutation failed")`
- No complex error type differentiation needed
- Focus on solving "Atom not found" rather than detailed error analysis

**3. Cleanup Strategy**: Automatic cleanup only
- Mutations removed from tracking map on completion or timeout
- No manual cleanup API or periodic cleanup tasks required
- Memory usage is minimal and automatically managed

## Related Tasks

- Task EDA-1-1: Implement MutationCompletionHandler struct
- Task EDA-1-2: Integrate completion handler into FoldDB
- Task EDA-1-3: Add mutation ID tracking to write_schema
- Task EDA-1-4: Implement wait_for_mutation API
- Task EDA-1-5: Update comprehensive filter tests
- Task EDA-1-6: Add synchronous mutation mode support