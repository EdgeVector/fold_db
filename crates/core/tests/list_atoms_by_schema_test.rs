use fold_db::atom::Atom;
use fold_db::testing_utils::TestDatabaseFactory;
use serde_json::json;

#[tokio::test]
async fn test_list_atoms_by_schema_returns_only_matching_schema() {
    let db_ops = TestDatabaseFactory::create_temp_db_ops()
        .await
        .expect("Failed to create DB");

    let blog_a = Atom::new("BlogPost".into(), json!({"title": "A"}));
    let blog_b = Atom::new("BlogPost".into(), json!({"title": "B"}));
    let blog_c = Atom::new("BlogPost".into(), json!({"title": "C"}));
    let other = Atom::new("OtherSchema".into(), json!({"x": 1}));

    db_ops
        .batch_store_atoms(
            vec![
                blog_a.clone(),
                blog_b.clone(),
                blog_c.clone(),
                other.clone(),
            ],
            None,
        )
        .await
        .expect("batch store failed");

    let atoms = db_ops
        .list_atoms_by_schema("BlogPost", None)
        .await
        .expect("list_atoms_by_schema failed");

    assert_eq!(atoms.len(), 3, "should only return BlogPost atoms");
    for a in &atoms {
        assert_eq!(a.source_schema_name(), "BlogPost");
    }

    let mut expected_uuids = vec![
        blog_a.uuid().to_string(),
        blog_b.uuid().to_string(),
        blog_c.uuid().to_string(),
    ];
    expected_uuids.sort();
    let actual_uuids: Vec<String> = atoms.iter().map(|a| a.uuid().to_string()).collect();
    assert_eq!(
        actual_uuids, expected_uuids,
        "results should be sorted by uuid for deterministic snapshots"
    );
}

#[tokio::test]
async fn test_list_atoms_by_schema_empty_when_no_match() {
    let db_ops = TestDatabaseFactory::create_temp_db_ops()
        .await
        .expect("Failed to create DB");

    db_ops
        .batch_store_atoms(vec![Atom::new("SchemaA".into(), json!({"v": 1}))], None)
        .await
        .expect("batch store failed");

    let atoms = db_ops
        .list_atoms_by_schema("SchemaB", None)
        .await
        .expect("list_atoms_by_schema failed");

    assert!(atoms.is_empty());
}

#[tokio::test]
async fn test_list_atoms_by_schema_org_falls_back_to_personal() {
    let db_ops = TestDatabaseFactory::create_temp_db_ops()
        .await
        .expect("Failed to create DB");

    let atom = Atom::new("SchemaX".into(), json!({"v": 1}));
    db_ops
        .batch_store_atoms(vec![atom.clone()], None)
        .await
        .expect("batch store failed");

    let atoms = db_ops
        .list_atoms_by_schema("SchemaX", Some("orghash"))
        .await
        .expect("list_atoms_by_schema failed");

    assert_eq!(atoms.len(), 1, "should fall back to personal namespace");
    assert_eq!(atoms[0].uuid(), atom.uuid());
}
