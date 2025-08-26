# [DTS-1-7C1] Basic Chain Parser Integration

[Back to task list](./tasks.md)

## Description

Implement basic integration with the existing `ChainParser` for single declarative expression parsing. This task focuses solely on parsing declarative expressions into the existing chain format without execution, validation, or multi-chain coordination.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 18:00:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Single Expression Parsing**: Parse individual declarative expressions using existing `ChainParser`
2. **Basic Error Handling**: Handle parsing errors for malformed expressions
3. **No Execution**: Defer actual execution to later tasks
4. **No Validation**: Defer field alignment validation to later tasks
5. **No Multi-Chain**: Handle only single expressions, not coordination between multiple

## Dependencies

- **DTS-1-7A**: Basic Transform Type Routing (must be completed first)
- **DTS-1-7B**: Simple Declarative Transform Execution (must be completed first)
- **DTS-1-6**: Schema Interpreter (for parsing declarative transforms)
- **DTS-1-1**: TransformKind enum (for transform type detection)
- **DTS-1-2**: DeclarativeSchemaDefinition (for schema structure)

## Implementation Plan

### Step 1: Import Chain Parser
- **Import existing `ChainParser`** from `src/schema/indexing/chain_parser.rs`
- **Create basic instance** using `ChainParser::new()`
- **Add to transform executor** for declarative transform handling

### Step 2: Basic Expression Parsing
- **Parse single declarative expressions** like `"blogpost.map().content.split_by_word()"`
- **Convert to existing `ParsedChain` format** using existing parser
- **Handle basic parsing errors** without complex validation
- **Store parsed chains** for later use by other components

### Step 3: Basic Error Handling
- **Map parser errors** to appropriate error types
- **Provide clear error messages** for parsing failures
- **Handle common parsing issues** (invalid syntax, unsupported operations)
- **Ensure parsing failures don't crash** the transform system

### Step 4: Integration with Transform Executor
- **Add chain parsing** to declarative transform execution path
- **Store parsed chains** in transform context for later use
- **Basic logging** of parsing results and errors
- **Prepare structure** for future validation and execution

## Verification

1. **Single Expression Parsing**: Individual declarative expressions parse correctly
2. **Error Handling**: Parsing errors are handled gracefully with clear messages
3. **Integration**: Chain parser integrates with transform executor without errors
4. **No Execution**: No actual execution occurs (placeholder only)
5. **No Validation**: No field alignment validation occurs (deferred)
6. **No Multi-Chain**: Only single expressions are handled (coordination deferred)

## Files Modified

- `src/transform/executor.rs` - Add basic chain parser integration
- `tests/unit/transform/chain_parser_integration_tests.rs` - Add parsing tests

## Test Plan

### Objective
Verify that basic chain parser integration works correctly for parsing single declarative expressions without execution or validation.

### Test Scope
- Single expression parsing using existing ChainParser
- Basic error handling for parsing failures
- Integration with transform executor
- No execution or validation testing

### Environment & Setup
- Standard Rust test environment
- Existing ChainParser component
- Existing transform system components
- Completed DTS-1-7A and DTS-1-7B

### Mocking Strategy
- Mock external dependencies as needed
- Use existing ChainParser component for testing
- Use existing transform system components for testing
- Create test fixtures for parsing scenarios

### Key Test Scenarios
1. **Valid Single Expression**: Test parsing of valid declarative expressions
2. **Invalid Syntax**: Test error handling for malformed expressions
3. **Unsupported Operations**: Test handling of unsupported chain operations
4. **Integration**: Test integration with transform executor
5. **Error Messages**: Test that error messages are clear and helpful
6. **No Execution**: Verify no execution occurs (placeholder only)

### Success Criteria
- All chain parser integration tests pass
- Single expressions parse correctly into ParsedChain format
- Parsing errors are handled gracefully with clear messages
- Integration with transform executor works without errors
- No execution or validation occurs (properly deferred)
- No regression in existing functionality
