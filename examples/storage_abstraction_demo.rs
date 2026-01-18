///! Storage Abstraction Demonstration
///!
///! This example shows how to use the storage abstraction layer (DbOperations)
///! with different backends: Sled, DynamoDB, and In-Memory.
///!
///! Run with:
///! ```bash
///! cargo run --example storage_abstraction_demo
///! ```
use datafold::db_operations::DbOperations;
use datafold::storage::{InMemoryNamespacedStore, NamespacedStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: String,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════╗");
    println!("║   Storage Abstraction Demonstration       ║");
    println!("╚════════════════════════════════════════════╝");
    println!();

    // ============================================
    // Example 1: Sled Backend (Local Storage)
    // ============================================
    println!("📁 Example 1: Sled Backend (Local Embedded Database)");
    println!("────────────────────────────────────────────");

    let temp_dir = tempfile::tempdir()?;
    let sled_db = sled::open(temp_dir.path())?;
    let db_ops_sled = DbOperations::from_sled(sled_db).await?;

    // Store data
    let user = User {
        id: "user_001".to_string(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    db_ops_sled.store_item("users:alice", &user).await?;
    println!("✅ Stored user: {:?}", user);

    // Retrieve data
    let retrieved: Option<User> = db_ops_sled.get_item("users:alice").await?;
    println!("✅ Retrieved user: {:?}", retrieved);

    // List keys
    let keys = db_ops_sled.list_items_with_prefix("users:").await?;
    println!("✅ Keys with prefix 'users:': {:?}", keys);

    println!("✅ Backend: {}", "Sled");
    println!();

    // ============================================
    // Example 2: In-Memory Backend (Testing)
    // ============================================
    println!("🧪 Example 2: In-Memory Backend (Fast Testing)");
    println!("────────────────────────────────────────────");

    let mem_store = Arc::new(InMemoryNamespacedStore::new());
    let db_ops_mem = DbOperations::from_namespaced_store(mem_store).await?;

    // Batch insert
    let users = vec![
        (
            "users:bob".to_string(),
            User {
                id: "user_002".to_string(),
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ),
        (
            "users:charlie".to_string(),
            User {
                id: "user_003".to_string(),
                name: "Charlie".to_string(),
                email: "charlie@example.com".to_string(),
            },
        ),
    ];

    db_ops_mem.batch_store_items(&users).await?;
    println!("✅ Batch stored {} users", users.len());

    // Query all users
    let all_keys = db_ops_mem.list_items_with_prefix("users:").await?;
    println!("✅ Total users: {}", all_keys.len());

    for key in all_keys {
        let user: Option<User> = db_ops_mem.get_item(&key).await?;
        if let Some(u) = user {
            println!("   - {} ({})", u.name, u.email);
        }
    }

    println!("✅ Backend: {}", "In-Memory");
    println!();

    // ============================================
    // Example 3: Namespace Operations
    // ============================================
    println!("📂 Example 3: Namespace Operations");
    println!("────────────────────────────────────────────");

    let store = Arc::new(InMemoryNamespacedStore::new());
    let db_ops = DbOperations::from_namespaced_store(store.clone()).await?;

    // Store in different namespaces
    db_ops
        .store_in_namespace(
            "users",
            "alice",
            &User {
                id: "user_001".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        )
        .await?;

    db_ops
        .store_in_namespace("products", "laptop", &"MacBook Pro".to_string())
        .await?;

    println!("✅ Stored data in different namespaces");

    // List namespaces - Arc implements Deref, so we can call methods directly
    let namespaces = store.as_ref().list_namespaces().await?;
    println!("✅ Available namespaces: {:?}", namespaces);

    // Query specific namespace
    let user_keys = db_ops.list_keys_in_namespace("users").await?;
    println!("✅ Keys in 'users' namespace: {:?}", user_keys);

    println!();

    // ============================================
    // Example 4: Same API, Different Backends!
    // ============================================
    println!("🔄 Example 4: Same API Across All Backends");
    println!("────────────────────────────────────────────");

    async fn store_and_retrieve<T>(
        db_ops: &DbOperations,
        backend_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + Clone,
    {
        let user = User {
            id: "demo".to_string(),
            name: format!("User from {}", backend_name),
            email: format!("user@{}.com", backend_name.to_lowercase()),
        };

        db_ops.store_item("demo", &user).await?;
        let retrieved: Option<User> = db_ops.get_item("demo").await?;

        println!("   {} → {:?}", backend_name, retrieved);
        Ok(())
    }

    // All three backends use the exact same API!
    store_and_retrieve::<User>(&db_ops_sled, "Sled").await?;
    store_and_retrieve::<User>(&db_ops_mem, "In-Memory").await?;
    // store_and_retrieve::<User>(&db_ops_dynamo, "DynamoDB").await?; // Would work too!

    println!();
    println!("╔════════════════════════════════════════════╗");
    println!("║  ✅ Storage Abstraction Working!           ║");
    println!("║                                            ║");
    println!("║  • Same API for all backends               ║");
    println!("║  • Type-safe operations                    ║");
    println!("║  • Async-first design                      ║");
    println!("║  • Namespace isolation                     ║");
    println!("╚════════════════════════════════════════════╝");

    Ok(())
}
