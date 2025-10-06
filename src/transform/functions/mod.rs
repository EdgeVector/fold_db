pub mod registry;

#[cfg(test)]
mod comprehensive_tests;

#[cfg(test)]
mod integration_tests;

pub use registry::{
    registry, FunctionMetadata, FunctionRegistry, FunctionType,
    IteratorFunction, IteratorExecutionResult, ReducerFunction,
};

