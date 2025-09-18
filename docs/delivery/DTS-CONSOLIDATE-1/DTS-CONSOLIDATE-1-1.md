# [DTS-CONSOLIDATE-1-1] Analyze current executor patterns and create consolidation plan

[Back to task list](./tasks.md)

## Description

Analyze the three separate executor modules (`single_executor.rs`, `range_executor.rs`, `hash_range_executor.rs`) to identify common execution patterns, duplication, and dependencies. Create a detailed consolidation plan that will guide the implementation of a unified execution pattern in `executor.rs`.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 20:00:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 20:30:00 | Status Update | Proposed | InProgress | Started implementation | AI Agent |
| 2025-01-27 21:00:00 | Status Update | InProgress | Review | Implementation completed, all tests pass | AI Agent |
| 2025-01-27 21:15:00 | Status Update | Review | Done | Changes committed successfully | AI Agent |

## Requirements

1. **Pattern Analysis**: Identify the common execution patterns across all three executor modules
2. **Duplication Mapping**: Document specific code duplication and overlapping logic
3. **Dependency Analysis**: Map dependencies between executors and other modules
4. **Consolidation Plan**: Create detailed plan for merging the executors
5. **Risk Assessment**: Identify potential risks and mitigation strategies

## Implementation Plan

### Step 1: Pattern Analysis

1. **Extract Common Patterns**: Identify the 8-step execution pattern:
   - Logging execution start
   - Schema validation
   - Expression collection
   - Batch parsing
   - Field alignment validation
   - ExecutionEngine setup
   - Execution
   - Result aggregation

2. **Schema-Specific Logic**: Identify differences between Single, Range, and HashRange execution:
   - Single: Simple expression collection, direct execution
   - Range: Range key handling, range-specific coordination
   - HashRange: Key config extraction, multi-chain coordination

3. **Timing and Monitoring**: Document timing structures and monitoring patterns:
   - `ExecutionTiming` in hash_range_executor
   - `ValidationTimings` in validation module
   - Performance logging patterns

### Step 2: Duplication Mapping

1. **ExecutionEngine Setup**: Document the duplicated pattern:
   ```rust
   let mut execution_engine = ExecutionEngine::new();
   let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
   let execution_result = execution_engine.execute_fields(...)
   ```

2. **Multi-Chain Coordination**: Map the duplication between range_executor and coordination module

3. **Validation Logic**: Identify overlapping validation functions and timing measurements

### Step 3: Dependency Analysis

1. **Import Dependencies**: Map all imports and dependencies for each executor
2. **Shared Utilities Usage**: Document usage of shared utilities across executors
3. **Test Dependencies**: Identify test files that depend on the separate executors

### Step 4: Consolidation Plan

1. **Unified Structure**: Design the unified execution pattern:
   ```rust
   impl TransformExecutor {
       fn execute_declarative_transform_unified(...)
       fn execute_with_common_pattern<F>(...)
       fn execute_single_pattern(...)
       fn execute_range_pattern(...)
       fn execute_hashrange_pattern(...)
   }
   ```

2. **Migration Strategy**: Plan the step-by-step migration:
   - Implement unified pattern in executor.rs
   - Update TransformExecutor::execute_transform to use unified pattern
   - Delete separate executor modules
   - Update imports and module declarations

3. **Testing Strategy**: Plan for maintaining test coverage during consolidation

## Test Plan

### Objective
Ensure the consolidation plan is comprehensive and addresses all aspects of the executor modules.

### Test Scope
- Analysis completeness
- Plan feasibility
- Risk mitigation strategies
- Implementation roadmap clarity

### Key Test Scenarios
1. **Pattern Completeness**: Verify all common patterns are identified
2. **Duplication Coverage**: Ensure all duplication is mapped
3. **Dependency Accuracy**: Confirm all dependencies are documented
4. **Plan Feasibility**: Validate that the consolidation plan is implementable

### Success Criteria
- Complete pattern analysis document
- Comprehensive duplication mapping
- Accurate dependency analysis
- Detailed, implementable consolidation plan
- Risk assessment with mitigation strategies

## Files Modified

- `docs/delivery/DTS-CONSOLIDATE-1/DTS-CONSOLIDATE-1-1.md` - This analysis document
- Analysis artifacts (temporary files documenting findings)

## Verification

1. **Pattern Analysis**: All common execution patterns identified and documented
2. **Duplication Mapping**: All code duplication mapped with specific examples
3. **Dependency Analysis**: All dependencies accurately documented
4. **Consolidation Plan**: Detailed, step-by-step implementation plan created
5. **Risk Assessment**: Potential risks identified with mitigation strategies
6. **Stakeholder Review**: Plan reviewed for completeness and feasibility
