use fold_db::atom::Atom;
use fold_db::testing_utils::TestDatabaseFactory;
use serde_json::json;

#[test]
fn test_atom_content_based_uuid() {
    // Create two atoms with identical content
    let content = json!({"title": "Test Post", "body": "Test content"});
    let atom1 = Atom::new("BlogPost".to_string(), "user1".to_string(), content.clone());
    let atom2 = Atom::new("BlogPost".to_string(), "user2".to_string(), content.clone());

    // Both atoms should have the same UUID because they have the same schema and content
    assert_eq!(
        atom1.uuid(),
        atom2.uuid(),
        "Atoms with identical schema and content should have the same UUID"
    );

    // Create an atom with different content
    let different_content = json!({"title": "Different Post", "body": "Different content"});
    let atom3 = Atom::new(
        "BlogPost".to_string(),
        "user1".to_string(),
        different_content,
    );

    // This atom should have a different UUID
    assert_ne!(
        atom1.uuid(),
        atom3.uuid(),
        "Atoms with different content should have different UUIDs"
    );

    // Create an atom with same content but different schema
    let atom4 = Atom::new(
        "DifferentSchema".to_string(),
        "user1".to_string(),
        content.clone(),
    );

    // This atom should have a different UUID
    assert_ne!(
        atom1.uuid(),
        atom4.uuid(),
        "Atoms with same content but different schema should have different UUIDs"
    );
}

#[tokio::test]
async fn test_atom_deduplication_in_db() {
    let db_ops = TestDatabaseFactory::create_temp_db_ops()
        .await
        .expect("Failed to create DB");

    // Create the first atom
    let content = json!({"title": "Duplicate Test", "body": "This is duplicate content"});
    let atom1 = db_ops
        .create_and_store_atom_for_mutation_deferred("TestSchema", "user1", content.clone(), None)
        .await
        .expect("Failed to create atom1");

    // Try to create a second atom with the same content
    let atom2 = db_ops
        .create_and_store_atom_for_mutation_deferred("TestSchema", "user2", content.clone(), None)
        .await
        .expect("Failed to create atom2");

    // Both should have the same UUID (deduplication)
    assert_eq!(
        atom1.uuid(),
        atom2.uuid(),
        "Database should return the same atom for identical content"
    );

    // The second atom should be the exact same as the first (including source_pub_key)
    // because we retrieved the existing atom from the database
    assert_eq!(
        atom1.source_pub_key(),
        atom2.source_pub_key(),
        "Deduplication should return the original atom with its original metadata"
    );

    // Verify only one atom is stored in the database
    // Use the same pattern as in atom_operations_v2.rs
    let atom_key = format!("atom:{}", atom1.uuid());
    // atoms_store() returns &Arc<TypedKvStore>, dereference to get TypedKvStore which implements TypedStore
    use fold_db::storage::traits::TypedStore;
    let stored_atom: Option<Atom> = (**db_ops.atoms_store())
        .get_item::<Atom>(&atom_key)
        .await
        .map_err(|e| {
            datafold::schema::SchemaError::InvalidData(format!(
                "Failed to check existing atom: {}",
                e
            ))
        })
        .expect("Failed to get atom");

    assert!(stored_atom.is_some(), "Atom should be stored in database");
    let stored = stored_atom.unwrap();
    assert_eq!(stored.uuid(), atom1.uuid());
    assert_eq!(stored.source_pub_key(), atom1.source_pub_key());
}

#[test]
fn test_atom_uuid_deterministic() {
    // Create the same atom multiple times
    let content = json!({"key": "value", "number": 42});

    let atom1 = Atom::new("Schema1".to_string(), "userA".to_string(), content.clone());
    let atom2 = Atom::new("Schema1".to_string(), "userB".to_string(), content.clone());
    let atom3 = Atom::new("Schema1".to_string(), "userC".to_string(), content.clone());

    // All should have identical UUIDs
    assert_eq!(atom1.uuid(), atom2.uuid());
    assert_eq!(atom2.uuid(), atom3.uuid());
    assert_eq!(atom1.uuid(), atom3.uuid());

    // The UUID should be a valid SHA256 hash (64 hex characters)
    assert_eq!(
        atom1.uuid().len(),
        64,
        "UUID should be 64 characters (SHA256 hex)"
    );
    assert!(
        atom1.uuid().chars().all(|c| c.is_ascii_hexdigit()),
        "UUID should be a valid hex string"
    );
}
