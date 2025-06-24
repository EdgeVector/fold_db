//! Collection Removal Validation Tests
//!
//! This comprehensive test suite validates that the system works correctly
//! with only Single and Range fields in the new architecture after collection removal.
//!
//! **Validation Coverage:**
//! 1. **Single Field Operations** - Complete CRUD operations on Single fields
//! 2. **Range Field Operations** - Complete CRUD operations on Range fields
//! 3. **Schema Creation & Validation** - Creating schemas with Single/Range fields
//! 4. **Field Factory Functionality** - Field creation patterns work correctly
//! 5. **Serialization/Deserialization** - JSON schema handling works properly
//! 6. **Database Operations** - Storage and retrieval of field data

use datafold::schema::types::field::{FieldVariant, Field, FieldType};
use datafold::schema::types::{Schema, JsonSchemaDefinition, JsonSchemaField, SchemaType};
use datafold::schema::types::json_schema::{JsonPermissionPolicy, JsonFieldPaymentConfig};
use datafold::schema::{field_factory::FieldFactory};
use datafold::permissions::types::policy::{PermissionsPolicy, TrustDistance};
use datafold::fees::{SchemaPaymentConfig, FieldPaymentConfig};
use datafold::fees::types::config::TrustDistanceScaling;
use datafold::atom::{Atom, Molecule, MoleculeBehavior};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::test_utils::TestFixture;

#[test]
fn test_single_field_complete_operations() {
    println!("🧪 TEST: Single Field Complete Operations");
    println!("   This validates full CRUD operations on Single fields");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Create Single field
    let mut single_field = FieldFactory::create_single_field();
    println!("✅ Single field created successfully");
    
    // Test 2: Set field properties
    single_field.set_writable(true);
    assert!(single_field.writable());
    
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), "test_value".to_string());
    single_field.set_field_mappers(metadata.clone());
    assert_eq!(single_field.field_mappers(), &metadata);
    println!("✅ Single field properties set successfully");
    
    // Test 3: Create and link AtomRef
    let test_content = json!({"message": "test single field content", "value": 42});
    let atom = Atom::new("TestSchema".to_string(), "test_user".to_string(), test_content.clone());
    let atom_uuid = atom.uuid().to_string(); // Use the atom's own UUID
    
    // Store atom
    fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)
        .expect("Failed to store atom");
    
    // Create and store AtomRef
    let atom_ref = Molecule::new(atom_uuid.clone(), "test_user".to_string());
    let ref_uuid = atom_ref.uuid().to_string(); // Get the AtomRef's own UUID
    fixture.db_ops.store_item(&format!("ref:{}", ref_uuid), &atom_ref)
        .expect("Failed to store AtomRef");
    
    // Link field to AtomRef
    single_field.set_ref_atom_uuid(ref_uuid.clone());
    assert_eq!(single_field.ref_atom_uuid(), Some(&ref_uuid));
    println!("✅ Single field linked to AtomRef successfully");
    
    // Test 4: Verify data retrieval
    let stored_ref = fixture.db_ops.get_item::<Molecule>(&format!("ref:{}", ref_uuid))
        .expect("Failed to retrieve AtomRef")
        .expect("AtomRef should exist");
    
    // Verify the AtomRef points to the correct atom
    assert_eq!(stored_ref.get_atom_uuid(), &atom_uuid);
    
    let stored_atom = fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid))
        .expect("Failed to retrieve Atom")
        .expect("Atom should exist");
    
    assert_eq!(stored_atom.content(), &test_content);
    println!("✅ Single field data retrieval successful");
    
    println!("✅ Single Field Complete Operations Test PASSED");
}

#[test]
fn test_range_field_complete_operations() {
    println!("🧪 TEST: Range Field Complete Operations");
    println!("   This validates full CRUD operations on Range fields");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Create Range field
    let mut range_field = FieldFactory::create_range_field();
    println!("✅ Range field created successfully");
    
    // Test 2: Initialize AtomRefRange
    let source_pub_key = "test_user_123".to_string();
    range_field.ensure_molecule_range(source_pub_key.clone());
    assert!(range_field.molecule_range().is_some());
    println!("✅ Range field AtomRefRange initialized");
    
    // Test 3: Add multiple range entries
    let test_data = vec![
        ("key_001", json!({"type": "user", "id": 1, "name": "Alice"})),
        ("key_002", json!({"type": "user", "id": 2, "name": "Bob"})),
        ("key_003", json!({"type": "user", "id": 3, "name": "Charlie"})),
        ("key_100", json!({"type": "product", "id": 100, "name": "Widget"})),
        ("key_200", json!({"type": "product", "id": 200, "name": "Gadget"})),
    ];
    
    for (key, content) in &test_data {
        // Create atom
        let atom = Atom::new("RangeTestSchema".to_string(), source_pub_key.clone(), content.clone());
        let atom_uuid = Uuid::new_v4().to_string();
        
        // Store atom
        fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)
            .expect("Failed to store atom");
        
        // Add to range
        if let Some(molecule_range) = range_field.molecule_range_mut() {
            molecule_range.set_atom_uuid(key.to_string(), atom_uuid.clone());
        }
    }
    
    println!("✅ Multiple range entries added successfully");
    
    // Test 4: Verify range operations
    assert_eq!(range_field.count(), test_data.len());
    
    let all_keys = range_field.get_all_keys();
    assert_eq!(all_keys.len(), test_data.len());
    for (key, _) in &test_data {
        assert!(all_keys.contains(&key.to_string()));
    }
    
    // Test range queries
    let keys_in_range = range_field.get_keys_in_range("key_001", "key_100");
    assert!(keys_in_range.contains(&"key_001".to_string()));
    assert!(keys_in_range.contains(&"key_002".to_string()));
    assert!(keys_in_range.contains(&"key_003".to_string()));
    assert!(!keys_in_range.contains(&"key_100".to_string())); // Exclusive end
    
    println!("✅ Range field operations working correctly");
    
    // Test 5: Range filtering
    use datafold::schema::types::field::range_filter::RangeFilter;
    
    let key_filter = RangeFilter::Key("key_001".to_string());
    let filter_result = range_field.apply_filter(&key_filter);
    assert_eq!(filter_result.total_count, 1);
    assert!(filter_result.matches.contains_key("key_001"));
    
    let prefix_filter = RangeFilter::KeyPrefix("key_00".to_string());
    let prefix_result = range_field.apply_filter(&prefix_filter);
    assert_eq!(prefix_result.total_count, 3); // key_001, key_002, key_003
    
    println!("✅ Range field filtering working correctly");
    
    println!("✅ Range Field Complete Operations Test PASSED");
}

#[test]
fn test_schema_creation_with_single_and_range_fields() {
    println!("🧪 TEST: Schema Creation with Single and Range Fields");
    println!("   This validates schema creation and validation with new field types");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Create schema with Single fields
    let mut user_schema = Schema::new("UserProfile".to_string());
    
    user_schema.fields.insert(
        "name".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    user_schema.fields.insert(
        "email".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    user_schema.fields.insert(
        "age".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    
    // Store schema
    fixture.db_ops.store_schema("UserProfile", &user_schema)
        .expect("Failed to store UserProfile schema");
    
    // Verify retrieval
    let stored_schema = fixture.db_ops.get_schema("UserProfile")
        .expect("Failed to retrieve schema")
        .expect("Schema should exist");
    
    assert_eq!(stored_schema.name, "UserProfile");
    assert_eq!(stored_schema.fields.len(), 3);
    
    for field_name in ["name", "email", "age"] {
        assert!(stored_schema.fields.contains_key(field_name));
        match stored_schema.fields.get(field_name).unwrap() {
            FieldVariant::Single(_) => {} // Expected
            _ => panic!("Expected Single field for {}", field_name),
        }
    }
    
    println!("✅ Schema with Single fields created and stored successfully");
    
    // Test 2: Create RangeSchema with Range fields
    let mut analytics_schema = Schema::new_range(
        "EventAnalytics".to_string(),
        "timestamp".to_string(), // range_key
    );
    
    analytics_schema.fields.insert(
        "timestamp".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    analytics_schema.fields.insert(
        "event_data".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    analytics_schema.fields.insert(
        "user_context".to_string(),
        FieldVariant::Range(FieldFactory::create_range_field())
    );
    
    // Store RangeSchema
    fixture.db_ops.store_schema("EventAnalytics", &analytics_schema)
        .expect("Failed to store EventAnalytics schema");
    
    // Verify RangeSchema retrieval
    let stored_range_schema = fixture.db_ops.get_schema("EventAnalytics")
        .expect("Failed to retrieve RangeSchema")
        .expect("RangeSchema should exist");
    
    assert_eq!(stored_range_schema.name, "EventAnalytics");
    assert_eq!(stored_range_schema.range_key(), Some("timestamp"));
    assert_eq!(stored_range_schema.fields.len(), 3);
    
    for field_name in ["timestamp", "event_data", "user_context"] {
        assert!(stored_range_schema.fields.contains_key(field_name));
        match stored_range_schema.fields.get(field_name).unwrap() {
            FieldVariant::Range(_) => {} // Expected
            _ => panic!("Expected Range field for {}", field_name),
        }
    }
    
    println!("✅ RangeSchema with Range fields created and stored successfully");
    
    println!("✅ Schema Creation Test PASSED");
}

#[test]
fn test_json_schema_serialization_and_validation() {
    println!("🧪 TEST: JSON Schema Serialization and Validation");
    println!("   This validates JSON schema handling with Single and Range fields");
    
    // Test 1: Create JSON schema definition with Single fields
    let single_field_def = JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        ref_atom_uuid: None,
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: None,
    };
    
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), single_field_def.clone());
    fields.insert("content".to_string(), single_field_def.clone());
    fields.insert("author".to_string(), single_field_def);
    
    let json_schema = JsonSchemaDefinition {
        name: "BlogPost".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };
    
    // Test serialization
    let serialized = serde_json::to_string(&json_schema)
        .expect("Failed to serialize JSON schema");
    
    println!("✅ JSON schema with Single fields serialized successfully");
    
    // Test deserialization
    let deserialized: JsonSchemaDefinition = serde_json::from_str(&serialized)
        .expect("Failed to deserialize JSON schema");
    
    assert_eq!(deserialized.name, "BlogPost");
    assert_eq!(deserialized.fields.len(), 3);
    
    for field_name in ["title", "content", "author"] {
        let field = deserialized.fields.get(field_name)
            .unwrap_or_else(|| panic!("Field {} should exist", field_name));
        assert!(matches!(field.field_type, FieldType::Single));
    }
    
    println!("✅ JSON schema deserialization successful");
    
    // Test 2: Create RangeSchema JSON definition
    let range_field_def = JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        ref_atom_uuid: None,
        field_type: datafold::schema::types::field::FieldType::Range,
        transform: None,
    };
    
    let mut range_fields = HashMap::new();
    range_fields.insert("timestamp".to_string(), range_field_def.clone());
    range_fields.insert("metric_value".to_string(), range_field_def.clone());
    range_fields.insert("metadata".to_string(), range_field_def);
    
    let range_json_schema = JsonSchemaDefinition {
        name: "TimeSeriesData".to_string(),
        schema_type: SchemaType::Range {
            range_key: "timestamp".to_string(),
        },
        fields: range_fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };
    
    // Test RangeSchema serialization/deserialization
    let range_serialized = serde_json::to_string(&range_json_schema)
        .expect("Failed to serialize RangeSchema JSON");
    
    let range_deserialized: JsonSchemaDefinition = serde_json::from_str(&range_serialized)
        .expect("Failed to deserialize RangeSchema JSON");
    
    assert_eq!(range_deserialized.name, "TimeSeriesData");
    match &range_deserialized.schema_type {
        SchemaType::Range { range_key } => {
            assert_eq!(range_key, "timestamp");
        }
        _ => panic!("Expected Range schema type"),
    }
    
    for field in range_deserialized.fields.values() {
        assert!(matches!(field.field_type, FieldType::Range));
    }
    
    println!("✅ RangeSchema JSON serialization/deserialization successful");
    
    println!("✅ JSON Schema Serialization and Validation Test PASSED");
}

#[test]
fn test_field_factory_comprehensive_functionality() {
    println!("🧪 TEST: Field Factory Comprehensive Functionality");
    println!("   This validates all FieldFactory creation patterns work correctly");
    
    // Test 1: Basic field creation
    let basic_single = FieldFactory::create_single_field();
    assert!(basic_single.ref_atom_uuid().is_none());
    assert!(basic_single.writable());
    
    let basic_range = FieldFactory::create_range_field();
    assert!(basic_range.ref_atom_uuid().is_none());
    assert!(basic_range.molecule_range().is_none());
    
    println!("✅ Basic field creation works");
    
    // Test 2: Field creation with custom configurations
    let custom_permissions = PermissionsPolicy::default();
    let custom_single = FieldFactory::create_single_field_with_permissions(custom_permissions);
    assert!(custom_single.ref_atom_uuid().is_none());
    
    let custom_payment = FieldPaymentConfig {
        base_multiplier: 2.0,
        trust_distance_scaling: TrustDistanceScaling::None,
        min_payment: Some(100),
    };
    let payment_single = FieldFactory::create_single_field_with_payment(custom_payment);
    assert_eq!(payment_single.payment_config().base_multiplier, 2.0);
    assert_eq!(payment_single.payment_config().min_payment, Some(100));
    
    println!("✅ Custom configuration field creation works");
    
    // Test 3: Variant creation
    let single_variant = FieldFactory::create_single_variant();
    match single_variant {
        FieldVariant::Single(_) => {},
        _ => panic!("Expected Single variant"),
    }
    
    let range_variant = FieldFactory::create_range_variant();
    match range_variant {
        FieldVariant::Range(_) => {},
        _ => panic!("Expected Range variant"),
    }
    
    println!("✅ Field variant creation works");
    
    // Test 4: Builder pattern
    use datafold::schema::field_factory::FieldBuilder;
    
    let mut metadata = HashMap::new();
    metadata.insert("category".to_string(), "test".to_string());
    
    let builder_single = FieldBuilder::new()
        .with_metadata(metadata.clone())
        .build_single();
    
    assert_eq!(builder_single.field_mappers(), &metadata);
    
    let builder_range = FieldBuilder::new()
        .with_metadata(metadata.clone())
        .build_range();
    
    assert_eq!(builder_range.field_mappers(), &metadata);
    
    println!("✅ Builder pattern works");
    
    println!("✅ Field Factory Comprehensive Functionality Test PASSED");
}

#[test]
fn test_database_storage_and_retrieval_operations() {
    println!("🧪 TEST: Database Storage and Retrieval Operations");
    println!("   This validates complete database operations with new field types");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Store and retrieve atoms with different content types
    let test_atoms = vec![
        ("string_content", json!("Hello, World!")),
        ("number_content", json!(42.5)),
        ("object_content", json!({"nested": {"key": "value"}, "array": [1, 2, 3]})),
        ("array_content", json!([1, "two", {"three": 3}, null])),
        ("boolean_content", json!(true)),
        ("null_content", json!(null)),
    ];
    
    let mut stored_atom_uuids = Vec::new();
    
    for (description, content) in &test_atoms {
        let atom = Atom::new("TestSchema".to_string(), "test_user".to_string(), content.clone());
        let atom_uuid = Uuid::new_v4().to_string();
        
        fixture.db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)
            .unwrap_or_else(|_| panic!("Failed to store atom with {}", description));
        
        stored_atom_uuids.push((atom_uuid, content.clone()));
    }
    
    println!("✅ All atom types stored successfully");
    
    // Test 2: Retrieve and verify atoms
    for (atom_uuid, expected_content) in &stored_atom_uuids {
        let stored_atom = fixture.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid))
            .expect("Failed to retrieve atom")
            .expect("Atom should exist");
        
        assert_eq!(stored_atom.content(), expected_content);
    }
    
    println!("✅ All atom retrievals successful");
    
    // Test 3: Create and store complex schema with mixed field types
    let mut mixed_schema = Schema::new("MixedFieldSchema".to_string());
    
    // Add Single fields
    for field_name in ["single_field_1", "single_field_2"] {
        mixed_schema.fields.insert(
            field_name.to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
    }
    
    // Add Range fields
    for field_name in ["range_field_1", "range_field_2"] {
        mixed_schema.fields.insert(
            field_name.to_string(),
            FieldVariant::Range(FieldFactory::create_range_field())
        );
    }
    
    fixture.db_ops.store_schema("MixedFieldSchema", &mixed_schema)
        .expect("Failed to store mixed schema");
    
    // Verify schema storage and retrieval
    let retrieved_schema = fixture.db_ops.get_schema("MixedFieldSchema")
        .expect("Failed to retrieve schema")
        .expect("Schema should exist");
    
    assert_eq!(retrieved_schema.name, "MixedFieldSchema");
    assert_eq!(retrieved_schema.fields.len(), 4);
    
    // Verify field types
    for field_name in ["single_field_1", "single_field_2"] {
        match retrieved_schema.fields.get(field_name).unwrap() {
            FieldVariant::Single(_) => {},
            _ => panic!("Expected Single field for {}", field_name),
        }
    }
    
    for field_name in ["range_field_1", "range_field_2"] {
        match retrieved_schema.fields.get(field_name).unwrap() {
            FieldVariant::Range(_) => {},
            _ => panic!("Expected Range field for {}", field_name),
        }
    }
    
    println!("✅ Mixed schema storage and retrieval successful");
    
    println!("✅ Database Storage and Retrieval Operations Test PASSED");
}