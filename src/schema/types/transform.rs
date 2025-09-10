use crate::transform::ast::TransformDeclaration;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a transformation that can be applied to field values.
///
/// Transforms define how data from source fields is processed to produce
/// a derived value. They are expressed in a domain-specific language (DSL)
/// that supports basic arithmetic, comparisons, conditionals, and a small
/// set of built-in functions.
///
/// # Features
///
/// * Declarative syntax for expressing transformations
/// * Support for basic arithmetic, comparisons, and conditionals
/// * Optional signature for verification and auditability
/// * Payment requirements for accessing transformed data
/// * Automatic input dependency tracking
///
/// # Example
///
/// ```
/// use datafold::schema::types::Transform;
///
/// let transform = Transform::new(
///     "let bmi = weight / (height ^ 2); return 0.5 * blood_pressure + 1.2 * bmi;".to_string(),
///     "health.risk_score".to_string(),
/// );
/// ```
/// Parameters for registering a transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRegistration {
    /// The ID of the transform
    pub transform_id: String,
    /// The transform itself
    pub transform: Transform,
    /// Input atom reference UUIDs
    pub input_molecules: Vec<String>,
    /// Names of input fields corresponding to the atom references
    #[serde(default)]
    pub input_names: Vec<String>,
    /// Fields that trigger the transform
    pub trigger_fields: Vec<String>,
    /// Output atom reference UUID
    pub output_molecule: String,
    /// Schema name
    pub schema_name: String,
    /// Field name
    pub field_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transform {
    /// Explicit input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,

    /// The transform kind (procedural or declarative)
    #[serde(flatten)]
    pub kind: crate::schema::types::json_schema::TransformKind,

    /// The parsed expression (not serialized, used for procedural transforms)
    #[serde(skip)]
    pub parsed_expression: Option<crate::transform::ast::Expression>,
}

// Custom deserialization to allow either a transform DSL string or a struct
impl<'de> serde::Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            // Legacy struct format with logic field (backward compatibility)
            LegacyStruct {
                inputs: Option<Vec<String>>,
                logic: String,
                output: String,
            },
            // New struct format with kind field
            NewStruct {
                inputs: Option<Vec<String>>,
                output: String,
                #[serde(flatten)]
                kind: crate::schema::types::json_schema::TransformKind,
            },
        }

        match Helper::deserialize(deserializer)? {
            Helper::Str(s) => {
                let parser = crate::transform::parser::TransformParser::new();
                let decl = parser.parse_transform(&s).map_err(|e| {
                    serde::de::Error::custom(format!("Failed to parse transform DSL: {}", e))
                })?;
                Ok(Self::from_declaration(decl))
            }
            Helper::LegacyStruct {
                inputs,
                logic,
                output,
            } => Ok(Self {
                inputs: inputs.unwrap_or_default(),
                output,
                kind: crate::schema::types::json_schema::TransformKind::Procedural { logic },
                parsed_expression: None,
            }),
            Helper::NewStruct {
                inputs,
                output,
                kind,
            } => Ok(Self {
                inputs: inputs.unwrap_or_default(),
                output,
                kind,
                parsed_expression: None,
            }),
        }
    }
}

impl Transform {
    /// Creates a new procedural `Transform` from raw logic and output field.
    #[must_use]
    pub fn new(logic: String, output: String) -> Self {
        Self {
            inputs: Vec::new(),
            output,
            kind: crate::schema::types::json_schema::TransformKind::Procedural { logic },
            parsed_expression: None,
        }
    }

    /// Creates a new procedural `Transform` with a pre-parsed expression.
    #[must_use]
    pub fn new_with_expr(
        logic: String,
        parsed_expression: crate::transform::ast::Expression,
        output: String,
    ) -> Self {
        Self {
            inputs: Vec::new(),
            output,
            kind: crate::schema::types::json_schema::TransformKind::Procedural { logic },
            parsed_expression: Some(parsed_expression),
        }
    }

    /// Creates a new declarative `Transform` from schema definition.
    #[must_use]
    pub fn from_declarative_schema(
        schema: crate::schema::types::json_schema::DeclarativeSchemaDefinition,
        inputs: Vec<String>,
        output: String,
    ) -> Self {
        Self {
            inputs,
            output,
            kind: crate::schema::types::json_schema::TransformKind::Declarative { schema },
            parsed_expression: None,
        }
    }

    /// Returns true if this is a declarative transform.
    pub fn is_declarative(&self) -> bool {
        matches!(self.kind, crate::schema::types::json_schema::TransformKind::Declarative { .. })
    }

    /// Returns true if this is a procedural transform.
    pub fn is_procedural(&self) -> bool {
        matches!(self.kind, crate::schema::types::json_schema::TransformKind::Procedural { .. })
    }

    /// Gets the declarative schema if this is a declarative transform.
    pub fn get_declarative_schema(&self) -> Option<&crate::schema::types::json_schema::DeclarativeSchemaDefinition> {
        if let crate::schema::types::json_schema::TransformKind::Declarative { schema } = &self.kind {
            Some(schema)
        } else {
            None
        }
    }

    /// Gets the procedural logic if this is a procedural transform.
    pub fn get_procedural_logic(&self) -> Option<&str> {
        if let crate::schema::types::json_schema::TransformKind::Procedural { logic } = &self.kind {
            Some(logic)
        } else {
            None
        }
    }

    /// Sets the explicit input fields for this transform.
    pub fn set_inputs(&mut self, inputs: Vec<String>) {
        self.inputs = inputs;
    }

    /// Gets the explicit input fields for this transform.
    pub fn get_inputs(&self) -> &[String] {
        &self.inputs
    }

    /// Sets the output field for this transform.
    pub fn set_output(&mut self, output: String) {
        self.output = output;
    }

    /// Gets the output field for this transform.
    pub fn get_output(&self) -> &str {
        &self.output
    }

    /// Analyzes the transform logic to extract variable names that might be input dependencies.
    ///
    /// This is a simple implementation that just looks for identifiers in the logic.
    /// A more sophisticated implementation would parse the logic and extract actual variable references.
    ///
    /// # Returns
    ///
    /// A set of potential input dependencies
    pub fn analyze_dependencies(&self) -> HashSet<String> {
        let mut dependencies = HashSet::new();

        // For procedural transforms, analyze the logic
        if let Some(logic) = self.get_procedural_logic() {
            // Split by dots to handle schema.field format
            for part in logic.split(|c: char| !c.is_alphanumeric() && c != '.') {
                if part.is_empty() || part.chars().next().unwrap().is_numeric() {
                    continue;
                }

                // Skip keywords and operators
                match part {
                    "let" | "if" | "else" | "return" | "true" | "false" | "null" => continue,
                    _ => {}
                }

                // If it contains a dot, it's a schema.field reference
                if part.contains('.') {
                    let parts: Vec<&str> = part.split('.').collect();
                    if parts.len() == 2 {
                        // Add the full schema.field reference
                        dependencies.insert(format!("{}.{}", parts[0], parts[1]));
                    }
                } else {
                    // For backward compatibility, add the whole part if it's not a schema.field
                    dependencies.insert(part.to_string());
                }
            }
        } else if let Some(schema) = self.get_declarative_schema() {
            // For declarative transforms, analyze field expressions
            for field_def in schema.fields.values() {
                if let Some(atom_uuid) = &field_def.atom_uuid {
                    // Extract schema names from expressions like "BlogPost.map().content"
                    // Take the first part before the first dot
                    if let Some(first_dot) = atom_uuid.find('.') {
                        let schema_name = &atom_uuid[..first_dot];
                        if !schema_name.is_empty() {
                            dependencies.insert(schema_name.to_string());
                        }
                    }
                }
            }
            
            // Also add explicitly declared inputs
            for input in &self.inputs {
                dependencies.insert(input.clone());
            }
        }

        dependencies
    }
    /// Creates a new Transform from a TransformDeclaration.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The transform declaration
    ///
    /// # Returns
    ///
    /// A new Transform instance
    #[must_use]
    pub fn from_declaration(declaration: TransformDeclaration) -> Self {
        // Extract logic from the declaration
        let logic = declaration
            .logic
            .iter()
            .map(|expr| format!("{}", expr))
            .collect::<Vec<_>>()
            .join("\n");

        // Placeholder output until attached to a field
        let output = format!("test.{}", declaration.name);

        Self {
            inputs: Vec::new(),
            output,
            kind: crate::schema::types::json_schema::TransformKind::Procedural { logic },
            parsed_expression: None,
        }
    }

    /// Validates the transform using existing infrastructure.
    /// 
    /// This performs comprehensive validation including iterator stack validation
    /// for declarative transforms and DSL syntax validation for procedural transforms.
    /// It also enforces that transforms cannot directly create atoms or molecules.
    pub fn validate(&self) -> Result<(), crate::schema::types::SchemaError> {
        use log::info;

        info!("🔍 Validating transform: {:?}", self.kind);

        // Validate transform kind
        self.kind.validate()?;

        // Validate inputs and output
        if self.output.trim().is_empty() {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "Transform output cannot be empty".to_string()
            ));
        }

        // **CRITICAL**: Validate that transform doesn't attempt direct atom/molecule creation
        self.validate_no_direct_creation()?;

        // Additional validation based on transform type
        match &self.kind {
            crate::schema::types::json_schema::TransformKind::Procedural { logic: _ } => {
                self.validate_procedural_transform()?;
            }
            crate::schema::types::json_schema::TransformKind::Declarative { schema } => {
                self.validate_declarative_transform(schema)?;
            }
        }

        info!("✅ Transform validation completed successfully");
        Ok(())
    }

    /// Validates procedural transform specific requirements
    fn validate_procedural_transform(&self) -> Result<(), crate::schema::types::SchemaError> {
        // Procedural validation is already handled by TransformKind::validate()
        // Additional procedural-specific validation could go here
        Ok(())
    }

    /// **CRITICAL**: Validates that transform doesn't attempt direct atom/molecule creation.
    /// 
    /// This ensures all data persistence goes through the mutation system.
    fn validate_no_direct_creation(&self) -> Result<(), crate::schema::types::SchemaError> {
        use crate::transform::restricted_access::TransformAccessValidator;
        
        // Get the transform code to validate
        let transform_code = match &self.kind {
            crate::schema::types::json_schema::TransformKind::Procedural { logic } => logic.clone(),
            crate::schema::types::json_schema::TransformKind::Declarative { schema } => {
                // For declarative transforms, check field expressions
                let mut code_parts = Vec::new();
                for field_def in schema.fields.values() {
                    if let Some(atom_uuid) = &field_def.atom_uuid {
                        code_parts.push(atom_uuid.clone());
                    }
                }
                code_parts.join(" ")
            }
        };
        
        // Validate no direct creation patterns
        TransformAccessValidator::validate_no_direct_creation(&transform_code)?;
        
        // Validate proper mutation usage
        TransformAccessValidator::validate_mutation_usage(&transform_code)?;
        
        Ok(())
    }

    /// Validates declarative transform using iterator stack infrastructure
    fn validate_declarative_transform(
        &self, 
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition
    ) -> Result<(), crate::schema::types::SchemaError> {
        use crate::transform::iterator_stack::chain_parser::ChainParser;
        use crate::transform::iterator_stack::field_alignment::FieldAlignmentValidator;
        use log::info;

        info!("🔍 Validating declarative transform with iterator stack infrastructure");

        // Validate that inputs are appropriate for declarative transforms
        // Note: For declarative transforms, inputs represent data sources, not schema fields
        // This validation focuses on ensuring inputs are reasonable
        for input in &self.inputs {
            if input.trim().is_empty() {
                return Err(crate::schema::types::SchemaError::InvalidField(
                    "Transform input names cannot be empty".to_string()
                ));
            }
        }

        // Parse and validate all field expressions for alignment
        let mut parsed_chains = Vec::new();
        let mut parsing_errors = Vec::new();
        
        for (field_name, field_def) in &schema.fields {
            if let Some(atom_uuid_expr) = &field_def.atom_uuid {
                let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
                match parser.parse(atom_uuid_expr) {
                    Ok(parsed_chain) => {
                        parsed_chains.push(parsed_chain);
                    }
                    Err(parse_error) => {
                        parsing_errors.push((field_name.clone(), atom_uuid_expr.clone(), parse_error));
                    }
                }
            }
        }

        // Report any parsing errors with detailed feedback
        if !parsing_errors.is_empty() {
            let error_details: Vec<String> = parsing_errors.iter()
                .map(|(field, expr, error)| {
                    format!("Field '{}' expression '{}': {}", field, expr, 
                           self.convert_iterator_error_to_readable_message(error))
                })
                .collect();
            
            return Err(crate::schema::types::SchemaError::InvalidField(format!(
                "Transform field expression validation failed: {}", 
                error_details.join("; ")
            )));
        }

        // Validate field alignment across all expressions
        if !parsed_chains.is_empty() {
            let validator = FieldAlignmentValidator::new();
            match validator.validate_alignment(&parsed_chains) {
                Ok(alignment_result) => {
                    if !alignment_result.valid {
                        let error_messages: Vec<String> = alignment_result.errors.iter()
                            .map(|err| format!("{:?}: {}", err.error_type, err.message))
                            .collect();
                        return Err(crate::schema::types::SchemaError::InvalidField(format!(
                            "Transform field alignment validation failed: {}", 
                            error_messages.join("; ")
                        )));
                    }

                    // Provide guidance on warnings
                    for warning in &alignment_result.warnings {
                        log::warn!("Transform validation warning: {:?}: {} (Fields: {})", 
                                  warning.warning_type, warning.message, warning.fields.join(", "));
                    }
                }
                Err(iterator_error) => {
                    return Err(crate::schema::types::SchemaError::InvalidField(format!(
                        "Transform field alignment error: {}", 
                        self.convert_iterator_error_to_readable_message(&iterator_error)
                    )));
                }
            }
        }

        // Schema type specific validation for transforms
        match &schema.schema_type {
            crate::schema::types::schema::SchemaType::HashRange => {
                self.validate_hashrange_transform_requirements(schema)?;
            }
            crate::schema::types::schema::SchemaType::Range { range_key } => {
                self.validate_range_transform_requirements(schema, range_key)?;
            }
            crate::schema::types::schema::SchemaType::Single => {
                self.validate_single_transform_requirements(schema)?;
            }
        }

        Ok(())
    }

    /// Converts iterator stack errors to user-friendly messages
    fn convert_iterator_error_to_readable_message(&self, error: &crate::transform::iterator_stack::errors::IteratorStackError) -> String {
        use crate::transform::iterator_stack::errors::IteratorStackError;

        match error {
            IteratorStackError::InvalidChainSyntax { expression, reason } => {
                format!("Expression '{}' has invalid syntax: {}. Check for typos in field names and method calls.", expression, reason)
            }
            IteratorStackError::IncompatibleFanoutDepths { field1, depth1, field2, depth2 } => {
                format!("Fields '{}' (depth {}) and '{}' (depth {}) have incompatible iterator depths. Consider using a reducer function or restructuring the expressions.", 
                       field1, depth1, field2, depth2)
            }
            IteratorStackError::CartesianFanoutError { field1, branch1, field2, branch2 } => {
                format!("Fields '{}' (branch '{}') and '{}' (branch '{}') create a cartesian product, which may cause performance issues. Consider restructuring to use the same branch.", 
                       field1, branch1, field2, branch2)
            }
            IteratorStackError::ReducerRequired { field, current_depth, max_depth } => {
                format!("Field '{}' at depth {} requires a reducer function (max allowed depth: {}). Add a reducer like .count(), .first(), or .last() to the expression.", 
                       field, current_depth, max_depth)
            }
            IteratorStackError::InvalidIteratorChain { chain, reason } => {
                format!("Iterator chain '{}' is invalid: {}. Check the chain structure and method order.", chain, reason)
            }
            IteratorStackError::AmbiguousFanoutDifferentBranches { branches } => {
                format!("Ambiguous fan-out across different branches: {}. Ensure all fields use consistent branching patterns.", branches.join(", "))
            }
            IteratorStackError::MaxDepthExceeded { current_depth, max_depth } => {
                format!("Iterator depth {} exceeds maximum allowed depth {}. Consider using simpler expressions or increasing the depth limit.", current_depth, max_depth)
            }
            IteratorStackError::FieldAlignmentError { field, reason } => {
                format!("Field '{}' has alignment issues: {}. Check the field expression for compatibility with other fields.", field, reason)
            }
            IteratorStackError::ExecutionError { message } => {
                format!("Execution error: {}. This may indicate an issue with the expression logic or data structure.", message)
            }
        }
    }

    /// Validates HashRange transform specific requirements
    fn validate_hashrange_transform_requirements(
        &self, 
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition
    ) -> Result<(), crate::schema::types::SchemaError> {
        let key_config = schema.key.as_ref().ok_or_else(|| {
            crate::schema::types::SchemaError::InvalidField(
                "HashRange transform requires key configuration with hash_field and range_field".to_string()
            )
        })?;

        // Ensure the output field name is appropriate for HashRange usage
        if self.output.trim().is_empty() {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "HashRange transform output field cannot be empty".to_string()
            ));
        }

        // Validate that hash_field and range_field are different
        if key_config.hash_field == key_config.range_field {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "HashRange hash_field and range_field must be different expressions".to_string()
            ));
        }

        Ok(())
    }

    /// Validates Range transform specific requirements
    fn validate_range_transform_requirements(
        &self, 
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
        range_key: &str
    ) -> Result<(), crate::schema::types::SchemaError> {
        // Ensure the range_key field exists in the schema
        if !schema.fields.contains_key(range_key) {
            return Err(crate::schema::types::SchemaError::InvalidField(format!(
                "Range transform range_key '{}' not found in schema fields", 
                range_key
            )));
        }

        // Ensure the output field name is appropriate for Range usage  
        if self.output.trim().is_empty() {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "Range transform output field cannot be empty".to_string()
            ));
        }

        Ok(())
    }

    /// Validates Single transform specific requirements
    fn validate_single_transform_requirements(
        &self, 
        schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition
    ) -> Result<(), crate::schema::types::SchemaError> {
        // Single transforms should have simple field structures
        let field_count = schema.fields.len();
        if field_count > 10 {
            log::warn!("Single transform has {} fields, which may impact performance. Consider breaking into smaller transforms or using HashRange schema.", field_count);
        }

        // Ensure the output field name is appropriate for Single usage
        if self.output.trim().is_empty() {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "Single transform output field cannot be empty".to_string()
            ));
        }

        Ok(())
    }

    /// Get debug information about the transform for logging and debugging
    pub fn get_debug_info(&self) -> String {
        let transform_type = match &self.kind {
            crate::schema::types::json_schema::TransformKind::Procedural { logic } => {
                format!("Procedural (logic: {})", logic)
            }
            crate::schema::types::json_schema::TransformKind::Declarative { schema } => {
                format!("Declarative (schema: {})", schema.name)
            }
        };
        
        format!(
            "Transform{{type: {}, inputs: {:?}, output: {}}}",
            transform_type, self.inputs, self.output
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::ast::{Expression, Operator, TransformDeclaration};

    #[test]
    fn test_transform_from_declaration() {
        let declaration = TransformDeclaration {
            name: "test_transform".to_string(),
            logic: vec![Expression::Return(Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Variable("field1".to_string())),
                operator: Operator::Add,
                right: Box::new(Expression::Variable("field2".to_string())),
            }))],
            reversible: false,
            signature: None,
        };

        let transform = Transform::from_declaration(declaration);

        assert_eq!(transform.get_procedural_logic().unwrap(), "return (field1 + field2)"); // Removed trailing semicolon
        assert_eq!(transform.output, "test.test_transform"); // Output derived from declaration name
        assert!(transform.parsed_expression.is_none());
    }

    #[test]
    fn test_output_field() {
        let transform = Transform::new("return x + 1".to_string(), "test.number".to_string());

        assert_eq!(transform.get_output(), "test.number");
    }
}
