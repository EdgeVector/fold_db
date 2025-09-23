use datafold::transform::{
    FieldValue, NativeFieldDefinition, NativeFieldDefinitionError, NativeFieldMapping,
    NativeFieldType, NativeFilterCondition, NativeFilterTransform, NativeMapTransform,
    NativeReduceTransform, NativeReducerType, NativeTransformSpec, NativeTransformSpecError,
    NativeTransformType,
};
use std::collections::HashMap;

fn optional_object_field(name: &str) -> NativeFieldDefinition {
    NativeFieldDefinition::new(
        name,
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false)
}

#[test]
fn map_transform_spec_validates_successfully() {
    let inputs = vec![
        NativeFieldDefinition::new("name", NativeFieldType::String),
        NativeFieldDefinition::new("age", NativeFieldType::Integer),
    ];

    let output = NativeFieldDefinition::new(
        "profile",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let mut mappings = HashMap::new();
    mappings.insert(
        "full_name".to_string(),
        NativeFieldMapping::Direct {
            field: "name".to_string(),
        },
    );
    mappings.insert(
        "age_next_year".to_string(),
        NativeFieldMapping::Function {
            name: "increment".to_string(),
            arguments: vec!["age".to_string()],
        },
    );

    let spec = NativeTransformSpec::new(
        "profile_builder",
        inputs,
        output,
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    spec.validate().expect("map transform should validate");
}

#[test]
fn map_transform_spec_rejects_unknown_field_reference() {
    let inputs = vec![NativeFieldDefinition::new("name", NativeFieldType::String)];
    let output = NativeFieldDefinition::new(
        "profile",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let mut mappings = HashMap::new();
    mappings.insert(
        "full_name".to_string(),
        NativeFieldMapping::Direct {
            field: "missing".to_string(),
        },
    );

    let spec = NativeTransformSpec::new(
        "profile_builder",
        inputs,
        output,
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("unknown field reference should fail validation");

    match error {
        NativeTransformSpecError::UnknownFieldReference { field } => {
            assert_eq!(field, "missing");
        }
        other => panic!("expected UnknownFieldReference error, got {other:?}"),
    }
}

#[test]
fn filter_transform_rejects_empty_condition_group() {
    let inputs = vec![NativeFieldDefinition::new("age", NativeFieldType::Integer)];
    let output = NativeFieldDefinition::new(
        "filtered",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let filter_transform = NativeFilterTransform {
        condition: NativeFilterCondition::And {
            conditions: Vec::new(),
        },
    };

    let spec = NativeTransformSpec::new(
        "age_filter",
        inputs,
        output,
        NativeTransformType::Filter(filter_transform),
    );

    let error = spec
        .validate()
        .expect_err("empty condition group should fail validation");

    match error {
        NativeTransformSpecError::EmptyConditionGroup => {}
        other => panic!("expected EmptyConditionGroup error, got {other:?}"),
    }
}

#[test]
fn reduce_transform_requires_known_source_field() {
    let inputs = vec![NativeFieldDefinition::new(
        "amount",
        NativeFieldType::Number,
    )];
    let output = NativeFieldDefinition::new(
        "totals",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let reducer = NativeReducerType::Sum {
        field: "unknown".to_string(),
    };

    let spec = NativeTransformSpec::new(
        "sum_values",
        inputs,
        output,
        NativeTransformType::Reduce(NativeReduceTransform::new(reducer, Vec::new())),
    );

    let error = spec
        .validate()
        .expect_err("unknown reducer field should fail validation");

    match error {
        NativeTransformSpecError::UnknownReducerField { reducer, field } => {
            assert_eq!(reducer, "sum");
            assert_eq!(field, "unknown");
        }
        other => panic!("expected UnknownReducerField error, got {other:?}"),
    }
}

#[test]
fn chain_transform_surfaces_nested_errors() {
    let nested_inputs = vec![NativeFieldDefinition::new(
        "value",
        NativeFieldType::Integer,
    )];
    let nested_output = NativeFieldDefinition::new(
        "result",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let mut nested_mappings = HashMap::new();
    nested_mappings.insert(
        "copied".to_string(),
        NativeFieldMapping::Direct {
            field: "missing".to_string(),
        },
    );

    let nested_spec = NativeTransformSpec::new(
        "inner",
        nested_inputs,
        nested_output.clone(),
        NativeTransformType::Map(NativeMapTransform::new(nested_mappings)),
    );

    let chain_spec = NativeTransformSpec::new(
        "chain",
        Vec::new(),
        nested_output,
        NativeTransformType::Chain(vec![nested_spec]),
    );

    let error = chain_spec
        .validate()
        .expect_err("invalid nested spec should surface through chain validation");

    match error {
        NativeTransformSpecError::InvalidNestedSpec { index, source } => {
            assert_eq!(index, 0);
            match source.as_ref() {
                NativeTransformSpecError::UnknownFieldReference { field } => {
                    assert_eq!(field, "missing");
                }
                other => panic!("unexpected nested error {other:?}"),
            }
        }
        other => panic!("expected InvalidNestedSpec error, got {other:?}"),
    }
}

#[test]
fn expression_mapping_rejects_empty_string() {
    let inputs = vec![NativeFieldDefinition::new("field", NativeFieldType::String)];
    let output = NativeFieldDefinition::new(
        "result",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

    let mut mappings = HashMap::new();
    mappings.insert(
        "output".to_string(),
        NativeFieldMapping::Expression {
            expression: "  ".to_string(),
        },
    );

    let spec = NativeTransformSpec::new(
        "expression",
        inputs,
        output,
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("empty expression should fail validation");

    match error {
        NativeTransformSpecError::EmptyExpressionMapping { field } => {
            assert_eq!(field, "output");
        }
        other => panic!("expected EmptyExpressionMapping error, got {other:?}"),
    }
}

#[test]
fn filter_condition_allows_known_field_references() {
    let inputs = vec![NativeFieldDefinition::new("age", NativeFieldType::Integer)];
    let output = optional_object_field("result");

    let filter_transform = NativeFilterTransform {
        condition: NativeFilterCondition::GreaterThan {
            field: "age".to_string(),
            value: FieldValue::Integer(18),
        },
    };

    let spec = NativeTransformSpec::new(
        "age_filter",
        inputs,
        output,
        NativeTransformType::Filter(filter_transform),
    );

    spec.validate().expect("filter condition should be valid");
}

#[test]
fn transform_spec_rejects_empty_name() {
    let mut mappings = HashMap::new();
    mappings.insert(
        "flag".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::Boolean(true),
        },
    );

    let spec = NativeTransformSpec::new(
        "   ",
        Vec::new(),
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("empty transform name should fail validation");

    match error {
        NativeTransformSpecError::EmptyName => {}
        other => panic!("expected EmptyName error, got {other:?}"),
    }
}

#[test]
fn transform_spec_rejects_duplicate_input_fields() {
    let inputs = vec![
        NativeFieldDefinition::new("dup", NativeFieldType::String),
        NativeFieldDefinition::new("dup", NativeFieldType::Integer),
    ];

    let mut mappings = HashMap::new();
    mappings.insert(
        "value".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::Null,
        },
    );

    let spec = NativeTransformSpec::new(
        "duplicates",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("duplicate inputs should fail validation");

    match error {
        NativeTransformSpecError::DuplicateInputField { field } => {
            assert_eq!(field, "dup");
        }
        other => panic!("expected DuplicateInputField error, got {other:?}"),
    }
}

#[test]
fn transform_spec_surfaces_input_validation_error() {
    let inputs = vec![NativeFieldDefinition::new("", NativeFieldType::String)];

    let mut mappings = HashMap::new();
    mappings.insert(
        "value".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::Null,
        },
    );

    let spec = NativeTransformSpec::new(
        "input_validation",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("invalid input field should surface as InputValidation error");

    match error {
        NativeTransformSpecError::InputValidation { field, source } => {
            assert!(field.is_empty());
            assert_eq!(source, NativeFieldDefinitionError::EmptyName);
        }
        other => panic!("expected InputValidation error, got {other:?}"),
    }
}

#[test]
fn transform_spec_surfaces_output_validation_error() {
    let mut mappings = HashMap::new();
    mappings.insert(
        "value".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::Null,
        },
    );

    let invalid_output = NativeFieldDefinition::new("result", NativeFieldType::Integer)
        .with_default(FieldValue::String("oops".to_string()));

    let spec = NativeTransformSpec::new(
        "output_validation",
        Vec::new(),
        invalid_output,
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("invalid output definition should surface as OutputValidation error");

    match error {
        NativeTransformSpecError::OutputValidation { field, source } => {
            assert_eq!(field, "result");
            match source {
                NativeFieldDefinitionError::DefaultTypeMismatch { .. } => {}
                other => panic!("unexpected output validation error {other:?}"),
            }
        }
        other => panic!("expected OutputValidation error, got {other:?}"),
    }
}

#[test]
fn map_transform_requires_field_mappings() {
    let spec = NativeTransformSpec::new(
        "empty_map",
        Vec::new(),
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(HashMap::new())),
    );

    let error = spec
        .validate()
        .expect_err("map transform must reject empty field mappings");

    match error {
        NativeTransformSpecError::EmptyFieldMappings => {}
        other => panic!("expected EmptyFieldMappings error, got {other:?}"),
    }
}

#[test]
fn map_transform_rejects_invalid_output_field_name() {
    let mut mappings = HashMap::new();
    mappings.insert(
        "  ".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::Null,
        },
    );

    let spec = NativeTransformSpec::new(
        "invalid_output_name",
        Vec::new(),
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("map transform should reject blank output field names");

    match error {
        NativeTransformSpecError::InvalidOutputFieldName { field } => {
            assert_eq!(field, "  ");
        }
        other => panic!("expected InvalidOutputFieldName error, got {other:?}"),
    }
}

#[test]
fn map_transform_function_requires_name() {
    let inputs = vec![NativeFieldDefinition::new("age", NativeFieldType::Integer)];

    let mut mappings = HashMap::new();
    mappings.insert(
        "future_age".to_string(),
        NativeFieldMapping::Function {
            name: "  ".to_string(),
            arguments: vec!["age".to_string()],
        },
    );

    let spec = NativeTransformSpec::new(
        "function_without_name",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("function mapping should require a name");

    match error {
        NativeTransformSpecError::EmptyFunctionName { field } => {
            assert_eq!(field, "future_age");
        }
        other => panic!("expected EmptyFunctionName error, got {other:?}"),
    }
}

#[test]
fn map_transform_function_arguments_must_be_known() {
    let inputs = vec![NativeFieldDefinition::new("age", NativeFieldType::Integer)];

    let mut mappings = HashMap::new();
    mappings.insert(
        "future_age".to_string(),
        NativeFieldMapping::Function {
            name: "increment".to_string(),
            arguments: vec!["missing".to_string()],
        },
    );

    let spec = NativeTransformSpec::new(
        "function_unknown_argument",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Map(NativeMapTransform::new(mappings)),
    );

    let error = spec
        .validate()
        .expect_err("function mapping should reject unknown arguments");

    match error {
        NativeTransformSpecError::UnknownFunctionArgument { function, argument } => {
            assert_eq!(function, "increment");
            assert_eq!(argument, "missing");
        }
        other => panic!("expected UnknownFunctionArgument error, got {other:?}"),
    }
}

#[test]
fn filter_condition_rejects_unknown_field_reference() {
    let inputs = vec![NativeFieldDefinition::new("known", NativeFieldType::String)];
    let output = optional_object_field("result");

    let filter_transform = NativeFilterTransform {
        condition: NativeFilterCondition::Contains {
            field: "missing".to_string(),
            value: FieldValue::String("value".to_string()),
        },
    };

    let spec = NativeTransformSpec::new(
        "contains_filter",
        inputs,
        output,
        NativeTransformType::Filter(filter_transform),
    );

    let error = spec
        .validate()
        .expect_err("filter condition should reject unknown field references");

    match error {
        NativeTransformSpecError::UnknownFieldReference { field } => {
            assert_eq!(field, "missing");
        }
        other => panic!("expected UnknownFieldReference error, got {other:?}"),
    }
}

#[test]
fn reduce_transform_requires_known_group_by_fields() {
    let inputs = vec![NativeFieldDefinition::new("age", NativeFieldType::Integer)];

    let spec = NativeTransformSpec::new(
        "group_by_unknown",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Reduce(NativeReduceTransform::new(
            NativeReducerType::Count,
            vec!["missing".to_string()],
        )),
    );

    let error = spec
        .validate()
        .expect_err("group-by should reference known fields");

    match error {
        NativeTransformSpecError::UnknownGroupByField { field } => {
            assert_eq!(field, "missing");
        }
        other => panic!("expected UnknownGroupByField error, got {other:?}"),
    }
}

#[test]
fn reduce_transform_rejects_missing_reducer_field() {
    let inputs = vec![NativeFieldDefinition::new(
        "amount",
        NativeFieldType::Number,
    )];

    let spec = NativeTransformSpec::new(
        "missing_reducer_field",
        inputs,
        optional_object_field("result"),
        NativeTransformType::Reduce(NativeReduceTransform::new(
            NativeReducerType::Sum {
                field: "".to_string(),
            },
            Vec::new(),
        )),
    );

    let error = spec
        .validate()
        .expect_err("reducers requiring fields should reject empty names");

    match error {
        NativeTransformSpecError::ReducerMissingField => {}
        other => panic!("expected ReducerMissingField error, got {other:?}"),
    }
}

#[test]
fn chain_transform_requires_non_empty_sequence() {
    let spec = NativeTransformSpec::new(
        "empty_chain",
        Vec::new(),
        optional_object_field("result"),
        NativeTransformType::Chain(Vec::new()),
    );

    let error = spec
        .validate()
        .expect_err("chain transform should reject empty sequences");

    match error {
        NativeTransformSpecError::EmptyTransformChain => {}
        other => panic!("expected EmptyTransformChain error, got {other:?}"),
    }
}
