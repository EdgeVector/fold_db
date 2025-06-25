use std::collections::{HashMap, HashSet};

use datafold::fold_db_core::transform_manager::utils::TransformUtils;
use crate::test_utils::TestFixture;

#[tokio::test]
async fn test_read_mapping_returns_default() {
    let fixture = TestFixture::new().expect("fixture");
    let map: HashMap<String, HashSet<String>> =
        TransformUtils::read_mapping(&fixture.db_ops, "missing", "test").unwrap();
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_read_mapping_invalid_json() {
    let fixture = TestFixture::new().expect("fixture");
    fixture
        .db_ops
        .store_transform_mapping("bad", b"not json")
        .unwrap();
    let map: HashMap<String, HashSet<String>> =
        TransformUtils::read_mapping(&fixture.db_ops, "bad", "bad").unwrap();
    assert!(map.is_empty());
}

#[test]
fn test_insert_mapping_set() {
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();
    TransformUtils::insert_mapping_set(&mut map, "field", "t1");
    TransformUtils::insert_mapping_set(&mut map, "field", "t2");
    let set = map.get("field").expect("set");
    assert!(set.contains("t1") && set.contains("t2"));
}

#[test]
fn test_handle_error() {
    let err = TransformUtils::handle_error("context", "oops");
    match err {
        datafold::schema::types::SchemaError::InvalidData(msg) => {
            assert!(msg.contains("context") && msg.contains("oops"));
        }
        _ => panic!("wrong error"),
    }
}

