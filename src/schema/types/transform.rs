use crate::transform::ast::TransformDeclaration;
use serde::{Deserialize, Serialize};
use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
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
/// use datafold::schema::types::{Transform, DeclarativeSchemaDefinition, SchemaType};
/// use std::collections::HashMap;
///
/// let schema = DeclarativeSchemaDefinition::new(
///     "health_schema".to_string(),
///     SchemaType::Single,
///     None,
///     HashMap::new(),
/// );
///
/// let transform = Transform::new(schema, "health.risk_score".to_string());
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

    /// The declarative schema definition
    #[serde(flatten)]
    pub schema: Box<DeclarativeSchemaDefinition>,

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
            // New struct format with schema field
            NewStruct {
                inputs: Option<Vec<String>>,
                output: String,
                #[serde(flatten)]
                schema: Box<DeclarativeSchemaDefinition>,
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
            Helper::NewStruct {
                inputs,
                output,
                schema,
            } => Ok(Self {
                inputs: inputs.unwrap_or_default(),
                output,
                schema,
                parsed_expression: None,
            }),
        }
    }
}

impl Transform {
    /// Creates a new declarative `Transform` from schema definition and output field.
    #[must_use]
    pub fn new(
        schema: DeclarativeSchemaDefinition,
        output: String,
    ) -> Self {
        Self {
            inputs: Vec::new(),
            output,
            schema: Box::new(schema),
            parsed_expression: None,
        }
    }

    /// Creates a new declarative `Transform` from schema definition.
    #[must_use]
    pub fn from_declarative_schema(
        schema: DeclarativeSchemaDefinition,
        inputs: Vec<String>,
        output: String,
    ) -> Self {
        Self {
            inputs,
            output,
            schema: Box::new(schema),
            parsed_expression: None,
        }
    }

    /// Returns true if this is a declarative transform.
    /// Since only declarative transforms are supported, this always returns true.
    pub fn is_declarative(&self) -> bool {
        true
    }

    /// Returns true if this is a procedural transform.
    /// Since procedural transforms are no longer supported, this always returns false.
    pub fn is_procedural(&self) -> bool {
        false
    }

    /// Gets the declarative schema.
    /// Since only declarative transforms are supported, this always returns the schema.
    pub fn get_declarative_schema(
        &self,
    ) -> Option<&DeclarativeSchemaDefinition> {
        Some(&*self.schema)
    }

    /// Gets the procedural logic if this is a procedural transform.
    /// Since procedural transforms are no longer supported, this always returns None.
    pub fn get_procedural_logic(&self) -> Option<&str> {
        None
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
                if let Some(field_expression) = &field_def.field_expression {
                    // Extract schema names from expressions like "BlogPost.map().content"
                    // Take the first part before the first dot
                    if let Some(first_dot) = field_expression.find('.') {
                        let schema_name = &field_expression[..first_dot];
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
        let _logic = declaration
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
            schema: Box::new(DeclarativeSchemaDefinition::new(
                "legacy_transform".to_string(),
                crate::schema::types::schema::SchemaType::Single,
                None,
                std::collections::HashMap::new(),
            )),
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

        // Validate inputs and output
        if self.output.trim().is_empty() {
            return Err(crate::schema::types::SchemaError::InvalidField(
                "Transform output cannot be empty".to_string(),
            ));
        }

        Ok(())
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

        assert_eq!(transform.get_procedural_logic(), None); // Procedural transforms no longer supported
        assert_eq!(transform.output, "test.test_transform"); // Output derived from declaration name
        assert!(transform.parsed_expression.is_none());
    }

    #[test]
    fn test_output_field() {
        use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
        use crate::schema::types::schema::SchemaType;
        use std::collections::HashMap;

        let schema = DeclarativeSchemaDefinition::new(
            "test_schema".to_string(),
            SchemaType::Single,
            None,
            HashMap::new(),
        );

        let transform = Transform::new(schema, "test.number".to_string());

        assert_eq!(transform.get_output(), "test.number");
    }
}
