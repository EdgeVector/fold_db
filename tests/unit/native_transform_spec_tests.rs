use datafold::transform::{
    FieldValue, NativeFieldDefinition, NativeFieldDefinitionError, NativeFieldMapping,
    NativeFieldType, NativeFilterCondition, NativeFilterTransform, NativeMapTransform,
    NativeReduceTransform, NativeReducer, NativeTransformSpec, NativeTransformSpecError,
    NativeTransformType,
};
use std::collections::HashMap;

#[test]
fn transform_spec_rejects_empty_name() {
    let spec = map_spec(
        "",
        vec![NativeFieldDefinition::new("field", NativeFieldType::String)],
        object_output("payload", vec![("field", NativeFieldType::String)]),
        vec![(
            "field",
            NativeFieldMapping::Direct {
                field: "field".to_string(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("empty transform name should be rejected");
    assert_eq!(error, NativeTransformSpecError::EmptyName);
}

#[test]
fn transform_spec_rejects_duplicate_inputs() {
    let spec = map_spec(
        "duplicate_inputs",
        vec![
            NativeFieldDefinition::new("value", NativeFieldType::Integer),
            NativeFieldDefinition::new("value", NativeFieldType::Integer),
        ],
        object_output("payload", vec![("result", NativeFieldType::Integer)]),
        vec![(
            "result",
            NativeFieldMapping::Direct {
                field: "value".to_string(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("duplicate input names should be rejected");
    assert_eq!(
        error,
        NativeTransformSpecError::DuplicateInput {
            name: "value".to_string(),
        },
    );
}

#[test]
fn transform_spec_rejects_invalid_input_definition() {
    let spec = map_spec(
        "invalid_input",
        vec![NativeFieldDefinition::new("", NativeFieldType::Integer)],
        object_output("payload", vec![("result", NativeFieldType::Integer)]),
        vec![(
            "result",
            NativeFieldMapping::Constant {
                value: FieldValue::Integer(1),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("invalid input definition should surface its error");
    assert_eq!(
        error,
        NativeTransformSpecError::InvalidInputDefinition {
            name: String::new(),
            source: NativeFieldDefinitionError::EmptyName,
        },
    );
}

#[test]
fn transform_spec_rejects_invalid_output_definition() {
    let spec = filter_spec(
        "invalid_output",
        vec![NativeFieldDefinition::new("flag", NativeFieldType::Boolean)],
        NativeFieldDefinition::new("bad-name", NativeFieldType::Boolean),
        NativeFilterCondition::Equals {
            field: "flag".to_string(),
            value: FieldValue::Boolean(true),
        },
    );

    let error = spec
        .validate()
        .expect_err("invalid output definition should surface its error");
    assert_eq!(
        error,
        NativeTransformSpecError::InvalidOutputDefinition(
            NativeFieldDefinitionError::InvalidNameCharacters {
                name: "bad-name".to_string(),
            },
        ),
    );
}

#[test]
fn map_transform_validation_accepts_well_formed_configuration() {
    let inputs = vec![
        NativeFieldDefinition::new("name", NativeFieldType::String),
        NativeFieldDefinition::new("age", NativeFieldType::Integer),
    ];

    let output = NativeFieldDefinition::new(
        "person",
        NativeFieldType::Object {
            fields: HashMap::from([
                ("full_name".to_string(), NativeFieldType::String),
                ("years_old".to_string(), NativeFieldType::Integer),
            ]),
        },
    );

    let mut field_mappings = HashMap::new();
    field_mappings.insert(
        "full_name".to_string(),
        NativeFieldMapping::Direct {
            field: "name".to_string(),
        },
    );
    field_mappings.insert(
        "years_old".to_string(),
        NativeFieldMapping::Direct {
            field: "age".to_string(),
        },
    );

    let spec = NativeTransformSpec {
        name: "profile_map".to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Map(NativeMapTransform { field_mappings }),
    };

    spec.validate()
        .expect("valid map transform specification should pass validation");
}

#[test]
fn map_transform_validation_rejects_unknown_input_reference() {
    let inputs = vec![NativeFieldDefinition::new("name", NativeFieldType::String)];

    let output = NativeFieldDefinition::new(
        "person",
        NativeFieldType::Object {
            fields: HashMap::from([("full_name".to_string(), NativeFieldType::String)]),
        },
    );

    let mut field_mappings = HashMap::new();
    field_mappings.insert(
        "full_name".to_string(),
        NativeFieldMapping::Direct {
            field: "missing".to_string(),
        },
    );

    let spec = NativeTransformSpec {
        name: "invalid_map".to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Map(NativeMapTransform { field_mappings }),
    };

    let error = spec
        .validate()
        .expect_err("missing input reference should fail validation");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownInputReference {
            target: "full_name".to_string(),
            referenced: "missing".to_string(),
        }
    );
}

#[test]
fn map_transform_rejects_constant_type_mismatch() {
    let inputs = Vec::new();

    let output = NativeFieldDefinition::new(
        "metrics",
        NativeFieldType::Object {
            fields: HashMap::from([("count".to_string(), NativeFieldType::Integer)]),
        },
    );

    let mut field_mappings = HashMap::new();
    field_mappings.insert(
        "count".to_string(),
        NativeFieldMapping::Constant {
            value: FieldValue::String("oops".to_string()),
        },
    );

    let spec = NativeTransformSpec {
        name: "bad_constant".to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Map(NativeMapTransform { field_mappings }),
    };

    let error = spec
        .validate()
        .expect_err("type mismatch should fail validation");
    assert_eq!(
        error,
        NativeTransformSpecError::ConstantTypeMismatch {
            target: "count".to_string(),
            expected: Box::new(NativeFieldType::Integer),
            actual: Box::new(NativeFieldType::String),
        }
    );
}

#[test]
fn map_transform_rejects_empty_mappings() {
    let spec = map_spec(
        "empty_mappings",
        vec![NativeFieldDefinition::new(
            "source",
            NativeFieldType::String,
        )],
        object_output("payload", vec![("result", NativeFieldType::String)]),
        Vec::new(),
    );

    let error = spec
        .validate()
        .expect_err("map transforms must contain at least one mapping");
    assert_eq!(error, NativeTransformSpecError::EmptyMapMappings);
}

#[test]
fn map_transform_requires_object_output() {
    let spec = map_spec(
        "non_object_output",
        vec![NativeFieldDefinition::new(
            "source",
            NativeFieldType::String,
        )],
        NativeFieldDefinition::new("scalar", NativeFieldType::String),
        vec![(
            "scalar",
            NativeFieldMapping::Direct {
                field: "source".to_string(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("map outputs must be declared as objects");
    assert_eq!(
        error,
        NativeTransformSpecError::MapOutputNotObject {
            actual: NativeFieldType::String,
        },
    );
}

#[test]
fn map_transform_rejects_unknown_output_field() {
    let spec = map_spec(
        "unknown_output_field",
        vec![NativeFieldDefinition::new(
            "source",
            NativeFieldType::String,
        )],
        object_output("payload", vec![("valid", NativeFieldType::String)]),
        vec![(
            "missing",
            NativeFieldMapping::Direct {
                field: "source".to_string(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("map transforms should reference declared output fields");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownMapOutputField {
            field: "missing".to_string(),
        },
    );
}

#[test]
fn map_transform_rejects_empty_expression() {
    let spec = map_spec(
        "empty_expression",
        vec![NativeFieldDefinition::new(
            "source",
            NativeFieldType::String,
        )],
        object_output("payload", vec![("computed", NativeFieldType::String)]),
        vec![(
            "computed",
            NativeFieldMapping::Expression {
                expression: "   ".to_string(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("expression mappings should not be empty");
    assert_eq!(
        error,
        NativeTransformSpecError::EmptyExpression {
            target: "computed".to_string(),
        },
    );
}

#[test]
fn map_transform_rejects_empty_function_name() {
    let spec = map_spec(
        "empty_function_name",
        vec![NativeFieldDefinition::new(
            "value",
            NativeFieldType::Integer,
        )],
        object_output("payload", vec![("result", NativeFieldType::Integer)]),
        vec![(
            "result",
            NativeFieldMapping::Function {
                name: "  ".to_string(),
                arguments: Vec::new(),
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("function mappings must provide a name");
    assert_eq!(
        error,
        NativeTransformSpecError::EmptyFunctionName {
            target: "result".to_string(),
        },
    );
}

#[test]
fn map_transform_rejects_unknown_function_argument() {
    let spec = map_spec(
        "unknown_function_argument",
        vec![NativeFieldDefinition::new(
            "value",
            NativeFieldType::Integer,
        )],
        object_output("payload", vec![("result", NativeFieldType::Integer)]),
        vec![(
            "result",
            NativeFieldMapping::Function {
                name: "sum".to_string(),
                arguments: vec!["missing".to_string()],
            },
        )],
    );

    let error = spec
        .validate()
        .expect_err("function arguments should reference known inputs");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownFunctionArgument {
            function: "sum".to_string(),
            argument: "missing".to_string(),
        },
    );
}

#[test]
fn filter_transform_rejects_unknown_field() {
    let inputs = vec![NativeFieldDefinition::new("name", NativeFieldType::String)];

    let output = NativeFieldDefinition::new(
        "filtered",
        NativeFieldType::Object {
            fields: HashMap::new(),
        },
    );

    let condition = NativeFilterCondition::Equals {
        field: "missing".to_string(),
        value: FieldValue::String("Ada".to_string()),
    };

    let spec = NativeTransformSpec {
        name: "invalid_filter".to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Filter(NativeFilterTransform { condition }),
    };

    let error = spec
        .validate()
        .expect_err("filter referencing unknown field should fail");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownFilterField {
            field: "missing".to_string(),
        }
    );
}

#[test]
fn filter_transform_rejects_empty_condition_group() {
    let spec = filter_spec(
        "empty_condition_group",
        vec![NativeFieldDefinition::new("flag", NativeFieldType::Boolean)],
        object_output("filtered", Vec::new()),
        NativeFilterCondition::And {
            conditions: Vec::new(),
        },
    );

    let error = spec
        .validate()
        .expect_err("logical groups must include at least one condition");
    assert_eq!(
        error,
        NativeTransformSpecError::EmptyConditionGroup { operator: "and" },
    );
}

#[test]
fn reduce_transform_detects_duplicate_group_by_fields() {
    let inputs = vec![
        NativeFieldDefinition::new("user_id", NativeFieldType::String),
        NativeFieldDefinition::new("amount", NativeFieldType::Number),
    ];

    let output = NativeFieldDefinition::new(
        "aggregated",
        NativeFieldType::Object {
            fields: HashMap::from([("total".to_string(), NativeFieldType::Number)]),
        },
    );

    let transform = NativeReduceTransform {
        reducer: NativeReducer::Sum {
            field: "amount".to_string(),
        },
        group_by: vec!["user_id".to_string(), "user_id".to_string()],
    };

    let spec = NativeTransformSpec {
        name: "duplicate_group".to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Reduce(transform),
    };

    let error = spec
        .validate()
        .expect_err("duplicate group by field should fail");
    assert_eq!(
        error,
        NativeTransformSpecError::DuplicateGroupByField {
            field: "user_id".to_string(),
        }
    );
}

#[test]
fn reduce_transform_rejects_unknown_reducer_field() {
    let spec = reduce_spec(
        "unknown_reducer_field",
        vec![NativeFieldDefinition::new(
            "user_id",
            NativeFieldType::String,
        )],
        object_output("aggregated", vec![("total", NativeFieldType::Number)]),
        NativeReducer::Sum {
            field: "amount".to_string(),
        },
        Vec::<&str>::new(),
    );

    let error = spec
        .validate()
        .expect_err("reduce reducer fields must reference known inputs");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownReducerField {
            field: "amount".to_string(),
        },
    );
}

#[test]
fn reduce_transform_rejects_unknown_group_by_field() {
    let spec = reduce_spec(
        "unknown_group_by",
        vec![
            NativeFieldDefinition::new("user_id", NativeFieldType::String),
            NativeFieldDefinition::new("amount", NativeFieldType::Number),
        ],
        object_output("aggregated", vec![("total", NativeFieldType::Number)]),
        NativeReducer::Sum {
            field: "amount".to_string(),
        },
        vec!["unknown"],
    );

    let error = spec
        .validate()
        .expect_err("group by fields must reference known inputs");
    assert_eq!(
        error,
        NativeTransformSpecError::UnknownGroupByField {
            field: "unknown".to_string(),
        },
    );
}

#[test]
fn chain_transform_propagates_nested_errors() {
    let nested_inputs = vec![NativeFieldDefinition::new(
        "value",
        NativeFieldType::Integer,
    )];
    let nested_output = NativeFieldDefinition::new(
        "nested",
        NativeFieldType::Object {
            fields: HashMap::from([("count".to_string(), NativeFieldType::Integer)]),
        },
    );

    let mut nested_mappings = HashMap::new();
    nested_mappings.insert(
        "count".to_string(),
        NativeFieldMapping::Direct {
            field: "missing".to_string(),
        },
    );

    let nested_spec = NativeTransformSpec {
        name: "inner".to_string(),
        inputs: nested_inputs,
        output: nested_output,
        transform_type: NativeTransformType::Map(NativeMapTransform {
            field_mappings: nested_mappings,
        }),
    };

    let outer_spec = NativeTransformSpec {
        name: "outer".to_string(),
        inputs: Vec::new(),
        output: NativeFieldDefinition::new(
            "outer_output",
            NativeFieldType::Object {
                fields: HashMap::new(),
            },
        ),
        transform_type: NativeTransformType::Chain(vec![nested_spec]),
    };

    let error = outer_spec
        .validate()
        .expect_err("nested validation error should bubble up");

    match error {
        NativeTransformSpecError::NestedSpecInvalid { name, source } => {
            assert_eq!(name, "inner");
            assert_eq!(
                *source,
                NativeTransformSpecError::UnknownInputReference {
                    target: "count".to_string(),
                    referenced: "missing".to_string(),
                }
            );
        }
        other => panic!("expected nested spec error, got {other:?}"),
    }
}

#[test]
fn chain_transform_requires_non_empty() {
    let spec = NativeTransformSpec {
        name: "empty_chain".to_string(),
        inputs: Vec::new(),
        output: object_output("result", Vec::new()),
        transform_type: NativeTransformType::Chain(Vec::new()),
    };

    let error = spec
        .validate()
        .expect_err("chains must contain at least one nested transform");
    assert_eq!(error, NativeTransformSpecError::EmptyChain);
}

fn object_output(name: &str, fields: Vec<(&str, NativeFieldType)>) -> NativeFieldDefinition {
    NativeFieldDefinition::new(
        name,
        NativeFieldType::Object {
            fields: fields
                .into_iter()
                .map(|(field_name, field_type)| (field_name.to_string(), field_type))
                .collect(),
        },
    )
}

fn map_spec(
    name: &str,
    inputs: Vec<NativeFieldDefinition>,
    output: NativeFieldDefinition,
    mappings: Vec<(&str, NativeFieldMapping)>,
) -> NativeTransformSpec {
    NativeTransformSpec {
        name: name.to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Map(NativeMapTransform {
            field_mappings: mappings
                .into_iter()
                .map(|(field, mapping)| (field.to_string(), mapping))
                .collect(),
        }),
    }
}

fn filter_spec(
    name: &str,
    inputs: Vec<NativeFieldDefinition>,
    output: NativeFieldDefinition,
    condition: NativeFilterCondition,
) -> NativeTransformSpec {
    NativeTransformSpec {
        name: name.to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Filter(NativeFilterTransform { condition }),
    }
}

fn reduce_spec(
    name: &str,
    inputs: Vec<NativeFieldDefinition>,
    output: NativeFieldDefinition,
    reducer: NativeReducer,
    group_by: Vec<&str>,
) -> NativeTransformSpec {
    NativeTransformSpec {
        name: name.to_string(),
        inputs,
        output,
        transform_type: NativeTransformType::Reduce(NativeReduceTransform {
            reducer,
            group_by: group_by
                .into_iter()
                .map(|value| value.to_string())
                .collect(),
        }),
    }
}
