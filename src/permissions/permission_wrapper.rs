use crate::permissions::permission_manager::PermissionManager;
use crate::schema::types::{Field, Mutation, Query, SchemaError};
use crate::schema::SchemaCore;
use crate::logging::features::LogFeature;
use crate::log_feature;

/// Provides a high-level interface for permission validation on schema operations.
///
/// The PermissionWrapper coordinates between:
/// - Schema-level operations (queries and mutations)
/// - Field-level permission policies
/// - The underlying permission manager
///
/// It handles:
/// - Schema validation and lookup
/// - Field existence verification
/// - Permission policy enforcement
/// - Error reporting and aggregation
#[derive(Default)]
pub struct PermissionWrapper {
    permission_manager: PermissionManager,
}

/// Result of a field-level permission check.
///
/// Contains:
/// - The field name being checked
/// - Whether access is allowed
/// - Any error that occurred during the check
///
/// This structure provides detailed feedback about why
/// a permission check succeeded or failed.
#[derive(Debug, Clone)]
pub struct FieldPermissionResult {
    /// Name of the field being checked
    pub field_name: String,
    /// Whether access is allowed
    pub allowed: bool,
    /// Error details if access was denied or check failed
    pub error: Option<SchemaError>,
}

impl PermissionWrapper {
    /// Creates a new PermissionWrapper instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a query operation has permission to access a specific field.
    ///
    /// This method performs a complete permission check by:
    /// 1. Verifying the schema exists
    /// 2. Checking the field exists in the schema
    /// 3. Validating read permissions based on:
    ///    - The requesting public key
    ///    - The field's permission policy
    ///    - The trust distance from the requester
    ///
    /// # Arguments
    ///
    /// * `query` - The query operation being validated
    /// * `field_name` - Name of the field being accessed
    /// * `schema_manager` - Manager containing schema definitions
    ///
    /// # Returns
    ///
    /// A FieldPermissionResult containing the check result and any errors
    pub fn check_query_field_permission(
        &self,
        query: &Query,
        field_name: &str,
        schema_manager: &SchemaCore,
    ) -> FieldPermissionResult {
        log_feature!(
            LogFeature::Permissions,
            info,
            "FIELD PERMISSION CHECK: schema={}, field={}, pub_key={}, trust_distance={}",
            query.schema_name, field_name, query.pub_key, query.trust_distance
        );

        let schema = match schema_manager.get_schema(&query.schema_name) {
            Ok(Some(s)) => s,
            Ok(None) => {
                return FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed: false,
                    error: Some(SchemaError::NotFound(format!(
                        "Schema {} not found",
                        query.schema_name
                    ))),
                }
            }
            Err(e) => {
                return FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed: false,
                    error: Some(e),
                }
            }
        };

        schema.fields.get(field_name).map_or_else(
            || {
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "FIELD PERMISSION CHECK: field {} not found in schema {}",
                    field_name,
                    query.schema_name
                );
                FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed: false,
                    error: Some(SchemaError::InvalidField(format!(
                        "Field {field_name} not found"
                    ))),
                }
            },
            |field| {
                let policy = field.permission_policy();
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "FIELD PERMISSION CHECK: field={}, policy={:?}",
                    field_name,
                    policy
                );

                let allowed = self.permission_manager.has_read_permission(
                    &query.pub_key,
                    policy,
                    query.trust_distance,
                );

                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "FIELD PERMISSION CHECK RESULT: field={}, allowed={}",
                    field_name,
                    allowed
                );

                FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed,
                    error: if allowed {
                        None
                    } else {
                        Some(SchemaError::InvalidPermission(format!(
                            "Read access denied for field {field_name}"
                        )))
                    },
                }
            },
        )
    }

    /// Checks if a mutation operation has permission to modify a specific field.
    ///
    /// This method performs a complete permission check by:
    /// 1. Verifying the schema exists
    /// 2. Checking the field exists in the schema
    /// 3. Validating write permissions based on:
    ///    - The requesting public key
    ///    - The field's permission policy
    ///    - The trust distance from the requester
    ///
    /// Write permission checks are typically stricter than read checks,
    /// as they involve modifying data.
    ///
    /// # Arguments
    ///
    /// * `mutation` - The mutation operation being validated
    /// * `field_name` - Name of the field being modified
    /// * `schema_manager` - Manager containing schema definitions
    ///
    /// # Returns
    ///
    /// A FieldPermissionResult containing the check result and any errors
    pub fn check_mutation_field_permission(
        &self,
        mutation: &Mutation,
        field_name: &str,
        schema_manager: &SchemaCore,
    ) -> FieldPermissionResult {
        let schema = match schema_manager.get_schema(&mutation.schema_name) {
            Ok(Some(s)) => s,
            Ok(None) => {
                return FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed: false,
                    error: Some(SchemaError::NotFound(format!(
                        "Schema {} not found",
                        mutation.schema_name
                    ))),
                }
            }
            Err(e) => {
                return FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed: false,
                    error: Some(e),
                }
            }
        };

        schema.fields.get(field_name).map_or_else(
            || FieldPermissionResult {
                field_name: field_name.to_string(),
                allowed: false,
                error: Some(SchemaError::InvalidField(format!(
                    "Field {field_name} not found"
                ))),
            },
            |field| {
                if !field.writable() {
                    return FieldPermissionResult {
                        field_name: field_name.to_string(),
                        allowed: false,
                        error: Some(SchemaError::InvalidPermission(format!(
                            "Field {field_name} is not writable"
                        ))),
                    };
                }

                let allowed = self.permission_manager.has_write_permission(
                    &mutation.pub_key,
                    field.permission_policy(),
                    mutation.trust_distance,
                );

                FieldPermissionResult {
                    field_name: field_name.to_string(),
                    allowed,
                    error: if allowed {
                        None
                    } else {
                        Some(SchemaError::InvalidPermission(format!(
                            "Write access denied for field {field_name}"
                        )))
                    },
                }
            },
        )
    }
}
