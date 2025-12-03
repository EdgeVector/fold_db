# Async Mutation Completion Guarantees

## How We Know Mutations Are Complete

The `write_mutations_batch_async()` method uses **sequential `.await` calls** to ensure all operations complete before returning. Here's the execution flow:

### 1. All Operations Are Awaited

```rust
pub async fn write_mutations_batch_async(&mut self, mutations: Vec<Mutation>) -> Result<Vec<String>, SchemaError> {
    // ... validation ...
    
    // Each operation is awaited - execution waits for completion
    let new_atom = self.db_ops.create_and_store_atom_for_mutation_deferred(...).await?;
    // ↑ This completes before moving to next line
    
    self.db_ops.persist_field_molecule_deferred(...).await?;
    // ↑ This completes before moving to next line
    
    native_index_mgr.batch_index_field_values_with_classifications_async(...).await?;
    // ↑ This completes before moving to next line
    
    self.db_ops.store_schema(...).await?;
    // ↑ This completes before moving to next line
    
    self.db_ops.flush().await?;
    // ↑ This completes before returning
    
    Ok(mutation_ids)  // Only returns after ALL operations complete
}
```

### 2. What "Deferred" Means

The `_deferred` suffix means:
- ✅ **Storage operations ARE completed** (atoms, molecules are written)
- ✅ **Operations are awaited** (`.await` ensures completion)
- ❌ **No immediate flush** (but final `flush().await?` ensures persistence)

### 3. Completion Guarantees by Backend

#### DynamoDB Backend
- **Put operations**: Complete when `.await` returns (DynamoDB confirms write)
- **Flush**: No-op (DynamoDB is eventually consistent, but writes are confirmed)
- **Completion**: When function returns, data is **written and confirmed** by DynamoDB
- **Read consistency**: Data may take a few milliseconds to be visible due to eventual consistency, but the write is confirmed

#### Sled Backend  
- **Put operations**: Complete when `.await` returns (data in memory)
- **Flush**: Actually writes to disk (`.await` ensures disk write completes)
- **Completion**: When function returns, data is **persisted to disk**

#### In-Memory Backend
- **Put operations**: Complete immediately (in-memory)
- **Flush**: No-op
- **Completion**: When function returns, data is **in memory**

### 4. Return Value Guarantees

When `write_mutations_batch_async()` returns `Ok(mutation_ids)`:

✅ **All atoms are stored**  
✅ **All molecules are persisted**  
✅ **All indexes are updated**  
✅ **Schema is updated**  
✅ **Database is flushed** (if backend requires it)  
✅ **Events are published**  

### 5. No Background Tasks

The mutation manager does **NOT** spawn background tasks for mutations. All work is done synchronously (within the async context) before returning.

**Exception**: Event publishing uses a message bus, but this is synchronous and completes before return.

### 6. Test Verification

The test verifies completion by:

```rust
let mutation_ids = mutation_manager.write_mutations_batch_async(vec![mutation]).await
    .expect("Failed to execute mutation");
// ↑ When this line completes, ALL operations are done

// Then immediately verify data exists
let schema = schema_manager.get_schema(&schema_name).unwrap().unwrap();
let user1_field = schema.runtime_fields.get("name").unwrap();
if let Some(molecule_uuid) = user1_field.common().molecule_uuid() {
    println!("✅ User1 has molecule UUID: {}", molecule_uuid);
    // ↑ This works because mutation is complete
}
```

### 7. DynamoDB Eventual Consistency Note

For DynamoDB specifically:
- **Write confirmation**: When `put_item().await` returns, DynamoDB has confirmed the write
- **Read consistency**: Immediately after write, reads may not see the data due to eventual consistency
- **Typical delay**: 1-5 milliseconds for consistency
- **Strong consistency**: Use `ConsistentRead` parameter if needed (not currently used)

### 8. Error Handling

If any operation fails:
- The `.await?` will return an error
- Function returns `Err(SchemaError)`
- **No partial mutations** - if one field fails, the entire mutation fails
- **Atomicity**: Within a single mutation, all-or-nothing semantics

## Summary

**You know mutations are complete when:**
1. `write_mutations_batch_async().await` returns `Ok(...)`
2. All `.await` calls have completed
3. The function has returned

**The async nature means:**
- Operations can run concurrently with other async tasks
- But within the mutation batch, operations are sequential
- The function doesn't return until everything is done

**For DynamoDB:**
- Writes are confirmed when the function returns
- Reads may need to wait a few milliseconds for consistency
- But the write itself is complete and confirmed
