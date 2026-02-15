# FoldDB React Application (v2.0.0)

A modern React application for managing schemas, mutations, queries, and data operations with comprehensive state management and SCHEMA-002 compliance enforcement.

## Architecture Overview

This application follows a **simplified, modular architecture** designed for maintainability, testability, and performance:

- **🎯 Centralized State Management**: Redux Toolkit for predictable state updates
- **🔌 Unified API Client**: Standardized HTTP communication with retry logic and caching
- **🪝 Custom Hooks**: Reusable business logic encapsulation
- **🧩 Modular Components**: Extracted, reusable UI components
- **📋 SCHEMA-002 Compliance**: Built-in enforcement at all architectural layers
- **⚡ Performance Optimized**: Request deduplication, caching, and memoization

## Quick Start

### Prerequisites

- Node.js 18+ and npm
- Access to FoldDB backend API
- Valid authentication credentials

### Installation

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Run tests
npm test
```

### Development Server

The application runs on `http://localhost:5173` with hot module replacement (HMR) enabled.

## Project Structure

```
src/
├── api/                    # Unified API client system
│   ├── core/              # Core client functionality
│   ├── clients/           # Specialized API clients
│   └── endpoints.ts       # API endpoint definitions
├── components/            # Reusable UI components
│   ├── form/             # Form field components
│   ├── schema/           # Schema-related components
│   └── tabs/             # Tab content components
├── constants/             # Centralized configuration
├── hooks/                 # Custom React hooks
├── store/                 # Redux store and slices
├── styles/               # Global styles and themes
├── test/                 # Testing utilities and integration tests
├── types/                # TypeScript type definitions
└── utils/                # Utility functions
```

## Key Features

### 🔐 Authentication & Security

- **Cryptographic Authentication**: Ed25519 key-based authentication
- **Message Signing**: Automatic payload signing for secure API calls
- **Key Management**: Secure private key storage and management

### 📊 Schema Management

- **SCHEMA-002 Compliance**: Only approved schemas available for operations
- **State Transitions**: Available → Approved → Blocked state management
- **Range Schema Support**: Specialized handling for time-series data
- **Real-time Updates**: Live schema state synchronization

### 🔍 Query & Mutation Operations

- **Interactive Query Builder**: Visual query construction
- **Mutation Forms**: Type-safe data modification interfaces
- **Range Operations**: Specialized range schema operations
- **Validation**: Comprehensive input validation and error handling

### 🎨 User Interface

- **Responsive Design**: Mobile-first, accessible interface
- **Tab Navigation**: Authentication-aware navigation system
- **Form Components**: Reusable, validated form fields
- **Error Handling**: User-friendly error messages and recovery

## Architecture Patterns

### Custom Hooks for Business Logic

```jsx
// Schema management
const { approvedSchemas, isLoading, error } = useApprovedSchemas();

// Range schema operations
const { isRange, formatRangeMutation } = useRangeSchema();

// Form validation
const { validate, errors, isFormValid } = useFormValidation();
```

### Redux State Management

```jsx
// Centralized state with Redux Toolkit
const schemas = useAppSelector(selectApprovedSchemas);
const dispatch = useAppDispatch();

// Async operations with automatic error handling
dispatch(fetchSchemas());
```

### Unified API Client

```jsx
// Standardized API calls with built-in features
import { schemaClient, mutationClient } from './api';

const schemas = await schemaClient.getApprovedSchemas();
const result = await mutationClient.executeMutation(mutation);
```

## SCHEMA-002 Compliance

The application enforces **SCHEMA-002** compliance at multiple levels:

- **Hook Level**: Only approved schemas returned from data hooks
- **Component Level**: Form components filter to approved schemas only
- **API Level**: Client validates schema state before operations
- **Store Level**: Redux selectors automatically filter by approval state

This ensures that users can only perform mutations and queries on approved schemas, maintaining data integrity and operational safety.

## Development Guidelines

### Component Development

1. **Single Responsibility**: Each component has one clear purpose
2. **Props Interface**: Use TypeScript for prop definitions
3. **Accessibility**: Include ARIA attributes and keyboard navigation
4. **Error Boundaries**: Handle errors gracefully

### Hook Development

1. **Pure Functions**: No side effects in custom hooks
2. **Error Handling**: Comprehensive error scenarios
3. **Testing**: Isolated unit tests for all hooks
4. **Documentation**: Complete JSDoc documentation

### State Management

1. **Normalized State**: Flat, normalized data structures
2. **Immutability**: Use Redux Toolkit's Immer integration
3. **Async Operations**: Use createAsyncThunk for API calls
4. **Selectors**: Memoized selectors for derived state

## Testing Strategy

### Testing Pyramid

- **Unit Tests**: Hooks, utilities, and components (70%)
- **Integration Tests**: Component + API integration (25%)
- **End-to-End Tests**: Complete user workflows (5%)

### Running Tests

```bash
# Run all tests
npm test

# Run tests in watch mode
npm run test:watch

# Generate coverage report
npm run test:coverage

# Run integration tests
npm run test:integration
```

### Test Utilities

The application provides comprehensive testing utilities:

```jsx
// Hook testing
import { renderHookWithProviders } from '../test/utils/hookUtils';

// Component testing with Redux
import { renderWithProviders } from '../test/utils/testUtils';

// API mocking
import { createMockApiClient } from '../test/utils/apiMocks';
```

## Performance Considerations

### Optimizations Implemented

- **Request Deduplication**: Prevents duplicate API calls
- **Response Caching**: Configurable TTL-based caching
- **Memoized Selectors**: Prevents unnecessary re-computations
- **Component Memoization**: React.memo for pure components
- **Code Splitting**: Lazy loading for route components

### Monitoring

```jsx
// Built-in performance metrics
const apiMetrics = apiClient.getMetrics();
console.log('Cache hit rate:', apiMetrics.cacheHitRate);
console.log('Average response time:', apiMetrics.averageResponseTime);
```

## Configuration

### Environment Variables

```bash
# Development
VITE_API_BASE_URL=http://localhost:8080
VITE_ENABLE_API_LOGGING=true
VITE_CACHE_TTL_MS=300000

# Production
VITE_API_BASE_URL=https://api.folddb.com
VITE_ENABLE_API_LOGGING=false
VITE_CACHE_TTL_MS=600000
```

### Build Configuration

The application uses Vite for fast development and optimized production builds:

- **Fast Refresh**: Instant updates during development
- **Tree Shaking**: Eliminates unused code
- **Code Splitting**: Automatic chunk optimization
- **Asset Optimization**: Image and font optimization

## Documentation

### Complete Documentation Suite

- **[Architecture Guide](./ARCHITECTURE.md)**: Detailed architectural overview
- **[Migration Guide](./MIGRATION.md)**: Upgrading from legacy architecture
- **[Testing Guide](./TESTING.md)**: Testing strategies and utilities
- **[API Documentation](./docs/)**: Complete API reference

### Code Documentation

All components, hooks, and utilities include comprehensive JSDoc documentation:

```jsx
/**
 * Custom hook for managing approved schemas with SCHEMA-002 compliance
 * @returns {UseApprovedSchemasResult} Hook result with schemas and utilities
 * @example
 * const { approvedSchemas, isLoading } = useApprovedSchemas();
 */
```

## Contributing

### Development Workflow

1. Create feature branch from `main`
2. Implement changes with tests
3. Update documentation as needed
4. Run full test suite
5. Submit pull request with description

### Code Standards

- **TypeScript**: Strong typing for all new code
- **ESLint**: Enforced code quality standards
- **Prettier**: Consistent code formatting
- **Testing**: Minimum 80% test coverage
- **Documentation**: JSDoc for all public APIs

## Support & Resources

### Getting Help

- **Architecture Questions**: See [ARCHITECTURE.md](./ARCHITECTURE.md)
- **Migration Issues**: See [MIGRATION.md](./MIGRATION.md)
- **Testing Help**: See [TESTING.md](./TESTING.md)
- **API Questions**: Check [API Documentation](./docs/)

### Common Patterns

- **Schema Operations**: Using `useApprovedSchemas` hook
- **Form Validation**: Using `useFormValidation` hook
- **Range Schemas**: Using `useRangeSchema` utilities
- **Error Handling**: Using unified error types

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 2.0.0 | 2025-06-24 | Complete architecture redesign with React simplification |
| 1.x | Prior | Legacy architecture (deprecated) |

---

**Built with modern React patterns for the FoldDB ecosystem** 🚀

For detailed implementation guidance, see the [Architecture Documentation](./ARCHITECTURE.md).
