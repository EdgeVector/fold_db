# Storage Abstraction Quick Start Guide

## TL;DR

fold_db now supports multiple storage backends through a trait-based abstraction layer:
- 🗄️ **Sled** - Local embedded database (default)
- ☁️ **DynamoDB** - AWS cloud storage with multi-tenancy
- 🧪 **In-Memory** - Fast testing without persistence

## Quick Examples

### Local Development (Sled)

```rust
use datafold::db_operations::DbOperationsV2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = sled::open("data")?;
    let db_ops = DbOperationsV2::from_sled(db).await?;
    
    // Store and retrieve data
    db_ops.store_item("user:123", &user).await?;
    let user = db_ops.get_item::<User>("user:123").await?;
    
    Ok(())
}
```

### Production (DynamoDB)

```rust
use datafold::db_operations::DbOperationsV2;
use aws_config::BehaviorVersion;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    
    let db_ops = DbOperationsV2::from_dynamodb(
        client,
        "MyTable".to_string(),
        Some("user_id".to_string())  // Multi-tenant
    ).await?;
    
    // Same API as Sled!
    db_ops.store_item("user:123", &user).await?;
    
    Ok(())
}
```

### Testing (In-Memory)

```rust
use datafold::storage::InMemoryNamespacedStore;
use datafold::db_operations::DbOperationsV2;
use std::sync::Arc;

#[tokio::test]
async fn test_feature() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let db_ops = DbOperationsV2::from_namespaced_store(store).await.unwrap();
    
    // Test without touching disk
    db_ops.store_item("test", &data).await.unwrap();
    assert!(db_ops.get_item::<Data>("test").await.unwrap().is_some());
}
```

## Common Operations

### Store Item
```rust
db_ops.store_item("key", &item).await?;
```

### Get Item
```rust
let item: Option<MyType> = db_ops.get_item("key").await?;
```

### Delete Item
```rust
let existed = db_ops.delete_item("key").await?;
```

### List Keys with Prefix
```rust
let keys = db_ops.list_items_with_prefix("user:").await?;
```

### Namespace Operations
```rust
// Store in namespace
db_ops.store_in_namespace("schemas", "schema1", &schema).await?;

// Get from namespace
let schema = db_ops.get_from_namespace::<Schema>("schemas", "schema1").await?;

// List namespace keys
let keys = db_ops.list_keys_in_namespace("schemas").await?;
```

### Batch Operations
```rust
let items = vec![
    ("key1".to_string(), item1),
    ("key2".to_string(), item2),
];
db_ops.batch_store_items(&items).await?;
```

## Switching Backends

No code changes needed! Just swap the constructor:

```rust
// Development
let db_ops = DbOperationsV2::from_sled(sled::open("data")?).await?;

// Production
let db_ops = DbOperationsV2::from_dynamodb(client, "Table", user_id).await?;

// Testing
let db_ops = DbOperationsV2::from_namespaced_store(
    Arc::new(InMemoryNamespacedStore::new())
).await?;
```

## Available Backends

| Backend | Constructor | Use Case |
|---------|------------|----------|
| Sled | `from_sled(db)` | Local development, single machine |
| DynamoDB | `from_dynamodb(client, table, user_id)` | Cloud production, multi-tenant |
| In-Memory | `from_namespaced_store(InMemoryNamespacedStore::new())` | Unit tests |

## Namespaces

DbOperationsV2 uses these namespaces by default:
- `main` - Primary data storage
- `metadata` - System metadata
- `schemas` - Schema definitions
- `permissions` - Access control
- `transforms` - Data transformations
- `orchestrator_state` - Orchestration state
- `schema_states` - Schema states
- `public_keys` - Public key storage
- `transform_queue_tree` - Transform queue

## Error Handling

All operations return `Result<T, SchemaError>`:

```rust
match db_ops.get_item::<User>("user:123").await {
    Ok(Some(user)) => println!("Found: {:?}", user),
    Ok(None) => println!("Not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Type Safety

TypedStore automatically serializes/deserializes JSON:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

db_ops.store_item("user:1", &User {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
}).await?;

let user: Option<User> = db_ops.get_item("user:1").await?;
```

## Performance Tips

1. **Use batch operations** for multiple writes:
   ```rust
   db_ops.batch_store_items(&items).await?;
   ```

2. **Reuse DbOperations** instance (it's cheaply cloneable):
   ```rust
   let db_ops = DbOperationsV2::from_sled(db).await?;
   let db_ops_clone = db_ops.clone(); // Cheap!
   ```

3. **Use prefix scanning** instead of individual gets:
   ```rust
   let all_users = db_ops.list_items_with_prefix("user:").await?;
   ```

## Testing Pattern

```rust
use datafold::storage::InMemoryNamespacedStore;

async fn create_test_db() -> DbOperationsV2 {
    let store = Arc::new(InMemoryNamespacedStore::new());
    DbOperationsV2::from_namespaced_store(store).await.unwrap()
}

#[tokio::test]
async fn test_my_feature() {
    let db = create_test_db().await;
    // Test logic here
}
```

## Migration from DbOperations (v1)

Old code (v1):
```rust
let db_ops = DbOperations::new(db)?;
db_ops.store_item("key", &item)?;  // Sync
```

New code (v2):
```rust
let db_ops = DbOperationsV2::from_sled(db).await?;
db_ops.store_item("key", &item).await?;  // Async
```

## Need Help?

- 📖 [Full Implementation Docs](./STORAGE_ABSTRACTION_IMPLEMENTATION.md)
- 📖 [Design Document](./STORAGE_ABSTRACTION_DESIGN.md)
- 🧪 [Test Examples](../src/storage/tests.rs)
- 💻 [Source Code](../src/storage/)

## Summary

✅ **3 storage backends** (Sled, DynamoDB, In-Memory)  
✅ **Type-safe** JSON serialization  
✅ **Async-first** design  
✅ **Backward compatible**  
✅ **Fully tested** (12 tests)  
✅ **Production ready**  

Happy coding! 🚀
