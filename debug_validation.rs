use std::collections::HashMap;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, KeyConfig};
use datafold::schema::types::schema::SchemaType;

fn main() {
    let invalid_hashrange = DeclarativeSchemaDefinition {
        name: "invalid".to_string(),
        schema_type: SchemaType::HashRange,
        key: None,
        fields: HashMap::new(),
    };
    
    let validation_result = invalid_hashrange.validate();
    println!("Validation result: {:?}", validation_result);
}
