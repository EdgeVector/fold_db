# [DTS-1-6] Schema Interpreter and Core Integration Updates

[Back to task list](./tasks.md)

## Description

Update the schema interpreter and core integration code to handle declarative transforms using the existing iterator stack infrastructure. This task focuses on the foundational changes needed to parse and interpret declarative transforms from JSON schemas while maintaining backward compatibility for procedural transforms.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 16:30:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 16:30:00 | Status Update | Proposed | InProgress | Started work | AI Agent |
| 2025-01-27 17:30:00 | Status Update | InProgress | Proposed | Reverted to Proposed - no actual implementation found in code | AI Agent |
| 2025-01-27 20:00:00 | Status Update | Proposed | InProgress | Restarted work on schema interpreter and core integration updates | User |
| 2025-01-27 21:00:00 | Status Update | InProgress | Done | Implementation complete - Transform type updated, schema interpreter compatible, integration tests pass | User |

## Requirements

1. **Schema Interpreter Updates**: Update `interpret_schema` function to handle declarative transforms
2. **Field Conversion Logic**: Modify `convert_field` to detect and handle both transform types
3. **Transform Creation Code**: Update code that creates transforms to handle both types
4. **Basic Integration**: Ensure declarative transforms can be registered and stored
5. **Backward Compatibility**: Ensure existing procedural transforms continue to work unchanged

## Implementation Plan

### Step 1: Update Schema Interpreter for Declarative Transforms
- **Update `interpret_schema` function** in `src/schema/schema_interpretation.rs`
- **Modify `convert_field` function** to detect and handle declarative transforms
- **Add logic to distinguish** between procedural and declarative transform types
- **Implement proper conversion** from `JsonTransform` to internal `Transform` types
- **Add basic validation** for declarative transform structures during interpretation

### Step 2: Update Field Conversion Logic
- **Handle both transform types** in field conversion:
  ```rust
  fn convert_field(json_field: JsonSchemaField) -> FieldVariant {
      let mut single_field = SingleField::new(
          json_field.permission_policy.into(),
          json_field.payment_config.into(),
          json_field.field_mappers,
      );

      if let Some(molecule_uuid) = json_field.molecule_uuid {
          single_field.set_molecule_uuid(molecule_uuid);
      }

      // Handle both transform types
      if let Some(json_transform) = json_field.transform {
          match json_transform {
              JsonTransform::Procedural { logic, inputs, output } => {
                  let transform = Transform::new(logic, output);
                  transform.set_inputs(inputs);
                  single_field.set_transform(transform.into());
              }
              JsonTransform::Declarative { schema, inputs, output } => {
                  let transform = Transform::from_declarative_schema(schema, inputs, output);
                  single_field.set_transform(transform.into());
              }
          }
      }

      FieldVariant::Single(single_field)
  }
  ```

### Step 3: Update Transform Creation Code
- **Identify all places where transforms are created**
  - Schema loading and interpretation
  - Transform registration endpoints
  - Transform import/export functionality
  - Test fixture creation

- **Update to handle both procedural and declarative types**
  - Add transform type detection logic
  - Implement proper initialization for declarative transforms
  - Add basic validation for declarative transform requirements
  - Ensure proper error handling for malformed declarative transforms

- **Add declarative transform factory methods**:
  ```rust
  impl Transform {
      pub fn from_declarative_schema(
          schema: DeclarativeSchemaDefinition,
          inputs: Vec<String>,
          output: String,
      ) -> Self {
          Self {
              inputs,
              output,
              kind: TransformKind::Declarative { schema },
              parsed_expression: None,
          }
      }

      pub fn is_declarative(&self) -> bool {
          matches!(self.kind, TransformKind::Declarative { .. })
      }

      pub fn get_declarative_schema(&self) -> Option<&DeclarativeSchemaDefinition> {
          if let TransformKind::Declarative { schema } = &self.kind {
              Some(schema)
          } else {
              None
          }
      }
  }
  ```

### Step 4: Basic Integration with Existing Infrastructure
- **Ensure declarative transforms can be registered** in existing transform registry
- **Verify basic storage and retrieval** works for both transform types
- **Test basic schema interpretation** for declarative transforms
- **Ensure backward compatibility** for existing procedural transforms

## Verification

1. **Schema Interpretation**: Declarative transforms are properly parsed from JSON schemas
2. **Transform Creation**: Both transform types can be created and initialized correctly
3. **Basic Integration**: Declarative transforms can be registered and stored
4. **Backward Compatibility**: Existing procedural transforms continue to work unchanged
5. **Error Handling**: Basic error handling for malformed declarative transforms
6. **Foundation**: Core infrastructure is in place for further integration

## Files Modified

- `src/schema/schema_interpretation.rs` - Update schema interpretation logic for declarative transforms
- `src/schema/types/transform.rs` - Add declarative transform factory methods and utilities
- `src/schema/transform.rs` - Update basic transform registration and processing
- `tests/unit/schema/declarative_transforms.rs` - Add basic unit tests for schema interpretation

## Test Plan

### Objective
Verify that the schema interpreter can properly parse and interpret declarative transforms from JSON schemas, and that basic integration with the existing transform system works correctly.

### Test Scope
- Schema interpretation updates for declarative transforms
- Basic transform creation for both types
- Basic integration with existing transform components
- Backward compatibility verification
- Basic error handling for malformed transforms

### Environment & Setup
- Standard Rust test environment
- Existing transform system components
- Test data fixtures for both transform types
- JSON schema files with declarative transforms

### Mocking Strategy
- Mock external dependencies as needed
- Use existing transform system components for basic integration testing
- Create test fixtures for various transform scenarios
- Test both valid and invalid declarative transform configurations

### Key Test Scenarios
1. **Schema Interpretation**: Test that declarative transforms are properly parsed from JSON schemas
2. **Transform Creation**: Test creating both procedural and declarative transforms
3. **Basic Integration**: Test basic registration and storage of both transform types
4. **Backward Compatibility**: Test existing procedural transforms still work
5. **Error Handling**: Test basic error scenarios for malformed declarative transforms
6. **Validation**: Test basic validation during schema interpretation

### Success Criteria
- All basic integration tests pass
- Declarative transforms are properly parsed and interpreted
- Both transform types can be created and registered
- Existing procedural transforms continue to work unchanged
- Basic error handling works for malformed declarative transforms
- Foundation is in place for further integration tasks
