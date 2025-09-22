use super::types::{FieldType, FieldValue};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Executor trait for running native transform specifications.
pub trait NativeTransformExecutor<S> {
    /// Error type emitted when execution fails.
    type Error;

    /// Execute a transform against the provided input data.
    fn execute_transform(
        &self,
        transform_spec: &S,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, Self::Error>;
}

/// Errors produced by the native data pipeline when orchestrating transform execution.
#[derive(Debug, Error)]
pub enum PipelineError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Underlying transform execution failed.
    #[error("transform execution failed at stage {stage_index}: {source}")]
    Transform {
        /// Index of the transform that failed.
        stage_index: usize,
        /// Root cause error emitted by the executor.
        #[source]
        source: E,
    },

    /// A transform in the chain returned a non-object value.
    #[error("transform at index {stage_index} returned non-object output ({actual_type:?})")]
    NonObjectOutput {
        /// Index of the transform that produced the invalid output.
        stage_index: usize,
        /// Type of the value returned by the transform.
        actual_type: FieldType,
    },
}

/// Context required to execute a transform chain.
#[derive(Debug, Clone)]
pub struct ProcessingContext<S> {
    schema_name: String,
    input_data: HashMap<String, FieldValue>,
    transform_specs: Vec<S>,
}

impl<S> ProcessingContext<S> {
    /// Create a new processing context.
    #[must_use]
    pub fn new(
        schema_name: impl Into<String>,
        input_data: HashMap<String, FieldValue>,
        transform_specs: Vec<S>,
    ) -> Self {
        Self {
            schema_name: schema_name.into(),
            input_data,
            transform_specs,
        }
    }

    /// Name of the schema associated with this processing run.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    /// Borrow the current input data.
    #[must_use]
    pub fn input_data(&self) -> &HashMap<String, FieldValue> {
        &self.input_data
    }

    /// Borrow the configured transform specifications.
    #[must_use]
    pub fn transform_specs(&self) -> &[S] {
        &self.transform_specs
    }

    /// Append an additional transform specification to the execution chain.
    pub fn push_transform(&mut self, transform_spec: S) {
        self.transform_specs.push(transform_spec);
    }

    /// Consume the context and return its constituent parts.
    #[must_use]
    pub fn into_parts(self) -> (String, HashMap<String, FieldValue>, Vec<S>) {
        (self.schema_name, self.input_data, self.transform_specs)
    }
}

/// Native data pipeline that orchestrates transform execution with native types.
#[derive(Debug)]
pub struct NativeDataPipeline<E, R> {
    engine: Arc<E>,
    schema_registry: Arc<R>,
}

impl<E, R> NativeDataPipeline<E, R> {
    /// Create a new pipeline with the provided execution engine and schema registry.
    #[must_use]
    pub fn new(engine: Arc<E>, schema_registry: Arc<R>) -> Self {
        Self {
            engine,
            schema_registry,
        }
    }

    /// Access the underlying execution engine.
    #[must_use]
    pub fn engine(&self) -> &Arc<E> {
        &self.engine
    }

    /// Access the schema registry reference.
    #[must_use]
    pub fn schema_registry(&self) -> &Arc<R> {
        &self.schema_registry
    }
}

impl<E, R> NativeDataPipeline<E, R> {
    /// Execute the configured transform chain using the provided context.
    pub fn process_data<S>(
        &self,
        context: ProcessingContext<S>,
    ) -> Result<HashMap<String, FieldValue>, PipelineError<E::Error>>
    where
        E: NativeTransformExecutor<S> + Send + Sync,
        E::Error: std::error::Error + Send + Sync + 'static,
    {
        let ProcessingContext {
            schema_name: _,
            input_data: mut current_data,
            transform_specs,
        } = context;

        for (index, transform_spec) in transform_specs.iter().enumerate() {
            let result = self
                .engine
                .execute_transform(transform_spec, &current_data)
                .map_err(|source| PipelineError::Transform {
                    stage_index: index,
                    source,
                })?;

            match result {
                FieldValue::Object(object) => {
                    current_data = object;
                }
                other => {
                    let actual_type = other.field_type();
                    return Err(PipelineError::NonObjectOutput {
                        stage_index: index,
                        actual_type,
                    });
                }
            }
        }

        Ok(current_data)
    }

    /// Execute a single transform outside of a full pipeline run.
    pub fn process_single_transform<S>(
        &self,
        transform_spec: &S,
        input_data: &HashMap<String, FieldValue>,
    ) -> Result<FieldValue, E::Error>
    where
        E: NativeTransformExecutor<S> + Send + Sync,
    {
        self.engine.execute_transform(transform_spec, input_data)
    }
}
