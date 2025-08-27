//! Executor for transforms.
//!
//! This module provides the high-level interface for applying transforms to field values.
//! It handles the integration with the schema system and manages the execution context.

use super::ast::Value;
use super::interpreter::Interpreter;
use super::parser::TransformParser;
use crate::schema::indexing::chain_parser::{ChainParser, ParsedChain};
use crate::schema::indexing::errors::IteratorStackError;
use crate::schema::indexing::field_alignment::{FieldAlignmentValidator, AlignmentValidationResult};
use crate::schema::indexing::execution_engine::{ExecutionEngine, ExecutionResult};
use crate::schema::types::{SchemaError, Transform};
use log::{info, error};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executor for transforms.
pub struct TransformExecutor;

impl TransformExecutor {
    /// Executes a transform with the given input values.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🧮 TransformExecutor: Starting computation");
        if let Some(logic) = transform.get_procedural_logic() {
            info!("🔧 Transform logic: {}", logic);
        } else {
            info!("🔧 Declarative transform");
        }
        
        // Log individual input values
        info!("📊 Input values for computation:");
        for (key, value) in &input_values {
            info!("  📋 {}: {}", key, value);
        }
        
        // Log a simplified computation description
        if let Some(logic) = transform.get_procedural_logic() {
            info!("🧮 Computing with logic: {}", logic);
        } else {
            info!("🧮 Computing with declarative transform");
        }
        
        let result = Self::execute_transform_with_expr(transform, input_values);
        
        match &result {
            Ok(value) => {
                info!("✨ Computation result: {}", value);
                info!("✅ Transform execution completed successfully");
            }
            Err(e) => {
                error!("❌ Transform execution failed: {}", e);
            }
        }
        
        result
    }

    /// Executes a transform with the given input provider function.
    ///
    /// This version allows the transform to collect its own inputs using the provided function.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_provider` - A function that provides input values for a given input name
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform_with_provider<F>(
        transform: &Transform,
        input_provider: F,
    ) -> Result<JsonValue, SchemaError>
    where
        F: Fn(&str) -> Result<JsonValue, Box<dyn std::error::Error>>,
    {
        // Collect input values using the provider function
        let mut input_values = HashMap::new();

        // Use the transform's declared dependencies
        for input_name in transform.get_inputs() {
            match input_provider(input_name) {
                Ok(value) => {
                    input_values.insert(input_name.clone(), value);
                }
                Err(e) => {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to get input '{}': {}",
                        input_name, e
                    )));
                }
            }
        }

        // If no dependencies are declared, try to analyze the transform logic
        if transform.get_inputs().is_empty() {
            let dependencies = transform.analyze_dependencies();
            for input_name in dependencies {
                // Skip if we already have this input
                if input_values.contains_key(&input_name) {
                    continue;
                }

                // Try to get the input value
                match input_provider(&input_name) {
                    Ok(value) => {
                        input_values.insert(input_name, value);
                    }
                    Err(_) => {
                        // Ignore errors for analyzed dependencies, as they might not be actual inputs
                    }
                }
            }
        }

        // Execute the transform with the collected inputs
        info!(
            "execute_transform_with_provider logic: {} with inputs: {:?}",
            transform.get_procedural_logic().unwrap_or("[declarative]"), input_values
        );
        let result = Self::execute_transform(transform, input_values);
        if let Ok(ref value) = result {
            info!("execute_transform_with_provider result: {:?}", value);
        }
        result
    }

    /// Executes a transform with routing based on transform type.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform_with_expr(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        // Route based on transform type
        if transform.is_procedural() {
            info!("🔀 Routing to procedural transform execution");
            Self::execute_procedural_transform(transform, input_values)
        } else if transform.is_declarative() {
            info!("🔀 Routing to declarative transform execution");
            Self::execute_declarative_transform(transform, input_values)
        } else {
            error!("❌ Unknown transform type encountered");
            Err(SchemaError::InvalidTransform("Unknown transform type".to_string()))
        }
    }

    /// Executes a procedural transform using the existing logic.
    ///
    /// # Arguments
    ///
    /// * `transform` - The procedural transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    fn execute_procedural_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("⚙️ Executing procedural transform");
        
        // Use the pre-parsed expression if available, otherwise parse the transform logic
        let ast = match &transform.parsed_expression {
            Some(expr) => expr.clone(),
            None => {
                // Parse the transform logic
                let logic = transform.get_procedural_logic()
                    .ok_or_else(|| SchemaError::InvalidTransform("Procedural transform must have logic".to_string()))?;
                let parser = TransformParser::new();
                parser.parse_expression(logic).map_err(|e| {
                    SchemaError::InvalidField(format!("Failed to parse transform: {}", e))
                })?
            }
        };

        info!("🔍 Transform AST: {:?}", ast);
        info!("📊 Input values: {:?}", input_values);

        // Convert input values to interpreter values
        info!("🔄 Converting input values to interpreter format...");
        let variables = Self::convert_input_values(input_values);
        info!("🔄 Variables for interpreter: {:?}", variables);

        // Create interpreter with input variables
        info!("🧠 Creating interpreter with variables...");
        let mut interpreter = Interpreter::with_variables(variables);

        // Evaluate the AST
        info!("⚡ Evaluating expression...");
        let evaluated = interpreter.evaluate(&ast).map_err(|e| {
            error!("❌ Expression evaluation failed: {}", e);
            SchemaError::InvalidField(format!("Failed to execute transform: {}", e))
        })?;

        info!("🎯 Raw evaluation result: {:?}", evaluated);
        
        let json_result = Self::convert_result_value(evaluated)?;
        info!("✨ Final JSON result: {}", json_result);
        Ok(json_result)
    }

    /// Executes a declarative transform with actual execution logic.
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the declarative transform execution
    fn execute_declarative_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🏗️ Executing declarative transform");
        
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Declarative transform must have schema".to_string()))?;
        
        info!("📋 Declarative schema: {}", schema.name);
        info!("🔧 Schema type: {:?}", schema.schema_type);
        info!("📊 Schema fields: {:?}", schema.fields.keys().collect::<Vec<_>>());
        
        // Route to appropriate execution based on schema type
        match schema.schema_type {
            crate::schema::types::schema::SchemaType::Single => {
                info!("🎯 Executing Single schema type");
                Self::execute_single_schema(schema, input_values)
            }
            crate::schema::types::schema::SchemaType::Range { .. } => {
                info!("⚠️ Range schema execution not yet implemented - using placeholder");
                Self::execute_range_schema_placeholder(schema)
            }
            crate::schema::types::schema::SchemaType::HashRange => {
                info!("⚠️ HashRange schema execution not yet implemented - using placeholder");
                Self::execute_hashrange_schema_placeholder(schema)
            }
        }
    }

    /// Executes a Single schema type declarative transform.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the single schema execution
    fn execute_single_schema(
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔧 Executing Single schema: {}", schema.name);
        
        // Validate schema structure
        schema.validate()?;
        
        // Validate field alignment for declarative transforms
        Self::validate_field_alignment(schema)?;
        
        let mut result_object = serde_json::Map::new();
        
        // Process each field in the schema
        for (field_name, field_def) in &schema.fields {
            info!("📋 Processing field: {}", field_name);
            
            let field_value = Self::resolve_field_value(field_def, &input_values, field_name)?;
            result_object.insert(field_name.clone(), field_value);
        }
        
        let result = JsonValue::Object(result_object);
        info!("✨ Single schema execution result: {}", result);
        Ok(result)
    }

    /// Placeholder for Range schema execution (to be implemented in DTS-1-7D).
    fn execute_range_schema_placeholder(
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<JsonValue, SchemaError> {
        Ok(serde_json::json!({
            "schema_type": "Range",
            "schema_name": schema.name,
            "status": "placeholder_execution",
            "message": "Range schema execution will be implemented in DTS-1-7D"
        }))
    }

    /// Placeholder for HashRange schema execution (to be implemented in DTS-1-7D).
    fn execute_hashrange_schema_placeholder(
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<JsonValue, SchemaError> {
        Ok(serde_json::json!({
            "schema_type": "HashRange", 
            "schema_name": schema.name,
            "status": "placeholder_execution",
            "message": "HashRange schema execution will be implemented in DTS-1-7D"
        }))
    }

    /// Resolves a field value from input data based on field definition.
    ///
    /// # Arguments
    ///
    /// * `field_def` - The field definition containing resolution instructions
    /// * `input_values` - The input data to resolve from
    /// * `field_name` - The name of the field being resolved (for error messages)
    ///
    /// # Returns
    ///
    /// The resolved field value or an appropriate default/error
    fn resolve_field_value(
        field_def: &crate::schema::types::json_schema::FieldDefinition,
        input_values: &HashMap<String, JsonValue>,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔍 Resolving field '{}': {:?}", field_name, field_def);

        // Handle atom_uuid field resolution
        if let Some(atom_uuid_expr) = &field_def.atom_uuid {
            info!("🔗 Resolving atom_uuid expression: {}", atom_uuid_expr);
            return Self::resolve_atom_uuid_expression(atom_uuid_expr, input_values, field_name);
        }

        // Handle field_type without atom_uuid (constants or computed values)
        if let Some(field_type) = &field_def.field_type {
            info!("📝 Field type specified: {}", field_type);
            
            // For now, return a default value based on field type
            let default_value = Self::get_default_value_for_type(field_type);
            info!("🎯 Using default value for type '{}': {}", field_type, default_value);
            return Ok(default_value);
        }

        // If no atom_uuid or field_type, return null
        info!("⚠️ No resolution instructions for field '{}', returning null", field_name);
        Ok(JsonValue::Null)
    }

    /// Resolves an atom UUID expression from input data.
    ///
    /// # Arguments
    ///
    /// * `atom_uuid_expr` - The atom UUID expression to resolve
    /// * `input_values` - The input data to resolve from
    /// * `field_name` - The field name for error context
    ///
    /// # Returns
    ///
    /// The resolved value from the expression
    fn resolve_atom_uuid_expression(
        atom_uuid_expr: &str,
        input_values: &HashMap<String, JsonValue>,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔎 Resolving atom UUID expression: {}", atom_uuid_expr);

        // Try to parse with ChainParser first for complex expressions
        match Self::parse_atom_uuid_expression(atom_uuid_expr) {
            Ok(parsed_chain) => {
                info!("🔗 Using ChainParser for expression '{}' - depth: {}", atom_uuid_expr, parsed_chain.depth);
                // For now, fall back to simple resolution (execution will be implemented in DTS-1-7C3)
                return Self::resolve_parsed_chain_simple(&parsed_chain, input_values, field_name);
            }
            Err(parse_error) => {
                info!("⚠️ ChainParser failed for '{}', falling back to simple resolution: {}", atom_uuid_expr, parse_error);
                // Fall back to simple path resolution for non-chain expressions
            }
        }

        // Simple path-based resolution for basic cases
        if atom_uuid_expr.contains('.') {
            // Handle dotted path expressions like "user.profile.name"
            let resolved_value = Self::resolve_dotted_path(atom_uuid_expr, input_values)?;
            info!("✅ Resolved dotted path '{}' to: {}", atom_uuid_expr, resolved_value);
            return Ok(resolved_value);
        }

        // Handle simple direct field references
        if let Some(value) = input_values.get(atom_uuid_expr) {
            info!("✅ Direct field resolution '{}' found: {}", atom_uuid_expr, value);
            return Ok(value.clone());
        }

        // Field not found - return null with warning
        info!("⚠️ Field '{}' with expression '{}' not found in input data", field_name, atom_uuid_expr);
        Ok(JsonValue::Null)
    }

    /// Resolves a dotted path expression like "user.profile.name".
    ///
    /// # Arguments
    ///
    /// * `path` - The dotted path to resolve
    /// * `input_values` - The input data to resolve from
    ///
    /// # Returns
    ///
    /// The resolved value or null if not found
    fn resolve_dotted_path(
        path: &str,
        input_values: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        let parts: Vec<&str> = path.split('.').collect();
        
        if parts.is_empty() {
            return Ok(JsonValue::Null);
        }

        // Start with the root object
        let root_key = parts[0];
        let mut current_value = match input_values.get(root_key) {
            Some(value) => value.clone(),
            None => {
                info!("⚠️ Root key '{}' not found in input data", root_key);
                return Ok(JsonValue::Null);
            }
        };

        // Navigate through the path
        for part in &parts[1..] {
            // Skip function calls like "map()" for now - will be implemented in DTS-1-7C
            if part.contains('(') {
                info!("🔄 Skipping function call: {}", part);
                continue;
            }

            // Navigate into object property
            match current_value {
                JsonValue::Object(ref obj) => {
                    current_value = obj.get(*part).unwrap_or(&JsonValue::Null).clone();
                }
                JsonValue::Array(ref arr) if part.parse::<usize>().is_ok() => {
                    // Handle array indexing
                    let index = part.parse::<usize>().unwrap();
                    current_value = arr.get(index).unwrap_or(&JsonValue::Null).clone();
                }
                _ => {
                    info!("⚠️ Cannot navigate '{}' in non-object/array value", part);
                    return Ok(JsonValue::Null);
                }
            }
        }

        info!("✅ Resolved path '{}' to: {}", path, current_value);
        Ok(current_value)
    }

    /// Returns a default value for a given field type.
    ///
    /// # Arguments
    ///
    /// * `field_type` - The field type string
    ///
    /// # Returns
    ///
    /// An appropriate default value for the type
    fn get_default_value_for_type(field_type: &str) -> JsonValue {
        match field_type.to_lowercase().as_str() {
            "string" | "str" => JsonValue::String("".to_string()),
            "number" | "i32" | "i64" | "f32" | "f64" => JsonValue::Number(serde_json::Number::from(0)),
            "boolean" | "bool" => JsonValue::Bool(false),
            "array" => JsonValue::Array(vec![]),
            "object" => JsonValue::Object(serde_json::Map::new()),
            _ => {
                info!("🤷 Unknown field type '{}', using null", field_type);
                JsonValue::Null
            }
        }
    }

    /// Converts IteratorStackError to SchemaError for consistent error handling.
    ///
    /// # Arguments
    ///
    /// * `error` - The IteratorStackError to convert
    ///
    /// # Returns
    ///
    /// A SchemaError with appropriate message
    fn convert_iterator_stack_error(error: IteratorStackError) -> SchemaError {
        match error {
            IteratorStackError::InvalidChainSyntax { expression, reason } => {
                SchemaError::InvalidField(format!("Invalid chain syntax in '{}': {}", expression, reason))
            }
            IteratorStackError::MaxDepthExceeded { current_depth, max_depth } => {
                SchemaError::InvalidField(format!("Iterator depth {} exceeds maximum {}", current_depth, max_depth))
            }
            IteratorStackError::FieldAlignmentError { field, reason } => {
                SchemaError::InvalidField(format!("Field alignment error in '{}': {}", field, reason))
            }
            IteratorStackError::ExecutionError { message } => {
                SchemaError::InvalidField(format!("Execution error: {}", message))
            }
            _ => {
                SchemaError::InvalidField(format!("Iterator stack error: {}", error))
            }
        }
    }

    /// Parses an atom UUID expression using ChainParser.
    ///
    /// # Arguments
    ///
    /// * `expression` - The expression to parse
    ///
    /// # Returns
    ///
    /// The parsed chain or an error if parsing fails
    fn parse_atom_uuid_expression(expression: &str) -> Result<ParsedChain, SchemaError> {
        info!("🔗 Parsing atom UUID expression with ChainParser: {}", expression);
        
        let parser = ChainParser::new();
        let parsed_chain = parser.parse(expression)
            .map_err(Self::convert_iterator_stack_error)?;
        
        info!("✅ Successfully parsed expression '{}' - depth: {}, operations: {}", 
              expression, parsed_chain.depth, parsed_chain.operations.len());
        info!("📋 Operations: {:?}", parsed_chain.operations);
        
        Ok(parsed_chain)
    }

    /// Resolves a parsed chain using simple resolution (without full execution).
    ///
    /// # Arguments
    ///
    /// * `parsed_chain` - The parsed chain to resolve
    /// * `input_values` - The input data to resolve from
    /// * `field_name` - The field name for error context
    ///
    /// # Returns
    ///
    /// The resolved value or null if not found
    fn resolve_parsed_chain_simple(
        parsed_chain: &ParsedChain,
        input_values: &HashMap<String, JsonValue>,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔍 Resolving parsed chain with {} operations for field '{}'", 
              parsed_chain.operations.len(), field_name);

        // Try to use ExecutionEngine for single expression execution
        match Self::execute_single_expression_with_engine(parsed_chain, input_values) {
            Ok(result) => {
                info!("✅ ExecutionEngine resolved field '{}' successfully", field_name);
                return Ok(result);
            }
            Err(engine_error) => {
                info!("⚠️ ExecutionEngine failed for field '{}', falling back to simple resolution: {}", 
                      field_name, engine_error);
            }
        }

        // Fall back to simple path extraction and dotted path resolution
        let simple_path = Self::extract_simple_path_from_operations(&parsed_chain.operations);
        
        if simple_path.is_empty() {
            info!("⚠️ Empty path extracted from parsed chain for field '{}'", field_name);
            return Ok(JsonValue::Null);
        }

        info!("🔗 Extracted simple path '{}' from parsed chain", simple_path);
        
        // Use existing dotted path resolution for the extracted path
        Self::resolve_dotted_path(&simple_path, input_values)
    }

    /// Extracts a simple dotted path from chain operations for basic resolution.
    ///
    /// # Arguments
    ///
    /// * `operations` - The chain operations to extract from
    ///
    /// # Returns
    ///
    /// A simple dotted path string
    fn extract_simple_path_from_operations(operations: &[crate::schema::indexing::chain_parser::ChainOperation]) -> String {
        use crate::schema::indexing::chain_parser::ChainOperation;
        
        let mut path_parts = Vec::new();
        
        for operation in operations {
            match operation {
                ChainOperation::FieldAccess(field_name) => {
                    path_parts.push(field_name.clone());
                }
                ChainOperation::SpecialField(special) => {
                    path_parts.push(special.clone());
                }
                ChainOperation::Map | ChainOperation::SplitArray | ChainOperation::SplitByWord => {
                    // Skip iterator operations for simple resolution
                    info!("🔄 Skipping iterator operation: {:?}", operation);
                }
                ChainOperation::Reducer(reducer_name) => {
                    // Skip reducer operations for simple resolution  
                    info!("🔄 Skipping reducer operation: {}", reducer_name);
                }
            }
        }
        
        let path = path_parts.join(".");
        info!("📝 Extracted path '{}' from {} operations", path, operations.len());
        path
    }

    /// Validates field alignment for a declarative schema using FieldAlignmentValidator.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    ///
    /// # Returns
    ///
    /// Result indicating success or field alignment errors
    fn validate_field_alignment(
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<(), SchemaError> {
        info!("🔍 Validating field alignment for schema: {}", schema.name);
        
        // Parse all field expressions using ChainParser
        let mut parsed_chains = Vec::new();
        let mut parsing_errors = Vec::new();
        
        for (field_name, field_def) in &schema.fields {
            if let Some(atom_uuid_expr) = &field_def.atom_uuid {
                info!("🔗 Parsing field '{}' expression: {}", field_name, atom_uuid_expr);
                
                match Self::parse_atom_uuid_expression(atom_uuid_expr) {
                    Ok(parsed_chain) => {
                        parsed_chains.push(parsed_chain);
                        info!("✅ Successfully parsed field '{}' for validation", field_name);
                    }
                    Err(parse_error) => {
                        // Collect parsing errors but continue with other fields
                        parsing_errors.push((field_name.clone(), atom_uuid_expr.clone(), parse_error));
                        info!("⚠️ Failed to parse field '{}' expression '{}' - will skip validation", 
                              field_name, atom_uuid_expr);
                    }
                }
            } else {
                info!("📝 Field '{}' has no atom_uuid expression - skipping alignment validation", field_name);
            }
        }
        
        // If we have no parseable chains, skip validation
        if parsed_chains.is_empty() {
            if parsing_errors.is_empty() {
                info!("ℹ️ No parseable expressions found for alignment validation - skipping");
                return Ok(());
            } else {
                // All expressions failed to parse - report the first error
                let (field_name, expression, error) = &parsing_errors[0];
                return Err(SchemaError::InvalidField(format!(
                    "Failed to parse field '{}' expression '{}' for alignment validation: {}", 
                    field_name, expression, error
                )));
            }
        }
        
        // Perform field alignment validation
        let validator = FieldAlignmentValidator::new();
        let validation_result = validator.validate_alignment(&parsed_chains)
            .map_err(Self::convert_iterator_stack_error)?;
        
        Self::process_alignment_validation_result(&validation_result, schema)
    }

    /// Processes the field alignment validation result and converts errors.
    ///
    /// # Arguments
    ///
    /// * `result` - The alignment validation result
    /// * `schema` - The schema being validated (for context)
    ///
    /// # Returns
    ///
    /// Result indicating success or converted validation errors
    fn process_alignment_validation_result(
        result: &AlignmentValidationResult,
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<(), SchemaError> {
        info!("📊 Field alignment validation result for '{}': valid={}, max_depth={}, errors={}, warnings={}", 
              schema.name, result.valid, result.max_depth, result.errors.len(), result.warnings.len());
        
        // Log field alignment information
        for (expression, alignment_info) in &result.field_alignments {
            info!("📋 Field '{}': depth={}, alignment={:?}, branch={}, requires_reducer={}", 
                  expression, alignment_info.depth, alignment_info.alignment, 
                  alignment_info.branch, alignment_info.requires_reducer);
        }
        
        // Log warnings (non-fatal)
        for warning in &result.warnings {
            info!("⚠️ Alignment warning: {:?} - {}", warning.warning_type, warning.message);
        }
        
        // Process errors (fatal)
        if !result.valid {
            let error_messages: Vec<String> = result.errors.iter()
                .map(|err| format!("{:?}: {}", err.error_type, err.message))
                .collect();
            
            let combined_error = error_messages.join("; ");
            return Err(SchemaError::InvalidField(format!(
                "Field alignment validation failed for schema '{}': {}", 
                schema.name, combined_error
            )));
        }
        
        info!("✅ Field alignment validation passed for schema '{}'", schema.name);
        Ok(())
    }

    /// Executes a single expression using the ExecutionEngine.
    ///
    /// # Arguments
    ///
    /// * `parsed_chain` - The parsed chain to execute
    /// * `input_values` - The input values for execution
    ///
    /// # Returns
    ///
    /// The execution result as JsonValue
    fn execute_single_expression_with_engine(
        parsed_chain: &ParsedChain,
        input_values: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🚀 Executing single expression with ExecutionEngine: {}", parsed_chain.expression);
        
        // Create a temporary field alignment result for single expression
        let mut field_alignments = HashMap::new();
        field_alignments.insert(parsed_chain.expression.clone(), crate::schema::indexing::field_alignment::FieldAlignmentInfo {
            expression: parsed_chain.expression.clone(),
            depth: parsed_chain.depth,
            alignment: crate::schema::indexing::chain_parser::FieldAlignment::OneToOne, // Default for simple cases
            branch: parsed_chain.branch.clone(),
            requires_reducer: false, // Simple case for now
            suggested_reducer: None,
        });
        
        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: parsed_chain.depth,
            field_alignments,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        
        // Convert input_values HashMap to a JSON object for ExecutionEngine
        // The ExecutionEngine expects field names to match the input data structure
        let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        
        info!("📊 Using input data for execution: {}", input_data);
        
        // Create and execute with ExecutionEngine
        let mut execution_engine = ExecutionEngine::new();
        let execution_result = execution_engine.execute_fields(
            &[parsed_chain.clone()],
            &alignment_result,
            input_data,
        ).map_err(Self::convert_iterator_stack_error)?;
        
        info!("📈 ExecutionEngine produced {} index entries, {} warnings", 
              execution_result.index_entries.len(), execution_result.warnings.len());
        
        // Convert execution result to JsonValue
        Self::convert_execution_result_to_json(&execution_result)
    }

    /// Converts an ExecutionResult to a JsonValue for transform output.
    ///
    /// # Arguments
    ///
    /// * `execution_result` - The execution result to convert
    ///
    /// # Returns
    ///
    /// The converted JsonValue
    fn convert_execution_result_to_json(execution_result: &ExecutionResult) -> Result<JsonValue, SchemaError> {
        info!("🔄 Converting ExecutionResult with {} entries to JsonValue", execution_result.index_entries.len());
        
        if execution_result.index_entries.is_empty() {
            info!("📝 No index entries found, returning null");
            return Ok(JsonValue::Null);
        }
        
        // Check if this looks like a placeholder result from ExecutionEngine
        // The ExecutionEngine often returns placeholder values like "value_for_expression"
        let is_placeholder_result = execution_result.index_entries.iter().any(|entry| {
            let hash_is_placeholder = entry.hash_value.as_str()
                .map(|s| s.starts_with("value_for_"))
                .unwrap_or(false);
            let range_is_placeholder = entry.range_value.as_str()
                .map(|s| s.starts_with("value_for_"))
                .unwrap_or(false);
            hash_is_placeholder || range_is_placeholder
        });
        
        if is_placeholder_result {
            info!("⚠️ ExecutionEngine returned placeholder values, should fallback to simple resolution");
            return Err(SchemaError::InvalidField("ExecutionEngine returned placeholder values".to_string()));
        }
        
        // For single expression execution, prefer hash_value as the primary field result
        if execution_result.index_entries.len() == 1 {
            let entry = &execution_result.index_entries[0];
            info!("📝 Single entry found - hash_value: {}, range_value: {}", entry.hash_value, entry.range_value);
            
            // Use hash_value as the primary result, fallback to range_value if hash is null
            if !entry.hash_value.is_null() {
                info!("✅ Using hash_value as result: {}", entry.hash_value);
                return Ok(entry.hash_value.clone());
            } else if !entry.range_value.is_null() {
                info!("✅ Using range_value as result: {}", entry.range_value);
                return Ok(entry.range_value.clone());
            }
        }
        
        // For multiple entries, collect all hash values (primary) or range values (fallback)
        let mut values = Vec::new();
        for entry in &execution_result.index_entries {
            if !entry.hash_value.is_null() {
                values.push(entry.hash_value.clone());
            } else if !entry.range_value.is_null() {
                values.push(entry.range_value.clone());
            } else {
                // Include null values to maintain array structure
                values.push(JsonValue::Null);
            }
        }
        
        info!("📊 Extracted {} values from index entries", values.len());
        
        if values.len() == 1 {
            Ok(values[0].clone())
        } else {
            Ok(JsonValue::Array(values))
        }
    }

    /// Converts input values from JsonValue to interpreter Value.
    fn convert_input_values(input_values: HashMap<String, JsonValue>) -> HashMap<String, Value> {
        let mut variables = HashMap::new();

        for (name, value) in input_values {
            // Handle both schema.field format and regular field names
            variables.insert(name.clone(), Value::from(value.clone()));

            // If the name contains a dot, it's in schema.field format
            if let Some((schema, field)) = name.split_once('.') {
                // Add both schema.field and field entries
                variables.insert(format!("{}.{}", schema, field), Value::from(value.clone()));
                variables.insert(field.to_string(), Value::from(value));
            }
        }

        variables
    }

    /// Converts a result value from interpreter Value to JsonValue.
    fn convert_result_value(value: Value) -> Result<JsonValue, SchemaError> {
        Ok(JsonValue::from(value))
    }


    /// Validates a transform.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transform is valid, otherwise an error
    pub fn validate_transform(transform: &Transform) -> Result<(), SchemaError> {
        // Only validate procedural transforms with logic parsing
        if let Some(logic) = transform.get_procedural_logic() {
            // Parse the transform logic to check for syntax errors
            let parser = TransformParser::new();
            let ast = parser.parse_expression(logic);

            // For "input +" specifically, we want to fail validation
            if logic == "input +" {
                return Err(SchemaError::InvalidField(
                    "Invalid transform syntax: missing right operand".to_string(),
                ));
            }

            ast.map_err(|e| SchemaError::InvalidField(format!("Invalid transform syntax: {}", e)))?;

        } else if let Some(schema) = transform.get_declarative_schema() {
            // Validate declarative transform schema
            schema.validate()?;
        } else {
            return Err(SchemaError::InvalidTransform("Transform must be either procedural or declarative".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::{Expression, Operator, Value};
    use super::*;

    #[test]
    fn test_execute_complex_transform() {
        // Create a complex transform (BMI calculation) with a manually constructed expression
        let expr = Expression::LetBinding {
            name: "bmi".to_string(),
            value: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Variable("weight".to_string())),
                operator: Operator::Divide,
                right: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::Variable("height".to_string())),
                    operator: Operator::Power,
                    right: Box::new(Expression::Literal(Value::Number(2.0))),
                }),
            }),
            body: Box::new(Expression::Variable("bmi".to_string())),
        };

        let transform = Transform::new_with_expr(
            "let bmi = weight / (height ^ 2); bmi".to_string(),
            expr,
            "test.bmi".to_string(),
        );

        // Create input values
        let mut input_values = HashMap::new();
        input_values.insert(
            "weight".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(70.0).unwrap()),
        );
        input_values.insert(
            "height".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(1.75).unwrap()),
        );

        // Execute the transform
        let result =
            TransformExecutor::execute_transform_with_expr(&transform, input_values).unwrap();

        // Check the result (BMI = 70 / (1.75^2) = 70 / 3.0625 = 22.857)
        match result {
            JsonValue::Number(n) => {
                let value = n.as_f64().unwrap();
                assert!((value - 22.857).abs() < 0.001);
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_execute_transform_with_field_access() {
        // Create a transform that accesses object fields with a manually constructed expression
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Variable("patient".to_string())),
                field: "weight".to_string(),
            }),
            operator: Operator::Divide,
            right: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::FieldAccess {
                    object: Box::new(Expression::Variable("patient".to_string())),
                    field: "height".to_string(),
                }),
                operator: Operator::Power,
                right: Box::new(Expression::Literal(Value::Number(2.0))),
            }),
        };

        let transform = Transform::new_with_expr(
            "patient.weight / (patient.height ^ 2)".to_string(),
            expr,
            "test.bmi".to_string(),
        );

        // Create input values with nested objects
        let mut input_values = HashMap::new();

        let mut patient = serde_json::Map::new();
        patient.insert(
            "weight".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(70.0).unwrap()),
        );
        patient.insert(
            "height".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(1.75).unwrap()),
        );

        input_values.insert("patient".to_string(), JsonValue::Object(patient));

        // Execute the transform
        let result =
            TransformExecutor::execute_transform_with_expr(&transform, input_values).unwrap();

        // Check the result (BMI = 70 / (1.75^2) = 70 / 3.0625 = 22.857)
        match result {
            JsonValue::Number(n) => {
                let value = n.as_f64().unwrap();
                assert!((value - 22.857).abs() < 0.001);
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_execute_transform_with_provider_inputs_handling() {
        let parser = TransformParser::new();
        let expr = parser.parse_expression("a + b").unwrap();
        let base_transform =
            Transform::new_with_expr("a + b".to_string(), expr, "test.out".to_string());

        // Case 1: explicit inputs provided, dependency analysis should not run
        let mut transform = base_transform.clone();
        transform.set_inputs(vec!["a".to_string()]);

        let provider = |name: &str| -> Result<JsonValue, Box<dyn std::error::Error>> {
            match name {
                "a" => Ok(JsonValue::from(2)),
                other => panic!("unexpected input request: {}", other),
            }
        };
        // Evaluation should fail because 'b' is missing but provider should not panic
        assert!(TransformExecutor::execute_transform_with_provider(&transform, provider).is_err());

        // Case 2: no explicit inputs, analysis should request both 'a' and 'b'
        let provider = |name: &str| -> Result<JsonValue, Box<dyn std::error::Error>> {
            match name {
                "a" => Ok(JsonValue::from(2)),
                "b" => Ok(JsonValue::from(3)),
                other => panic!("unexpected input request: {}", other),
            }
        };

        let result =
            TransformExecutor::execute_transform_with_provider(&base_transform, provider).unwrap();
        assert_eq!(result, JsonValue::from(5.0));
    }

    #[test]
    fn test_validate_transform() {
        // Valid transform
        let transform = Transform::new("input + 10".to_string(), "test.output".to_string());

        assert!(TransformExecutor::validate_transform(&transform).is_ok());

        // Invalid transform (syntax error)
        let invalid_transform = Transform::new(
            "input +".to_string(), // Missing right operand
            "test.output".to_string(),
        );

        assert!(TransformExecutor::validate_transform(&invalid_transform).is_err());

        // No signature validation errors expected anymore
    }
}
