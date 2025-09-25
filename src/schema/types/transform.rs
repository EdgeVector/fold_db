use crate::transform::ast::TransformDeclaration;
use serde::{Deserialize, Serialize};
use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
use std::collections::{HashSet, HashMap};

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRegistration {
    /// The ID of the transform
    pub transform_id: String,
    /// The transform itself
    pub transform: Transform,
    /// Fields that trigger the transform
    pub trigger_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Transform {
    /// The declarative schema definition
    #[serde(flatten)]
    pub schema: Box<DeclarativeSchemaDefinition>,
}

// Custom deserialization for declarative transforms only
impl<'de> serde::Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Only support declarative schema format now
        let schema = DeclarativeSchemaDefinition::deserialize(deserializer)?;
        Ok(Self {
            schema: Box::new(schema),
        })
    }
}

impl Transform {
    /// Creates a new declarative `Transform` from schema definition and output field.
    #[must_use]
    pub fn new(
        schema: DeclarativeSchemaDefinition,
    ) -> Self {
        Self {
            schema: Box::new(schema),
        }
    }

    /// Creates a new declarative `Transform` from schema definition.
    #[must_use]
    pub fn from_declarative_schema(
        schema: DeclarativeSchemaDefinition,
    ) -> Self {
        Self {
            schema: Box::new(schema),
        }
    }

    /// Gets the declarative schema.
    /// Since only declarative transforms are supported, this always returns the schema.
    pub fn get_declarative_schema(
        &self,
    ) -> Option<&DeclarativeSchemaDefinition> {
        Some(&*self.schema)
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

        Self {
            schema: Box::new(DeclarativeSchemaDefinition::new(
                "legacy_transform".to_string(),
                crate::schema::types::schema::SchemaType::Single,
                None,
                None,
                None,
            )),
        }
    }
}