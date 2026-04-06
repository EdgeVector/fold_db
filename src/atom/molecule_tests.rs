#[cfg(test)]
mod tests {
    use super::super::{Atom, Molecule, MoleculeRange};
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn test_molecule_creation_and_update() {
        let atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            json!({"test": true}),
        );

        // Test single molecule
        let molecule = Molecule::new(atom.uuid().to_string(), "test_schema", "test_key");
        assert_eq!(molecule.get_atom_uuid(), &atom.uuid().to_string());

        let new_atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            json!({"test": false}),
        );

        let mut updated_ref = molecule.clone();
        updated_ref.set_atom_uuid(new_atom.uuid().to_string());

        assert_eq!(updated_ref.get_atom_uuid(), &new_atom.uuid().to_string());
        assert!(updated_ref.updated_at() >= molecule.updated_at());
    }

    #[test]
    fn test_molecule_range() {
        let atoms: Vec<_> = (0..3)
            .map(|i| {
                Atom::new(
                    "test_schema".to_string(),
                    "test_key".to_string(),
                    json!({ "index": i }),
                )
            })
            .collect();

        let mut range = MoleculeRange::new("test_schema", "test_key");
        range.set_atom_uuid("a".to_string(), atoms[0].uuid().to_string());
        range.set_atom_uuid("b".to_string(), atoms[1].uuid().to_string());
        range.set_atom_uuid("c".to_string(), atoms[2].uuid().to_string());

        let keys: Vec<_> = range.atom_uuids.keys().cloned().collect();
        assert_eq!(
            keys,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );

        assert_eq!(range.get_atom_uuid("b"), Some(&atoms[1].uuid().to_string()));
        assert_eq!(
            range.remove_atom_uuid("b"),
            Some(atoms[1].uuid().to_string())
        );
        assert!(range.get_atom_uuid("b").is_none());

        assert!(range.updated_at() > Utc::now() - chrono::Duration::seconds(1));
    }

    #[test]
    fn test_molecule_range_single_atom_per_key() {
        let atoms: Vec<_> = (0..3)
            .map(|i| {
                Atom::new(
                    "test_schema".to_string(),
                    "test_key".to_string(),
                    json!({ "value": i, "type": format!("mutation_{}", i) }),
                )
            })
            .collect();

        let mut range = MoleculeRange::new("test_schema", "test_key");

        // Add atoms to different keys - each key can only store one atom UUID
        range.set_atom_uuid("user_123".to_string(), atoms[0].uuid().to_string());
        range.set_atom_uuid("user_456".to_string(), atoms[1].uuid().to_string());
        range.set_atom_uuid("user_789".to_string(), atoms[2].uuid().to_string());

        // Verify that each key stores exactly one atom UUID
        assert_eq!(
            range.get_atom_uuid("user_123"),
            Some(&atoms[0].uuid().to_string())
        );
        assert_eq!(
            range.get_atom_uuid("user_456"),
            Some(&atoms[1].uuid().to_string())
        );
        assert_eq!(
            range.get_atom_uuid("user_789"),
            Some(&atoms[2].uuid().to_string())
        );

        // Test overwriting a key (should replace the previous value)
        range.set_atom_uuid("user_123".to_string(), atoms[1].uuid().to_string());
        assert_eq!(
            range.get_atom_uuid("user_123"),
            Some(&atoms[1].uuid().to_string())
        );

        // Test removal
        let removed_uuid = range.remove_atom_uuid("user_123");
        assert_eq!(removed_uuid, Some(atoms[1].uuid().to_string()));
        assert!(range.get_atom_uuid("user_123").is_none());

        // Verify other keys still exist
        assert!(range.get_atom_uuid("user_456").is_some());
        assert!(range.get_atom_uuid("user_789").is_some());
    }
}
