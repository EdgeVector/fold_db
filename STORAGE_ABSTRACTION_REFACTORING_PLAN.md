# Storage Abstraction Refactoring Plan

## Executive Summary

The current storage abstraction layer provides value for basic CRUD operations but leaks backend-specific concerns (async/sync execution models, flush semantics, deadlock handling) throughout the codebase. This plan outlines a refactoring to make the abstraction truly backend-agnostic.

## Current Problems

### 1. Execution Model Leakage
**Problem**: Backend-specific execution models (async vs sync) leak into business logic.

**Evidence**:
- `is_dynamodb()` checks in 37+ locations
- `flush_sync()` method specifically for DynamoDB
- Conditional `spawn_blocking` avoidance for DynamoDB
- `run_async()` helper with deadlock workarounds

**Impact**: 
- Business logic must know about backend implementation details
- Deadlock issues when mixing sync/async contexts
- Difficult to add new backends without modifying business logic

### 2. Flush Semantics Mismatch
**Problem**: Flush means different things for different backends.

**Current State**:
- Sled: `flush()` = actual disk write, blocking operation
- DynamoDB: `flush()` = no-op (eventually consistent)
- Code must check backend type to decide flush behavior

**Impact**:
- `flush_sync()` workaround needed
- Confusion about when data is actually persisted
- Potential data loss if flush semantics misunderstood

### 3. Backend-Specific Code Paths
**Problem**: Business logic branches based on backend type.

**Examples**:
```rust
// mutation_manager.rs
if is_dynamodb {
    // Special handling
} else {
    // Different handling
}

// operation_processor.rs
if is_dynamodb {
    // Avoid spawn_blocking
} else {
    // Use spawn_blocking
}
```

**Impact**:
- Violates abstraction principle
- Makes testing harder
- Increases maintenance burden

## Proposed Solution: Async-First Abstraction

### Core Principle
**Make all storage operations async, even for sync backends.**

This eliminates the async/sync split and makes the abstraction truly backend-agnostic.

### Architecture Changes

#### 1. Unified Async Trait Interface

**Current** (Mixed):
```rust
// Some operations are async
async fn flush(&self) -> StorageResult<()>;

// But we need sync versions
fn flush_sync(&self) -> Result<(), SchemaError>;
```

**Proposed** (Unified):
```rust
#[async_trait]
pub trait KvStore: Send + Sync {
    // All operations are async
    async fn flush(&self) -> StorageResult<()>;
    
    // Backend capabilities exposed via metadata
    fn execution_model(&self) -> ExecutionModel;
    fn flush_behavior(&self) -> FlushBehavior;
}
```

#### 2. Execution Model Metadata

```rust
/// Describes how the backend executes operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    /// Backend is truly async (network I/O)
    Async,
    /// Backend is sync but wrapped in async (local I/O)
    SyncWrapped,
}

/// Describes flush behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushBehavior {
    /// Flush is a no-op (eventually consistent backend)
    NoOp,
    /// Flush performs actual persistence (strongly consistent)
    Persists,
}
```

#### 3. Backend Implementations

**Sled Backend** (Sync → Async Wrapper):
```rust
impl KvStore for SledKvStore {
    async fn flush(&self) -> StorageResult<()> {
        // Wrap sync operation in spawn_blocking
        tokio::task::spawn_blocking({
            let tree = self.tree.clone();
            move || tree.flush()
        })
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?
        .map_err(|e| StorageError::Io(e))
    }
    
    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::SyncWrapped
    }
    
    fn flush_behavior(&self) -> FlushBehavior {
        FlushBehavior::Persists
    }
}
```

**DynamoDB Backend** (Native Async):
```rust
impl KvStore for DynamoDbKvStore {
    async fn flush(&self) -> StorageResult<()> {
        // DynamoDB is eventually consistent, flush is no-op
        Ok(())
    }
    
    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Async
    }
    
    fn flush_behavior(&self) -> FlushBehavior {
        FlushBehavior::NoOp
    }
}
```

#### 4. Remove Backend-Specific Checks

**Before**:
```rust
if is_dynamodb {
    // Special path
} else {
    // Normal path
}
```

**After**:
```rust
// All paths use async, no branching needed
db_ops.flush().await?;
```

#### 5. Mutation Path Refactoring

**Current Problem**: Mutations use `spawn_blocking` which deadlocks with DynamoDB.

**Solution**: Make mutation operations fully async.

**Before**:
```rust
// operation_processor.rs
let mut ids = if is_dynamodb {
    // Direct call (no spawn_blocking)
    db_guard.mutation_manager.write_mutations_batch(vec![mutation])?
} else {
    // Use spawn_blocking
    tokio::task::spawn_blocking(move || {
        node.mutate_batch(vec![mutation])
    }).await?
};
```

**After**:
```rust
// operation_processor.rs
let mut ids = db_guard.mutation_manager
    .write_mutations_batch_async(vec![mutation])
    .await?;
```

**Mutation Manager Changes**:
```rust
impl MutationManager {
    // New async version
    pub async fn write_mutations_batch_async(
        &mut self,
        mutations: Vec<Mutation>
    ) -> Result<Vec<String>, SchemaError> {
        // All storage operations are async
        // No run_async() needed, no spawn_blocking
        // Direct async/await throughout
    }
    
    // Keep sync version for backward compatibility (deprecated)
    #[deprecated]
    pub fn write_mutations_batch(
        &mut self,
        mutations: Vec<Mutation>
    ) -> Result<Vec<String>, SchemaError> {
        // Wrapper that calls async version
        tokio::runtime::Handle::current()
            .block_on(self.write_mutations_batch_async(mutations))
    }
}
```

## Implementation Plan

### Phase 1: Add Execution Model Metadata (Week 1)

**Goal**: Expose backend capabilities without breaking changes.

**Tasks**:
1. Add `ExecutionModel` and `FlushBehavior` enums
2. Add methods to `KvStore` trait:
   - `fn execution_model(&self) -> ExecutionModel`
   - `fn flush_behavior(&self) -> FlushBehavior`
3. Implement for all backends (Sled, DynamoDB, InMemory)
4. Add tests to verify metadata is correct

**Files to Modify**:
- `src/storage/traits.rs`
- `src/storage/sled_backend.rs`
- `src/storage/dynamodb_backend.rs`
- `src/storage/inmemory_backend.rs`

**Success Criteria**:
- All backends expose correct metadata
- No breaking changes to existing code
- Tests pass

### Phase 2: Make Sled Backend Fully Async (Week 2)

**Goal**: Wrap all Sled operations in async, eliminating sync/async split.

**Tasks**:
1. Update `SledKvStore` to wrap all sync operations in `spawn_blocking`
2. Ensure all trait methods are async
3. Update error handling for async context
4. Add performance benchmarks to ensure no regression

**Files to Modify**:
- `src/storage/sled_backend.rs`
- `src/storage/tests.rs` (add async tests)

**Success Criteria**:
- All Sled operations are async
- No performance regression (< 5% overhead)
- All tests pass

### Phase 3: Create Async Mutation Manager (Week 3)

**Goal**: Make mutation operations fully async, eliminating deadlock issues.

**Tasks**:
1. Create `write_mutations_batch_async()` method
2. Refactor mutation logic to use direct async/await
3. Remove `run_async()` helper (no longer needed)
4. Update mutation paths to use async version
5. Keep sync version as deprecated wrapper

**Files to Modify**:
- `src/fold_db_core/mutation_manager.rs`
- `src/datafold_node/operation_processor.rs`
- `src/datafold_node/db.rs` (add async mutate methods)

**Success Criteria**:
- Mutations work without deadlocks
- No `is_dynamodb()` checks in mutation path
- All mutation tests pass

### Phase 4: Remove Backend-Specific Checks (Week 4)

**Goal**: Eliminate all `is_dynamodb()` and backend name checks.

**Tasks**:
1. Remove `is_dynamodb()` method from `DbOperationsV2`
2. Remove `flush_sync()` method (use async `flush()` everywhere)
3. Update all call sites to use async operations
4. Remove conditional execution paths
5. Update error messages to be backend-agnostic

**Files to Modify**:
- `src/db_operations/core_refactored.rs`
- `src/fold_db_core/mutation_manager.rs`
- `src/datafold_node/operation_processor.rs`
- `src/db_operations/schema_operations_v2.rs`
- All files with `is_dynamodb()` checks

**Success Criteria**:
- Zero `is_dynamodb()` checks in business logic
- Zero `backend_name == "dynamodb"` checks
- All tests pass
- Code is cleaner and more maintainable

### Phase 5: Documentation and Cleanup (Week 5)

**Goal**: Document the new architecture and remove deprecated code.

**Tasks**:
1. Update architecture documentation
2. Add migration guide for developers
3. Remove deprecated sync methods (after migration period)
4. Update examples and tutorials
5. Add performance benchmarks comparing old vs new

**Files to Modify**:
- `docs/STORAGE_ABSTRACTION_DESIGN.md`
- `docs/STORAGE_ABSTRACTION_IMPLEMENTATION.md`
- `README.md`
- Examples and tests

**Success Criteria**:
- Documentation is complete and accurate
- All deprecated code removed
- Examples work with new API

## Migration Strategy

### Backward Compatibility

**Approach**: Gradual migration with deprecation warnings.

1. **Phase 1-2**: Add new async methods alongside existing sync methods
2. **Phase 3**: Update call sites to use async methods
3. **Phase 4**: Mark sync methods as deprecated
4. **Phase 5**: Remove sync methods after migration period (e.g., 2 releases)

### Testing Strategy

1. **Unit Tests**: Test each backend independently
2. **Integration Tests**: Test with all backends (Sled, DynamoDB, InMemory)
3. **Performance Tests**: Ensure no regression with async wrappers
4. **Deadlock Tests**: Verify mutations don't deadlock with DynamoDB
5. **Compatibility Tests**: Ensure deprecated methods still work

## Benefits

### 1. True Abstraction
- Business logic doesn't need to know about backend implementation
- Easy to add new backends (PostgreSQL, Redis, etc.)
- Backend-specific code isolated to storage layer

### 2. Eliminates Deadlocks
- No more `spawn_blocking` + `block_on` deadlocks
- Consistent async execution model
- Easier to reason about concurrency

### 3. Better Performance
- No unnecessary thread pool overhead for async backends
- Consistent execution model allows better optimization
- Can use async batching more effectively

### 4. Improved Maintainability
- Less code (no conditional paths)
- Easier to test (consistent async model)
- Clearer error messages (backend-agnostic)

### 5. Future-Proof
- Easy to add new async backends
- Can leverage async ecosystem (tokio, async-std)
- Better support for distributed systems

## Risks and Mitigations

### Risk 1: Performance Regression for Sled
**Mitigation**: 
- Benchmark before/after
- Use `spawn_blocking` efficiently (batch operations)
- Consider thread-local caching for hot paths

### Risk 2: Breaking Changes
**Mitigation**:
- Gradual migration with deprecation
- Keep sync methods during transition
- Comprehensive test coverage

### Risk 3: Increased Complexity
**Mitigation**:
- Clear documentation
- Code examples
- Incremental rollout

## Success Metrics

1. **Code Quality**:
   - Zero `is_dynamodb()` checks in business logic
   - Zero `backend_name` checks in business logic
   - Reduced cyclomatic complexity

2. **Reliability**:
   - Zero deadlock issues
   - All tests pass
   - No performance regression

3. **Maintainability**:
   - Reduced lines of code
   - Clearer error messages
   - Better documentation

## Timeline

- **Week 1**: Execution model metadata
- **Week 2**: Sled async wrapper
- **Week 3**: Async mutation manager
- **Week 4**: Remove backend checks
- **Week 5**: Documentation and cleanup

**Total**: 5 weeks for complete refactoring

## Next Steps

1. Review and approve this plan
2. Create GitHub issues for each phase
3. Set up performance benchmarking infrastructure
4. Begin Phase 1 implementation
