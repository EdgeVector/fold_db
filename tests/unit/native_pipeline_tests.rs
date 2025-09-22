use datafold::transform::{
    FieldValue, NativeDataPipeline, NativeFieldType, NativeTransformExecutor, PipelineError,
    ProcessingContext,
};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone)]
struct TestTransformSpec {
    id: &'static str,
}

impl TestTransformSpec {
    fn new(id: &'static str) -> Self {
        Self { id }
    }
}

#[derive(Debug, Error)]
#[error("transform {id} failed")]
struct MockError {
    id: &'static str,
}

#[derive(Debug)]
struct MockExecutor {
    responses: HashMap<String, FieldValue>,
    failures: HashSet<String>,
}

impl MockExecutor {
    fn new(responses: HashMap<String, FieldValue>, failures: HashSet<String>) -> Self {
        Self {
            responses,
            failures,
        }
    }
}

impl NativeTransformExecutor<TestTransformSpec> for MockExecutor {
    type Error = MockError;

    fn execute_transform(
        &self,
        transform_spec: &TestTransformSpec,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, Self::Error> {
        if self.failures.contains(transform_spec.id) {
            return Err(MockError {
                id: transform_spec.id,
            });
        }

        if let Some(value) = self.responses.get(transform_spec.id) {
            return Ok(value.clone());
        }

        Ok(FieldValue::Object(input_data.clone()))
    }
}

fn empty_executor() -> Arc<MockExecutor> {
    Arc::new(MockExecutor::new(HashMap::new(), HashSet::new()))
}

#[test]
fn pipeline_processes_transform_chain_successfully() {
    let mut stage_one_map = HashMap::new();
    stage_one_map.insert("intermediate".to_string(), FieldValue::Integer(1));

    let mut stage_two_map = HashMap::new();
    stage_two_map.insert("final".to_string(), FieldValue::Integer(2));

    let executor = Arc::new(MockExecutor::new(
        HashMap::from([
            (
                "stage-1".to_string(),
                FieldValue::Object(stage_one_map.clone()),
            ),
            (
                "stage-2".to_string(),
                FieldValue::Object(stage_two_map.clone()),
            ),
        ]),
        HashSet::new(),
    ));

    let pipeline = NativeDataPipeline::new(executor, Arc::new(()));

    let context = ProcessingContext::new(
        "test-schema",
        HashMap::from([("initial".to_string(), FieldValue::Integer(0))]),
        vec![
            TestTransformSpec::new("stage-1"),
            TestTransformSpec::new("stage-2"),
        ],
    );

    let result = pipeline
        .process_data(context)
        .expect("pipeline should succeed");

    assert_eq!(result, stage_two_map);
}

#[test]
fn pipeline_allows_empty_transform_chain() {
    let pipeline = NativeDataPipeline::new(empty_executor(), Arc::new(()));

    let initial_data = HashMap::from([("count".to_string(), FieldValue::Integer(42))]);

    let context = ProcessingContext::new("schema", initial_data.clone(), Vec::new());

    let result = pipeline
        .process_data(context)
        .expect("empty chain should return initial data");

    assert_eq!(result, initial_data);
}

#[test]
fn pipeline_returns_error_when_transform_output_is_not_object() {
    let executor = Arc::new(MockExecutor::new(
        HashMap::from([("stage-1".to_string(), FieldValue::Number(PI))]),
        HashSet::new(),
    ));

    let pipeline = NativeDataPipeline::new(executor, Arc::new(()));

    let context = ProcessingContext::new(
        "schema",
        HashMap::new(),
        vec![TestTransformSpec::new("stage-1")],
    );

    let error = pipeline
        .process_data(context)
        .expect_err("non-object output should surface as error");

    match error {
        PipelineError::NonObjectOutput {
            stage_index,
            actual_type,
        } => {
            assert_eq!(stage_index, 0);
            assert_eq!(actual_type, NativeFieldType::Number);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn pipeline_propagates_transform_errors() {
    let executor = Arc::new(MockExecutor::new(
        HashMap::new(),
        HashSet::from(["stage-2".to_string()]),
    ));

    let pipeline = NativeDataPipeline::new(executor, Arc::new(()));

    let context = ProcessingContext::new(
        "schema",
        HashMap::new(),
        vec![
            TestTransformSpec::new("stage-1"),
            TestTransformSpec::new("stage-2"),
        ],
    );

    let error = pipeline
        .process_data(context)
        .expect_err("executor failure should bubble up");

    match error {
        PipelineError::Transform {
            stage_index,
            source,
        } => {
            assert_eq!(stage_index, 1);
            assert_eq!(source.id, "stage-2");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn process_single_transform_invokes_executor_directly() {
    let executor = Arc::new(MockExecutor::new(
        HashMap::from([("stage-1".to_string(), FieldValue::Boolean(true))]),
        HashSet::new(),
    ));

    let pipeline = NativeDataPipeline::new(executor, Arc::new(()));

    let input_data = HashMap::from([("flag".to_string(), FieldValue::Boolean(false))]);

    let result = pipeline
        .process_single_transform(&TestTransformSpec::new("stage-1"), &input_data)
        .expect("executor should return mocked value");

    assert_eq!(result, FieldValue::Boolean(true));
}
