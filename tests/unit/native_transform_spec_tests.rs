use datafold::transform::{
    FieldValue, NativeFieldDefinition, NativeFieldMapping, NativeFieldType, NativeFilterCondition,
    NativeFilterTransform, NativeMapTransform, NativeReduceTransform, NativeReducerType,
    NativeTransformSpec, NativeTransformSpecError, NativeTransformType,
};
use std::collections::HashMap;

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
    let output = NativeFieldDefinition::new(
        "result",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    )
    .with_required(false);

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
