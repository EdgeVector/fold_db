# TASK-003: State Management Consolidation with Redux

[Back to task list](./tasks.md)

## Description

Consolidate schema state management into a centralized Redux slice to eliminate prop drilling, reduce local state complexity, and ensure consistent schema data across all components. This task will extend the existing Redux store (currently managing authentication in [`authSlice.ts`](../../../src/datafold_node/static-react/src/store/authSlice.ts)) to include comprehensive schema state management.

The primary focus is removing the schema state management from [`App.jsx:22-23,38-94`](../../../src/datafold_node/static-react/src/App.jsx:22) and establishing a single source of truth for schema data that enforces SCHEMA-002 compliance at the store level.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 17:30:00 | Created | N/A | Proposed | Task file created for Redux state consolidation | System |

## Requirements

### Core Requirements
- Create `schemaSlice.ts` Redux slice for centralized schema state management
- Implement async thunks for schema operations (fetch, approve, block, unload)
- Enforce SCHEMA-002 compliance at the store level
- Maintain existing functionality while eliminating prop drilling
- Integrate with existing authentication state for access control

### Required Constants (Section 2.1.12)
```typescript
const SCHEMA_CACHE_TTL_MS = 300000; // 5 minutes
const SCHEMA_FETCH_RETRY_ATTEMPTS = 3;
const SCHEMA_OPERATION_TIMEOUT_MS = 10000;
const REDUX_BATCH_SIZE = 50;
const SCHEMA_STATE_PERSIST_KEY = 'datafold_schemas';
```

### DRY Compliance Requirements
- Single source of truth for all schema data
- Centralized schema operation logic (approve, block, unload)
- Unified error handling and loading states
- Shared schema filtering and validation logic
- Common cache invalidation patterns

### SCHEMA-002 Compliance
- Store-level enforcement of schema access rules
- Selectors that filter schemas by approval state
- Action validation that prevents operations on non-approved schemas
- Automatic state synchronization with backend schema states

## Implementation Plan

### Phase 1: Create Schema Redux Slice
1. **Schema State Structure**
   ```typescript
   interface SchemaState {
     schemas: Record<string, Schema>;
     loading: {
       fetch: boolean;
       operations: Record<string, boolean>; // keyed by schema name
     };
     errors: {
       fetch: string | null;
       operations: Record<string, string>; // keyed by schema name
     };
     lastFetched: number | null;
     cache: {
       ttl: number;
       version: string;
     };
   }
   ```

2. **Initial State**
   ```typescript
   const initialState: SchemaState = {
     schemas: {},
     loading: {
       fetch: false,
       operations: {}
     },
     errors: {
       fetch: null,
       operations: {}
     },
     lastFetched: null,
     cache: {
       ttl: SCHEMA_CACHE_TTL_MS,
       version: '1.0.0'
     }
   };
   ```

### Phase 2: Implement Async Thunks
1. **Schema Fetching**
   - Create `fetchSchemas` async thunk to replace [`App.jsx:42-94`](../../../src/datafold_node/static-react/src/App.jsx:42) logic
   - Implement cache validation with `SCHEMA_CACHE_TTL_MS`
   - Add retry mechanism with `SCHEMA_FETCH_RETRY_ATTEMPTS`
   - Handle both available and persisted schema states

2. **Schema Operations**
   ```typescript
   // Async thunks for schema operations
   export const approveSchema = createAsyncThunk(
     'schemas/approve',
     async (schemaName: string, { rejectWithValue }) => {
       // Implementation with SCHEMA_OPERATION_TIMEOUT_MS
     }
   );
   
   export const blockSchema = createAsyncThunk(
     'schemas/block',
     async (schemaName: string, { rejectWithValue }) => {
       // Implementation with state validation
     }
   );
   
   export const unloadSchema = createAsyncThunk(
     'schemas/unload',
     async (schemaName: string, { rejectWithValue }) => {
       // Implementation with SCHEMA-001 compliance
     }
   );
   ```

### Phase 3: Create SCHEMA-002 Compliant Selectors
1. **Basic Selectors**
   ```typescript
   // SCHEMA-002 compliant selectors
   export const selectApprovedSchemas = createSelector(
     [selectAllSchemas],
     (schemas) => schemas.filter(schema => schema.state === 'approved')
   );
   
   export const selectAvailableSchemas = createSelector(
     [selectAllSchemas],
     (schemas) => schemas.filter(schema => schema.state === 'available')
   );
   
   export const selectBlockedSchemas = createSelector(
     [selectAllSchemas],
     (schemas) => schemas.filter(schema => schema.state === 'blocked')
   );
   ```

2. **Advanced Selectors**
   ```typescript
   // Range schema selectors
   export const selectApprovedRangeSchemas = createSelector(
     [selectApprovedSchemas],
     (schemas) => schemas.filter(isRangeSchema)
   );
   
   // Schema operation selectors
   export const selectSchemaOperationState = createSelector(
     [selectSchemaState, (_, schemaName: string) => schemaName],
     (state, schemaName) => ({
       loading: state.loading.operations[schemaName] || false,
       error: state.errors.operations[schemaName] || null
     })
   );
   ```

### Phase 4: Integrate with Components
1. **Update App.jsx**
   - Replace local schema state with Redux selectors
   - Remove [`fetchSchemas`](../../../src/datafold_node/static-react/src/App.jsx:42) function
   - Use `useAppDispatch` for schema operations
   - Maintain existing component interfaces

2. **Update Tab Components**
   - Refactor [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) to use Redux state
   - Update [`MutationTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/MutationTab.jsx) to use approved schema selectors
   - Modify [`QueryTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/QueryTab.jsx) for Redux integration

3. **Error Handling Integration**
   - Connect component error displays to Redux error state
   - Implement automatic error clearing on successful operations
   - Add global error handling for schema operations

### Phase 5: Cache Management and Persistence
1. **Cache Strategy**
   - Implement intelligent cache invalidation
   - Add cache versioning for schema updates
   - Handle cache warming on application startup

2. **State Persistence** (Optional)
   - Consider Redux Persist for schema state
   - Implement selective persistence (avoid persisting errors/loading states)
   - Add migration strategy for state structure changes

## Verification

### Unit Testing Requirements
- [ ] Redux slice reducers tested with all action types
- [ ] Async thunks tested with success and failure scenarios
- [ ] Selectors tested with various schema state combinations
- [ ] SCHEMA-002 compliance verified in all selectors
- [ ] Cache TTL behavior tested with time-based scenarios
- [ ] Error handling tested for network failures and timeouts

### Integration Testing Requirements
- [ ] Component integration tested with Redux state
- [ ] Schema operations tested end-to-end through Redux
- [ ] State synchronization tested across multiple components
- [ ] Authentication integration tested with schema access
- [ ] Cache invalidation tested with real API calls

### Performance Requirements
- [ ] Redux state updates do not cause unnecessary re-renders
- [ ] Selector memoization prevents excessive recalculations
- [ ] Large schema lists handled efficiently (up to `REDUX_BATCH_SIZE`)
- [ ] Memory usage remains stable with extended usage

### Documentation Requirements
- [ ] Redux slice actions and reducers documented
- [ ] Selector usage examples provided
- [ ] State structure documented with TypeScript interfaces
- [ ] Migration guide created for component updates

## Files Modified

### Created Files
- `src/datafold_node/static-react/src/store/schemaSlice.ts`
- `src/datafold_node/static-react/src/store/selectors/schemaSelectors.ts`
- `src/datafold_node/static-react/src/types/schema.ts`
- `src/datafold_node/static-react/src/store/middleware/schemaMiddleware.ts`

### Modified Files
- `src/datafold_node/static-react/src/store/store.ts` - Add schema reducer
- `src/datafold_node/static-react/src/App.jsx` - Remove local schema state, use Redux
- `src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx` - Use Redux selectors and actions
- `src/datafold_node/static-react/src/components/tabs/MutationTab.jsx` - Use approved schema selectors
- `src/datafold_node/static-react/src/components/tabs/QueryTab.jsx` - Use Redux schema state
- `src/datafold_node/static-react/src/components/tabs/TransformsTab.jsx` - Use Redux schema state

### Test Files
- `src/datafold_node/static-react/src/store/__tests__/schemaSlice.test.ts`
- `src/datafold_node/static-react/src/store/__tests__/schemaSelectors.test.ts`
- `src/datafold_node/static-react/src/store/__tests__/schemaThunks.test.ts`
- `src/datafold_node/static-react/src/test/integration/ReduxSchemaIntegration.test.tsx`

## Rollback Plan

If issues arise during Redux integration:

1. **State Isolation**: Temporarily disable schema slice and restore local state
2. **Component Rollback**: Revert components to use props instead of Redux selectors
3. **Incremental Migration**: Move one component at a time back to Redux
4. **Data Integrity**: Ensure no schema operations are lost during rollback
5. **Cache Preservation**: Maintain schema cache during rollback process
6. **Testing Verification**: Run full integration tests after rollback