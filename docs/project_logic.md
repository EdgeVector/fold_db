# Project Logic

This document contains the most up-to-date and condensed information about the project's logical processes, built up over time. Each logic entry tracks which modules it applies to and any conflicts with other logic.

## Logic Table

| Logic_ID | Logic | Module | Updated At | Conflicts with |
|----------|-------|--------|------------|----------------|
| SCHEMA-001 | Once approved, schemas cannot be unloaded. They can only transition between approved and blocked states. | schema_manager, schema_routes | 2025-06-23 19:14:00 | None |
| SCHEMA-002 | Only approved schemas can be mutated or queried by the user. | query_routes, mutation_tab, query_tab, schema_manager | 2025-06-23 19:17:00 | None |
| SCHEMA-003 | Transforms can write field values to any schema regardless of state, but cannot modify schema structure. | transform_manager, transform_queue, schema_manager | 2025-06-23 19:23:00 | None |
| SCHEMA-004 | Available schemas are discovered from JSON files only and are not loaded automatically. | schema/discovery, schema/persistence | 2025-06-24 21:49:43 | None |
| API-CLIENT-001 | All frontend API operations must use specialized unified API clients instead of direct fetch() calls. | api/clients/*, api/core/*, components/tabs/* | 2025-06-28 19:02:00 | None |
| API-ERROR-001 | API error handling must be standardized across all clients with consistent error types and user messages. | api/core/errors, api/clients/* | 2025-06-28 19:02:00 | None |
| API-CONFIG-001 | API endpoint URLs and configuration must be centralized to eliminate duplication and ensure consistency. | constants/api, api/endpoints | 2025-06-28 19:02:00 | None |
| API-CACHE-001 | API clients must implement intelligent caching, request deduplication, and timeout management for performance. | api/core/client, api/core/cache | 2025-06-28 19:02:00 | None |
| API-SEC-001 | Authentication patterns and security validation must be standardized across all API operations. | api/core/client, utils/authenticationWrapper | 2025-06-28 19:02:00 | None |
| INGESTION-001 | Large file ingestion must use streaming architecture with configurable batch processing to handle files of any size without memory constraints. | ingestion/core, ingestion/large_file | 2025-01-27 15:30:00 | None |
| TRANSFORM-001 | Transform system must support both procedural and declarative transform types seamlessly while maintaining backward compatibility. | transform/, schema/types, fold_db_core/transform_manager, fold_db_core/orchestration | 2025-01-27 12:00:00 | None |
| TRANSFORM-003 | DeclarativeSchemaDefinition requires KeyConfig with hash_field and range_field for HashRange schemas and FieldDefinition metadata for optional atom_uuid and field_type. | schema/types/json_schema.rs | 2025-08-26 19:00:00 | None |

### SCHEMA-001: Schema State Transition Rules
- **Description**: Enforces valid state transitions for schema lifecycle management
- **Rationale**: Prevents data loss and ensures production schemas remain available; once approved, schemas are considered critical and cannot be unloaded
- **Valid Transitions**:
  - available → approved (initial approval)
  - approved → blocked (temporary blocking)
  - blocked → approved (re-approval)
- **Invalid Transitions**:
  - approved → unloaded (prohibited - approved schemas cannot be unloaded)
  - blocked → unloaded (prohibited - previously approved schemas cannot be unloaded)

### SCHEMA-002: Schema Access Control Rules
- **Description**: Restricts user access to mutation and query operations based on schema state
- **Rationale**: Ensures data integrity and prevents operations on schemas that are not ready for production use
- **Access Rules**:
  - Only schemas with "approved" state can be used for mutations
  - Only schemas with "approved" state can be used for queries
  - Available and blocked schemas are read-only for inspection purposes
  - Transform viewing is allowed for all schemas (inspection purposes only)
- **Enforcement Points**:
  - Query tab should only allow queries against approved schemas
  - Mutation tab should only allow mutations against approved schemas
  - Transforms tab shows all transforms for inspection regardless of schema state
  - API endpoints should validate schema state before processing requests

### SCHEMA-003: Transform Field Write Access
- **Description**: Allows transforms to write field values while protecting schema structure integrity
- **Rationale**: Transforms need to update computed field values as part of their execution, but should not be able to modify schema definitions
- **Access Rules**:
  - Transforms can write/update field values in any schema regardless of approval state
  - Transforms cannot modify schema structure (add/remove fields, change field types)
  - Field value updates are treated as data operations, not schema modifications
  - Schema immutability applies only to structural changes, not field value updates
- **Implementation Notes**:
  - Backend should distinguish between schema structure modifications and field value updates
  - Transform execution should use field value update operations, not schema modification operations
  - Error handling should clearly differentiate between these operation types

### SCHEMA-004: Schema Discovery Rules
- **Description**: Defines how available schemas are discovered and managed in the system
- **Rationale**: Ensures clear separation between schema availability discovery and actual schema loading/persistence
- **Discovery Rules**:
  - Available schemas are discovered from JSON files in the available_schemas directory
  - Schema discovery does not automatically load schemas into the system
  - Discovery is a read-only operation that identifies potential schemas
  - Loading and persistence are separate operations that require explicit user action
- **Implementation Notes**:
  - Schema discovery scans filesystem for valid JSON schema files
  - Discovery results show schema metadata without loading schema data
  - User must explicitly approve/load discovered schemas for system use

### API-CLIENT-001: Unified API Client Architecture
- **Description**: Mandates use of specialized unified API clients for all frontend HTTP operations
- **Rationale**: Eliminates code duplication, ensures consistency, and provides type safety across all API operations
- **Architecture Rules**:
  - All API operations must use specialized clients (SchemaClient, SystemClient, SecurityClient, etc.)
  - Direct `fetch()` calls are prohibited in favor of unified client methods
  - Each domain has a dedicated client with domain-specific operations
  - All clients inherit from unified core client with shared functionality
- **Enforcement Points**:
  - Components and hooks must import from `api/clients/*` instead of using fetch directly
  - Build-time linting should detect direct fetch usage and suggest client alternatives
  - Code reviews should verify compliance with client architecture
- **Implementation Notes**:
  - Core client provides authentication, error handling, caching, and retry logic
  - Specialized clients extend core functionality with domain-specific methods
  - Type safety enforced through TypeScript interfaces for all requests/responses

### API-ERROR-001: Standardized Error Handling
- **Description**: Requires consistent error handling patterns across all API operations
- **Rationale**: Provides predictable error behavior and user experience across the application
- **Error Handling Rules**:
  - All API clients must use standardized `ApiError` types from `api/core/errors`
  - Error messages must be user-friendly and actionable
  - HTTP status codes must be consistently mapped to error types
  - Retry logic must be implemented for transient errors
- **Error Categories**:
  - Network errors (connection failures, timeouts)
  - Authentication errors (401, 403)
  - Validation errors (400, 422)
  - Server errors (500, 502, 503)
  - Not found errors (404)
- **Implementation Notes**:
  - Core client handles error transformation and standardization
  - Components receive consistent error objects regardless of API endpoint
  - Error boundaries can catch and display standardized error messages

### API-CONFIG-001: Centralized API Configuration
- **Description**: Centralizes all API endpoint URLs and configuration to eliminate duplication
- **Rationale**: Ensures consistency, reduces maintenance burden, and enables easy environment configuration
- **Configuration Rules**:
  - All API endpoints must be defined in `constants/api.ts`
  - Base URLs must be configurable per environment
  - Endpoint paths must use consistent naming conventions
  - No hardcoded URLs allowed in components or clients
- **Configuration Structure**:
  - Base URL configuration for different environments
  - Endpoint path constants organized by domain
  - Request/response timeout configurations
  - Default headers and authentication settings
- **Implementation Notes**:
  - Configuration exported as typed constants for compile-time checking
  - Environment-specific overrides supported through build configuration
  - All clients import endpoints from centralized configuration

### API-CACHE-001: API Caching and Performance Strategy
- **Description**: Implements intelligent caching, request deduplication, and timeout management
- **Rationale**: Improves application performance, reduces server load, and enhances user experience
- **Caching Rules**:
  - GET requests must implement intelligent caching based on data volatility
  - Duplicate concurrent requests must be deduplicated automatically
  - Cache invalidation must be triggered by relevant mutations
  - Request timeouts must be configured appropriately per operation type
- **Cache Strategies**:
  - Schema data: Long-term caching with explicit invalidation
  - System status: Short-term caching with automatic refresh
  - User data: Session-based caching with privacy considerations
  - Real-time data: Minimal or no caching
- **Implementation Notes**:
  - Core client manages cache lifecycle and invalidation
  - Cache keys generated consistently across all operations
  - Memory usage monitored to prevent cache bloat

### API-SEC-001: Authentication and Security Patterns
- **Description**: Standardizes authentication and security validation across all API operations
- **Rationale**: Ensures consistent security posture and simplifies authentication management
- **Security Rules**:
  - All authenticated requests must use standardized authentication headers
  - Request signing must be handled automatically by the client layer
  - Authentication state must be managed centrally
  - Security validation must be consistent across all endpoints
- **Authentication Patterns**:
  - Automatic authentication header injection for protected endpoints
  - Request signing for mutation operations using cryptographic keys
  - Token refresh handling for expired authentication
  - Graceful handling of authentication failures
- **Implementation Notes**:
  - Core client integrates with authentication context
  - Security validation performed before request transmission
  - Authentication errors handled consistently across all operations

### INGESTION-001: Large File Ingestion Architecture
- **Description**: Mandates streaming architecture with batch processing for handling large files efficiently
- **Rationale**: Enables processing of files of any size without memory constraints while maintaining performance and reliability
- **Architecture Rules**:
  - All large file ingestion must use streaming parsers to avoid loading entire files into memory
  - Data must be processed in configurable batch sizes with memory limits
  - Progress tracking and checkpointing must be implemented for resume capability
  - Multiple file formats (JSON, CSV, NDJSON) must be supported
- **Processing Rules**:
  - Files must be validated before processing begins
  - Schema analysis must be performed on initial batches
  - Batch processing must be configurable (default: 1000 records)
  - Error handling must allow partial success with detailed reporting
- **Implementation Notes**:
  - Use streaming parsers (serde_json::StreamDeserializer, csv::Reader)
  - Implement batch buffers with configurable memory limits
  - Create progress tracking with persistent checkpoints
  - Support pause/resume/cancel operations for long-running jobs
- **Performance Requirements**:
  - Memory usage must remain constant regardless of file size
  - Processing rate should scale with available CPU cores
  - Database operations must use batch operations for efficiency
  - Temporary files must be managed securely and cleaned up automatically

### TRANSFORM-001: Declarative Transforms System Architecture
- **Description**: Extends the transform system to support both procedural and declarative transform types seamlessly
- **Rationale**: Enables users to define transforms using declarative JSON schema definitions instead of writing procedural DSL code, while maintaining backward compatibility
- **Architecture Rules**:
  - TransformKind enum must support both Procedural and Declarative variants
  - Declarative transforms must use DeclarativeSchemaDefinition with proper validation
  - Both transform types must coexist in the same system without conflicts
  - Existing procedural transforms must continue to work unchanged
- **Data Structure Rules**:
  - DeclarativeSchemaDefinition must support "Single" and "HashRange" schema types
  - HashRange schemas must include KeyConfig with hash_field and range_field expressions
  - Field definitions must support atom UUID mappings and type inference
  - All structs must have proper serde serialization/deserialization
- **Integration Rules**:
  - New data structures must integrate with existing TransformManager and TransformOrchestrator
  - Both transform types must use the same queue and execution infrastructure
  - Field-to-transform mappings must work for both transform types
  - Validation must provide clear error messages for invalid configurations
- **Implementation Notes**:
  - Use serde's tag-based serialization for TransformKind variants
  - Implement validation traits for declarative transform structures
  - Maintain backward compatibility through proper field defaults
  - Add comprehensive testing for both transform types
- **Performance Requirements**:
  - No unacceptable performance degradation from new functionality
  - Validation must run efficiently for large schema definitions
  - Serialization/deserialization must maintain acceptable performance
  - Integration with existing components must be seamless

### TRANSFORM-002: Declarative Transform Specification Alignment
- **Description**: Declarative transforms must use the exact data structures and integration patterns specified in schema-generation-transforms.md, and must leverage existing iterator stack infrastructure
- **Rationale**: Ensures consistency between documentation and implementation, guarantees seamless integration with existing transform infrastructure, and leverages proven, tested iterator stack components
- **Specification Rules**:
  - Data structures must exactly match schema-generation-transforms.md definitions
  - Integration points must follow the documented queue and orchestration patterns
  - Transform registration must use the same registry as procedural transforms
  - Queue integration must use identical QueueItem structures for both transform types
  - **Iterator stack integration must use existing infrastructure components**
- **Required Structures**:
  - JsonTransform with TransformKind enum (Procedural/Declarative variants)
  - DeclarativeSchemaDefinition with name, schema_type, key, and fields
  - KeyConfig with hash_field and range_field for HashRange schemas
  - FieldDefinition with atom_uuid and field_type for reference fields
- **Integration Requirements**:
  - Declarative transforms stored in same transform registry as procedural transforms
  - Same QueueItem structure used for both transform types
  - TransformOrchestrator processes both types through same execution flow
  - Field-to-transform mappings created for automatic triggering
  - Automatic queuing when source schema data changes
  - **Iterator Stack Integration Requirements**:
    - Must use existing `IteratorStack` from `src/schema/indexing/iterator_stack.rs`
    - Must leverage existing `ChainParser` from `src/schema/indexing/chain_parser.rs`
    - Must integrate with existing `ExecutionEngine` from `src/schema/indexing/execution_engine.rs`
    - Must use existing field alignment validation from `src/schema/indexing/field_alignment.rs`
    - Must leverage existing error types from `src/schema/indexing/errors.rs`
- **Implementation Notes**:
  - Schema interpreter must parse declarative transforms from JSON schemas
  - Transform creation code must handle both procedural and declarative types
  - Transform processing must route to appropriate execution path
  - Storage and retrieval must support both transform types
  - Validation must ensure declarative transforms have required fields
  - Backward compatibility maintained for all existing procedural transforms
  - **Iterator Stack Implementation Notes**:
    - Declarative expressions must be parsed into existing `ParsedChain` format
    - Iterator stacks must be created using `IteratorStack::from_chain()` method
    - Execution must use `ExecutionEngine::execute_chains()` method
    - Field alignment validation must use existing validation logic
    - Performance optimizations must leverage existing iterator stack optimizations

### Migration and Breaking Changes (API-STD-1)
- **Description**: Documents the comprehensive migration from direct fetch() usage to unified API clients
- **Migration Scope**: 86 individual fetch implementations consolidated into 6 specialized clients
- **Breaking Changes**:
  - `httpClient.ts` utilities are deprecated and should not be used
  - Direct `fetch()` calls in components replaced with specialized client methods
  - Authentication patterns changed from manual header management to automatic injection
  - Error handling moved from component-level to client-level standardization
- **Migration Benefits**:
  - Eliminated code duplication across 86 API operations
  - Introduced full TypeScript safety for all API operations
  - Standardized error handling and user messaging
  - Implemented intelligent caching and performance optimizations
  - Centralized authentication and security patterns
- **References**:
  - Full migration details in [`docs/delivery/API-STD-1/migration-reference.md`](delivery/API-STD-1/migration-reference.md)
  - Architecture documentation in [`docs/delivery/API-STD-1/api-client-architecture.md`](delivery/API-STD-1/api-client-architecture.md)
  - Developer guide in [`docs/delivery/API-STD-1/developer-guide.md`](delivery/API-STD-1/developer-guide.md)