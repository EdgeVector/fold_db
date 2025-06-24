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

    /// The transform logic expressed in the DSL
    pub logic: String,

    /// Output field for this transform in `Schema.field` format
    pub output: String,

    /// The parsed expression (not serialized)
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
            Struct {
                inputs: Option<Vec<String>>,
                logic: String,
                output: String,
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
            Helper::Struct {
                inputs,
                logic,
                output,
            } => Ok(Self {
                inputs: inputs.unwrap_or_default(),
                logic,
                output,
                parsed_expression: None,
            }),
        }
    }
}

impl Transform {
    /// Creates a new `Transform` from raw logic and output field.
    #[must_use]
    pub fn new(logic: String, output: String) -> Self {
        Self {
            inputs: Vec::new(),
            logic,
            output,
            parsed_expression: None,
        }
    }

    /// Creates a new `Transform` with a pre-parsed expression.
    #[must_use]
    pub fn new_with_expr(
        logic: String,
        parsed_expression: crate::transform::ast::Expression,
        output: String,
    ) -> Self {
        Self {
            inputs: Vec::new(),
            logic,
            output,
            parsed_expression: Some(parsed_expression),
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

        // Split by dots to handle schema.field format
        for part in self.logic.split(|c: char| !c.is_alphanumeric() && c != '.') {
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
                    // Add just the field name, not the schema prefix
                    dependencies.insert(parts[1].to_string());
                }
            } else {
                // For backward compatibility, add the whole part if it's not a schema.field
                dependencies.insert(part.to_string());
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
            logic,
            output,
            parsed_expression: None,
        }
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

        assert_eq!(transform.logic, "return (field1 + field2)"); // Removed trailing semicolon
        assert_eq!(transform.output, "test.test_transform"); // Output derived from declaration name
        assert!(transform.parsed_expression.is_none());
    }

    #[test]
    fn test_output_field() {
        let transform = Transform::new("return x + 1".to_string(), "test.number".to_string());

        assert_eq!(transform.get_output(), "test.number");
    }
}
