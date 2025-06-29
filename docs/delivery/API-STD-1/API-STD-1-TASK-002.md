# [API-STD-1-TASK-002] System API Client Refactor

## Description
Create SystemClient and replace direct fetch() calls in StatusSection.jsx and LogSidebar.jsx with SystemClient methods to standardize system operation API access patterns and improve maintainability.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-28 17:50:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-06-28 17:53:00 | Status Update | Proposed | Done | Completed system API client refactor | AI Agent |

## Requirements
- Create new SystemClient with methods for system operations
- Replace direct fetch() call in StatusSection.jsx for database reset
- Replace direct fetch() call in LogSidebar.jsx for log retrieval
- Maintain existing functionality including log streaming
- Add SystemClient to API client exports
- Follow existing client patterns from SchemaClient and SecurityClient
- Implement proper TypeScript interfaces and JSDoc documentation

## Implementation Plan

### Phase 1: SystemClient Creation
1. Add system endpoints to API_ENDPOINTS:
   - `SYSTEM_RESET_DATABASE: '/api/system/reset-database'`
   - `SYSTEM_LOGS: '/api/logs'`
   - `SYSTEM_LOGS_STREAM: '/api/logs/stream'`
   - `SYSTEM_STATUS: '/system/status'` (for future use)

2. Create SystemClient with methods:
   - `getLogs()` - Get system logs array
   - `resetDatabase(confirm: boolean)` - Reset database with confirmation
   - `getSystemStatus()` - Get system health status (future endpoint)
   - `createLogStream()` - Helper for EventSource log streaming
   - `validateResetRequest()` - Client-side validation helper

### Phase 2: API Client Integration
1. Create `src/api/clients/index.ts` for centralized client exports
2. Export SystemClient alongside SchemaClient and SecurityClient
3. Include proper TypeScript type exports

### Phase 3: StatusSection.jsx Refactor  
1. Import systemClient
2. Replace fetch('/api/system/reset-database') with systemClient.resetDatabase()
3. Adapt error handling to use unified client response format
4. Maintain existing user feedback and confirmation dialog functionality

### Phase 4: LogSidebar.jsx Refactor
1. Import systemClient  
2. Replace fetch('/api/logs') with systemClient.getLogs()
3. Replace direct EventSource usage with systemClient.createLogStream()
4. Adapt error handling to use unified client response format
5. Maintain existing log streaming and display functionality

### Phase 5: Testing & Validation
1. Test database reset functionality still works correctly
2. Verify log display and streaming continues to function
3. Ensure error handling is maintained
4. Check TypeScript compilation passes

## Verification
- [x] SystemClient created following established patterns
- [x] System endpoints added to API_ENDPOINTS
- [x] SystemClient exported through index.ts
- [x] No direct fetch() call remains in StatusSection.jsx
- [x] No direct fetch() call remains in LogSidebar.jsx  
- [x] Database reset functionality preserved
- [x] Log retrieval and streaming functionality preserved
- [x] Error handling patterns maintained
- [x] TypeScript interfaces properly defined
- [x] JSDoc documentation included
- [x] Unified client response format used
- [x] Authentication patterns followed (reset requires auth, logs don't)

## Files Created/Modified
- `src/datafold_node/static-react/src/api/clients/systemClient.ts` (created)
- `src/datafold_node/static-react/src/api/clients/index.ts` (created)  
- `src/datafold_node/static-react/src/api/endpoints.ts` (modified - added system endpoints)
- `src/datafold_node/static-react/src/components/StatusSection.jsx` (refactored fetch call)
- `src/datafold_node/static-react/src/components/LogSidebar.jsx` (refactored fetch call)

## Impact Summary
This task eliminates **2 more fetch() violations** from the codebase:
1. StatusSection.jsx: `fetch('/api/system/reset-database', ...)`
2. LogSidebar.jsx: `fetch('/api/logs')`

**Progress: 18 of 33 fetch() violations resolved**

## SystemClient Methods

### Core Methods
- `getLogs()` - Retrieve system logs (UNPROTECTED)
- `resetDatabase(confirm: boolean)` - Reset database (PROTECTED)
- `getSystemStatus()` - Get system health status (UNPROTECTED, future endpoint)

### Helper Methods  
- `createLogStream(onMessage, onError)` - Create EventSource for real-time logs
- `validateResetRequest(request)` - Client-side request validation
- `getMetrics()` - Get system operation metrics
- `clearCache()` - Clear system-related cache

### Response Types
- `LogsResponse` - Logs array with metadata
- `ResetDatabaseResponse` - Reset operation result
- `SystemStatusResponse` - System health information

[Back to task list](./tasks.md)