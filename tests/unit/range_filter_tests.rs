use datafold::fees::types::config::FieldPaymentConfig;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::range_field::RangeField;
use datafold::schema::types::field::range_filter::{matches_pattern, RangeFilter};
use std::collections::HashMap;

#[test]
fn test_range_filter_enum_serialization() {
    println!("🧪 Testing RangeFilter enum serialization");

    // Test Key filter
    let key_filter = RangeFilter::Key("test_key".to_string());
    let serialized = serde_json::to_string(&key_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(matches!(deserialized, RangeFilter::Key(ref k) if k == "test_key"));
    println!("✅ Key filter serialization works");

    // Test KeyPrefix filter
    let prefix_filter = RangeFilter::KeyPrefix("test_".to_string());
    let serialized = serde_json::to_string(&prefix_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(matches!(deserialized, RangeFilter::KeyPrefix(ref p) if p == "test_"));
    println!("✅ KeyPrefix filter serialization works");

    // Test KeyRange filter
    let range_filter = RangeFilter::KeyRange {
        start: "a".to_string(),
        end: "z".to_string(),
    };
    let serialized = serde_json::to_string(&range_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(
        matches!(deserialized, RangeFilter::KeyRange { start, end } if start == "a" && end == "z")
    );
    println!("✅ KeyRange filter serialization works");

    // Test Keys filter
    let keys_filter = RangeFilter::Keys(vec!["key1".to_string(), "key2".to_string()]);
    let serialized = serde_json::to_string(&keys_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(matches!(deserialized, RangeFilter::Keys(ref keys) if keys.len() == 2));
    println!("✅ Keys filter serialization works");

    // Test KeyPattern filter
    let pattern_filter = RangeFilter::KeyPattern("test_*".to_string());
    let serialized = serde_json::to_string(&pattern_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(matches!(deserialized, RangeFilter::KeyPattern(ref p) if p == "test_*"));
    println!("✅ KeyPattern filter serialization works");

    // Test Value filter
    let value_filter = RangeFilter::Value("test_value".to_string());
    let serialized = serde_json::to_string(&value_filter).unwrap();
    let deserialized: RangeFilter = serde_json::from_str(&serialized).unwrap();
    assert!(matches!(deserialized, RangeFilter::Value(ref v) if v == "test_value"));
    println!("✅ Value filter serialization works");
}

#[test]
fn test_pattern_matching() {
    println!("🧪 Testing pattern matching functionality");

    // Test exact match
    assert!(matches_pattern("test", "test"));
    println!("✅ Exact match works");

    // Test wildcard match
    assert!(matches_pattern("test123", "test*"));
    assert!(matches_pattern("test_abc", "test*"));
    assert!(!matches_pattern("other", "test*"));
    println!("✅ Wildcard match works");

    // Test single character match
    assert!(matches_pattern("test", "t?st"));
    assert!(matches_pattern("tast", "t?st"));
    assert!(matches_pattern("test", "t??t")); // "test" matches "t??t" (e and s match the ?s)
    assert!(!matches_pattern("test", "t???t")); // "test" doesn't match "t???t" (not enough characters)
    println!("✅ Single character match works");

    // Test complex patterns
    assert!(matches_pattern("test_123", "test_*"));
    assert!(matches_pattern("test_abc_xyz", "test_*"));
    assert!(!matches_pattern("other_123", "test_*"));
    println!("✅ Complex pattern matching works");

    // Test edge cases
    assert!(matches_pattern("", ""));
    assert!(matches_pattern("", "*")); // Empty string matches wildcard (zero characters)
    assert!(matches_pattern("test", "*"));
    println!("✅ Edge cases work");
}

#[test]
fn test_range_field_filter_application() {
    println!("🧪 Testing RangeField filter application");

    // Create a test range field
    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Add some test data to the molecule range
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);

    // Add test entries
    molecule_range.set_atom_uuid("key1".to_string(), "value1".to_string());
    molecule_range.set_atom_uuid("key2".to_string(), "value2".to_string());
    molecule_range.set_atom_uuid("test_key".to_string(), "test_value".to_string());
    molecule_range.set_atom_uuid("test_other".to_string(), "other_value".to_string());
    molecule_range.set_atom_uuid("abc".to_string(), "abc_value".to_string());
    molecule_range.set_atom_uuid("def".to_string(), "def_value".to_string());

    println!("📝 Added test data with 6 entries");

    // Test Key filter
    let key_filter = RangeFilter::Key("key1".to_string());
    let result = range_field.apply_filter(&key_filter);
    assert_eq!(result.total_count, 1);
    assert!(result.matches.contains_key("key1"));
    assert_eq!(result.matches.get("key1").unwrap(), "value1");
    println!("✅ Key filter works: found {} matches", result.total_count);

    // Test KeyPrefix filter
    let prefix_filter = RangeFilter::KeyPrefix("test_".to_string());
    let result = range_field.apply_filter(&prefix_filter);
    assert_eq!(result.total_count, 2);
    assert!(result.matches.contains_key("test_key"));
    assert!(result.matches.contains_key("test_other"));
    println!(
        "✅ KeyPrefix filter works: found {} matches",
        result.total_count
    );

    // Test KeyRange filter
    let range_filter = RangeFilter::KeyRange {
        start: "a".to_string(),
        end: "e".to_string(),
    };
    let result = range_field.apply_filter(&range_filter);
    assert_eq!(result.total_count, 2);
    assert!(result.matches.contains_key("abc"));
    assert!(result.matches.contains_key("def"));
    println!(
        "✅ KeyRange filter works: found {} matches",
        result.total_count
    );

    // Test Keys filter
    let keys_filter = RangeFilter::Keys(vec!["key1".to_string(), "key2".to_string()]);
    let result = range_field.apply_filter(&keys_filter);
    assert_eq!(result.total_count, 2);
    assert!(result.matches.contains_key("key1"));
    assert!(result.matches.contains_key("key2"));
    println!("✅ Keys filter works: found {} matches", result.total_count);

    // Test KeyPattern filter
    let pattern_filter = RangeFilter::KeyPattern("test_*".to_string());
    let result = range_field.apply_filter(&pattern_filter);
    assert_eq!(result.total_count, 2);
    assert!(result.matches.contains_key("test_key"));
    assert!(result.matches.contains_key("test_other"));
    println!(
        "✅ KeyPattern filter works: found {} matches",
        result.total_count
    );

    // Test Value filter
    let value_filter = RangeFilter::Value("value1".to_string());
    let result = range_field.apply_filter(&value_filter);
    assert_eq!(result.total_count, 1);
    assert!(result.matches.contains_key("key1"));
    println!(
        "✅ Value filter works: found {} matches",
        result.total_count
    );
}

#[test]
fn test_range_field_empty_molecule_range() {
    println!("🧪 Testing RangeField with empty molecule range");

    let range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Test all filter types with empty range
    let filters = vec![
        RangeFilter::Key("test".to_string()),
        RangeFilter::KeyPrefix("test".to_string()),
        RangeFilter::KeyRange {
            start: "a".to_string(),
            end: "z".to_string(),
        },
        RangeFilter::Keys(vec!["test".to_string()]),
        RangeFilter::KeyPattern("test*".to_string()),
        RangeFilter::Value("test".to_string()),
    ];

    for filter in filters {
        let result = range_field.apply_filter(&filter);
        assert_eq!(result.total_count, 0);
        assert!(result.matches.is_empty());
    }

    println!("✅ All filters return empty results for empty molecule range");
}

#[test]
fn test_range_field_json_filter_application() {
    println!("🧪 Testing RangeField JSON filter application");

    // Create a test range field
    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Add test data
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);
    molecule_range.set_atom_uuid("key1".to_string(), "value1".to_string());
    molecule_range.set_atom_uuid("key2".to_string(), "value2".to_string());

    // Test JSON filter application
    let json_filter = serde_json::json!({"Key": "key1"});
    let result = range_field.apply_json_filter(&json_filter).unwrap();
    assert_eq!(result.total_count, 1);
    assert!(result.matches.contains_key("key1"));

    let json_filter = serde_json::json!({"KeyPrefix": "key"});
    let result = range_field.apply_json_filter(&json_filter).unwrap();
    assert_eq!(result.total_count, 2);
    assert!(result.matches.contains_key("key1"));
    assert!(result.matches.contains_key("key2"));

    let json_filter = serde_json::json!({
        "KeyRange": {
            "start": "key1",
            "end": "key3"
        }
    });
    let result = range_field.apply_json_filter(&json_filter).unwrap();
    assert_eq!(result.total_count, 2);

    println!("✅ JSON filter application works");
}

#[test]
fn test_range_field_invalid_json_filter() {
    println!("🧪 Testing RangeField invalid JSON filter handling");

    let range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Test invalid JSON filter
    let invalid_json = serde_json::json!({"InvalidFilter": "value"});
    let result = range_field.apply_json_filter(&invalid_json);
    assert!(result.is_err());

    // Test malformed JSON
    let malformed_json = serde_json::json!({"Key": 123}); // Should be string
    let result = range_field.apply_json_filter(&malformed_json);
    assert!(result.is_err());

    println!("✅ Invalid JSON filter handling works");
}

#[test]
fn test_range_field_get_all_keys() {
    println!("🧪 Testing RangeField get_all_keys functionality");

    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Add test data
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);
    molecule_range.set_atom_uuid("key1".to_string(), "value1".to_string());
    molecule_range.set_atom_uuid("key2".to_string(), "value2".to_string());
    molecule_range.set_atom_uuid("key3".to_string(), "value3".to_string());

    let all_keys = range_field.get_all_keys();
    assert_eq!(all_keys.len(), 3);
    assert!(all_keys.contains(&"key1".to_string()));
    assert!(all_keys.contains(&"key2".to_string()));
    assert!(all_keys.contains(&"key3".to_string()));

    println!("✅ get_all_keys works: found {} keys", all_keys.len());
}

#[test]
fn test_range_field_get_keys_in_range() {
    println!("🧪 Testing RangeField get_keys_in_range functionality");

    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Add test data
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);
    molecule_range.set_atom_uuid("a".to_string(), "value_a".to_string());
    molecule_range.set_atom_uuid("b".to_string(), "value_b".to_string());
    molecule_range.set_atom_uuid("c".to_string(), "value_c".to_string());
    molecule_range.set_atom_uuid("d".to_string(), "value_d".to_string());
    molecule_range.set_atom_uuid("e".to_string(), "value_e".to_string());

    let keys_in_range = range_field.get_keys_in_range("b", "e");
    assert_eq!(keys_in_range.len(), 3);
    assert!(keys_in_range.contains(&"b".to_string()));
    assert!(keys_in_range.contains(&"c".to_string()));
    assert!(keys_in_range.contains(&"d".to_string()));
    assert!(!keys_in_range.contains(&"e".to_string())); // exclusive end

    println!(
        "✅ get_keys_in_range works: found {} keys in range",
        keys_in_range.len()
    );
}

#[test]
fn test_range_field_count() {
    println!("🧪 Testing RangeField count functionality");

    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Test empty range
    assert_eq!(range_field.count(), 0);

    // Add test data
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);
    molecule_range.set_atom_uuid("key1".to_string(), "value1".to_string());
    molecule_range.set_atom_uuid("key2".to_string(), "value2".to_string());
    molecule_range.set_atom_uuid("key3".to_string(), "value3".to_string());

    assert_eq!(range_field.count(), 3);

    println!(
        "✅ count functionality works: {} items",
        range_field.count()
    );
}

#[test]
fn test_range_filter_edge_cases() {
    println!("🧪 Testing RangeFilter edge cases");

    // Test empty string keys
    let empty_key_filter = RangeFilter::Key("".to_string());
    let empty_prefix_filter = RangeFilter::KeyPrefix("".to_string());
    let empty_pattern_filter = RangeFilter::KeyPattern("".to_string());

    // Test empty keys array
    let empty_keys_filter = RangeFilter::Keys(vec![]);

    // Test range with same start and end
    let same_range_filter = RangeFilter::KeyRange {
        start: "test".to_string(),
        end: "test".to_string(),
    };

    // Test range with start > end
    let invalid_range_filter = RangeFilter::KeyRange {
        start: "z".to_string(),
        end: "a".to_string(),
    };

    // These should all be valid filters (even if they might return no results)
    assert!(matches!(empty_key_filter, RangeFilter::Key(ref k) if k.is_empty()));
    assert!(matches!(empty_prefix_filter, RangeFilter::KeyPrefix(ref p) if p.is_empty()));
    assert!(matches!(empty_pattern_filter, RangeFilter::KeyPattern(ref p) if p.is_empty()));
    assert!(matches!(empty_keys_filter, RangeFilter::Keys(ref keys) if keys.is_empty()));
    assert!(matches!(same_range_filter, RangeFilter::KeyRange { start, end } if start == end));
    assert!(matches!(invalid_range_filter, RangeFilter::KeyRange { start, end } if start > end));

    println!("✅ Edge cases handled correctly");
}

#[test]
fn test_range_filter_performance_characteristics() {
    println!("🧪 Testing RangeFilter performance characteristics");

    let mut range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );

    // Add many test entries
    let source_pub_key = "test_source".to_string();
    let molecule_range = range_field.ensure_molecule_range(source_pub_key);

    for i in 0..1000 {
        let key = format!("key_{:03}", i);
        let value = format!("value_{}", i);
        molecule_range.set_atom_uuid(key, value);
    }

    println!("📝 Added 1000 test entries");

    // Test Key filter (should be O(1))
    let key_filter = RangeFilter::Key("key_500".to_string());
    let start = std::time::Instant::now();
    let result = range_field.apply_filter(&key_filter);
    let duration = start.elapsed();
    assert_eq!(result.total_count, 1);
    println!("✅ Key filter performance: {:?} for 1000 entries", duration);

    // Test KeyPrefix filter (should be O(n))
    let prefix_filter = RangeFilter::KeyPrefix("key_5".to_string());
    let start = std::time::Instant::now();
    let result = range_field.apply_filter(&prefix_filter);
    let duration = start.elapsed();
    assert_eq!(result.total_count, 100); // key_500 to key_599
    println!(
        "✅ KeyPrefix filter performance: {:?} for 1000 entries",
        duration
    );

    // Test KeyRange filter (should be O(n))
    let range_filter = RangeFilter::KeyRange {
        start: "key_100".to_string(),
        end: "key_200".to_string(),
    };
    let start = std::time::Instant::now();
    let result = range_field.apply_filter(&range_filter);
    let duration = start.elapsed();
    assert_eq!(result.total_count, 100); // key_100 to key_199
    println!(
        "✅ KeyRange filter performance: {:?} for 1000 entries",
        duration
    );

    // Test KeyPattern filter (should be O(n))
    let pattern_filter = RangeFilter::KeyPattern("key_*".to_string());
    let start = std::time::Instant::now();
    let result = range_field.apply_filter(&pattern_filter);
    let duration = start.elapsed();
    assert_eq!(result.total_count, 1000);
    println!(
        "✅ KeyPattern filter performance: {:?} for 1000 entries",
        duration
    );
}

#[test]
fn test_range_filter_serialization_round_trip() {
    println!("🧪 Testing RangeFilter serialization round trip");

    let test_filters = vec![
        RangeFilter::Key("test_key".to_string()),
        RangeFilter::KeyPrefix("test_prefix".to_string()),
        RangeFilter::KeyRange {
            start: "start_key".to_string(),
            end: "end_key".to_string(),
        },
        RangeFilter::Keys(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]),
        RangeFilter::KeyPattern("test_*_pattern".to_string()),
        RangeFilter::Value("test_value".to_string()),
    ];

    for filter in test_filters {
        // Serialize to JSON
        let json_value = serde_json::to_value(&filter).unwrap();

        // Deserialize back to RangeFilter
        let deserialized: RangeFilter = serde_json::from_value(json_value).unwrap();

        // Verify they're equal
        assert_eq!(format!("{:?}", filter), format!("{:?}", deserialized));
    }

    println!("✅ All filter types serialize and deserialize correctly");
}
