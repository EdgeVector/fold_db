use datafold::transform::{FieldValue, NativeFieldType};
use std::collections::HashMap;
use std::f64::consts::PI;

#[test]
fn field_type_infers_scalar_and_collection_variants() {
    assert_eq!(
        FieldValue::String("hello".to_string()).field_type(),
        NativeFieldType::String
    );
    assert_eq!(
        FieldValue::Integer(42).field_type(),
        NativeFieldType::Integer
    );
    assert_eq!(FieldValue::Number(PI).field_type(), NativeFieldType::Number);
    assert_eq!(
        FieldValue::Boolean(true).field_type(),
        NativeFieldType::Boolean
    );

    let array_value = FieldValue::Array(vec![
        FieldValue::String("a".to_string()),
        FieldValue::Null,
        FieldValue::String("b".to_string()),
    ]);
    assert_eq!(
        array_value.field_type(),
        NativeFieldType::Array {
            element_type: Box::new(NativeFieldType::String),
        }
    );

    let mut object_entries = HashMap::new();
    object_entries.insert("name".to_string(), FieldValue::String("Ada".to_string()));
    object_entries.insert("age".to_string(), FieldValue::Integer(37));
    let object_value = FieldValue::Object(object_entries);

    let expected_object_type = NativeFieldType::Object {
        fields: HashMap::from([
            ("name".to_string(), NativeFieldType::String),
            ("age".to_string(), NativeFieldType::Integer),
        ]),
    };
    assert_eq!(object_value.field_type(), expected_object_type);

    assert_eq!(FieldValue::Null.field_type(), NativeFieldType::Null);

    let mixed_array = FieldValue::Array(vec![
        FieldValue::String("x".to_string()),
        FieldValue::Number(1.0),
    ]);
    assert_eq!(
        mixed_array.field_type(),
        NativeFieldType::Array {
            element_type: Box::new(NativeFieldType::Null),
        }
    );
}

#[test]
fn field_value_json_round_trip_preserves_structure() {
    let value = FieldValue::Object(HashMap::from([
        (
            "title".to_string(),
            FieldValue::String("Rust Native Types".to_string()),
        ),
        ("views".to_string(), FieldValue::Integer(128)),
        (
            "tags".to_string(),
            FieldValue::Array(vec![
                FieldValue::String("rust".to_string()),
                FieldValue::Null,
                FieldValue::String("transforms".to_string()),
            ]),
        ),
    ]));

    let json_value = value.to_json_value();
    let round_tripped = FieldValue::from_json_value(json_value);

    assert_eq!(round_tripped, value);
}

#[test]
fn field_type_matching_validates_values() {
    let string_type = NativeFieldType::String;
    assert!(string_type.matches(&FieldValue::String("value".to_string())));
    assert!(string_type.matches(&FieldValue::Null));
    assert!(!string_type.matches(&FieldValue::Integer(5)));

    let array_type = NativeFieldType::Array {
        element_type: Box::new(NativeFieldType::Number),
    };
    let numeric_array = FieldValue::Array(vec![
        FieldValue::Number(1.23),
        FieldValue::Null,
        FieldValue::Number(9.87),
    ]);
    assert!(array_type.matches(&numeric_array));

    let invalid_array = FieldValue::Array(vec![
        FieldValue::Number(1.23),
        FieldValue::String("nope".to_string()),
    ]);
    assert!(!array_type.matches(&invalid_array));

    let object_type = NativeFieldType::Object {
        fields: HashMap::from([
            ("name".to_string(), NativeFieldType::String),
            ("active".to_string(), NativeFieldType::Boolean),
        ]),
    };
    let valid_object = FieldValue::Object(HashMap::from([
        ("name".to_string(), FieldValue::String("Riley".to_string())),
        ("active".to_string(), FieldValue::Boolean(true)),
        ("extra".to_string(), FieldValue::Integer(1)),
    ]));
    assert!(object_type.matches(&valid_object));

    let missing_field_object = FieldValue::Object(HashMap::from([(
        "name".to_string(),
        FieldValue::String("Riley".to_string()),
    )]));
    assert!(!object_type.matches(&missing_field_object));

    let null_type = NativeFieldType::Null;
    assert!(null_type.matches(&FieldValue::Null));
    assert!(!null_type.matches(&FieldValue::Boolean(true)));
}
