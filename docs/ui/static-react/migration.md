# Migration Guide: React Simplification (v2.0.0)

This guide provides comprehensive instructions for migrating from the legacy React patterns to the new simplified architecture introduced in **PBI-REACT-SIMPLIFY-001**. The new architecture consolidates state management, standardizes API clients, and provides reusable components and hooks.

## Overview of Changes

### Architecture Transformation
- **State Management**: Migrated from local component state to centralized Redux store
- **API Clients**: Unified API client system replacing scattered fetch calls
- **Components**: Extracted reusable components from monolithic structures
- **Hooks**: Custom hooks for business logic encapsulation
- **Constants**: Centralized configuration and constants management

### Key Benefits
- **Reduced Complexity**: Eliminated prop drilling and state duplication
- **Better Performance**: Optimized re-renders and caching
- **Improved Maintainability**: Single source of truth for data and configuration
- **Enhanced Testability**: Isolated logic in testable units
- **SCHEMA-002 Compliance**: Enforced at architectural level

## Breaking Changes

### 1. Redux State Management

**Before (Local State):**
```jsx
// Legacy component with local state
function SchemaList() {
  const [schemas, setSchemas] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    setLoading(true);
    fetch('/api/schemas')
      .then(res => res.json())
      .then(data => {
        setSchemas(data);
        setLoading(false);
      })
      .catch(err => {
        setError(err.message);
        setLoading(false);
      });
  }, []);

  // Component logic...
}
```

**After (Redux + Custom Hook):**
```jsx
// New component using Redux and custom hooks
import { useApprovedSchemas } from '../hooks/useApprovedSchemas';

function SchemaList() {
  const {
    approvedSchemas,
    isLoading,
    error,
    refetch
  } = useApprovedSchemas();

  // Component logic is now simpler and more focused
}
```

**Migration Steps:**
1. Remove local state declarations
2. Replace with appropriate custom hook
3. Update component logic to use hook return values
4. Remove manual API calls and useEffect for data fetching

### 2. API Client Unification

**Before (Direct Fetch Calls):**
```jsx
// Legacy scattered fetch calls
const approveSchema = async (schemaName) => {
  try {
    const response = await fetch(`/api/schema/${schemaName}/approve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    
    return await response.json();
  } catch (error) {
    console.error('Approval failed:', error);
    throw error;
  }
};
```

**After (Unified API Client):**
```jsx
// New unified API client usage
import { schemaClient } from '../api';

const approveSchema = async (schemaName) => {
  try {
    const result = await schemaClient.approveSchema(schemaName);
    return result;
  } catch (error) {
    // Error handling is standardized
    console.error('Approval failed:', error.toUserMessage());
    throw error;
  }
};
```

**Migration Steps:**
1. Replace direct fetch calls with appropriate API client methods
2. Update error handling to use standardized error types
3. Remove custom retry logic (now handled by API client)
4. Update authentication handling (now automatic)

### 3. Component Extraction

**Before (Monolithic Components):**
```jsx
// Legacy large component with everything inline
function App() {
  return (
    <div>
      {/* Inline tab navigation */}
      <div className="border-b border-gray-200">
        <div className="flex space-x-8">
          {tabs.map(tab => (
            <button
              key={tab.id}
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === tab.id ? 'text-blue-600' : 'text-gray-500'
              }`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>
      
      {/* Inline form fields */}
      <div className="space-y-2">
        <label className="block text-sm font-medium">
          Schema Name
        </label>
        <input
          type="text"
          className="block w-full px-3 py-2 border"
          // ... more inline styling and logic
        />
      </div>
    </div>
  );
}
```

**After (Modular Components):**
```jsx
// New modular approach with extracted components
import TabNavigation from './components/TabNavigation';
import TextField from './components/form/TextField';

function App() {
  return (
    <div>
      <TabNavigation
        activeTab={activeTab}
        isAuthenticated={isAuthenticated}
        onTabChange={setActiveTab}
      />
      
      <TextField
        label="Schema Name"
        value={schemaName}
        onChange={setSchemaName}
        required
      />
    </div>
  );
}
```

**Migration Steps:**
1. Identify reusable UI patterns in existing components
2. Extract them to dedicated component files
3. Replace inline JSX with component usage
4. Move styling to component-level constants

### 4. Form Validation

**Before (Manual Validation):**
```jsx
// Legacy manual validation
function MutationForm() {
  const [errors, setErrors] = useState({});
  
  const validateField = (name, value) => {
    const newErrors = { ...errors };
    
    if (!value.trim()) {
      newErrors[name] = 'Field is required';
    } else if (name === 'rangeKey' && !isValidRangeKey(value)) {
      newErrors[name] = 'Invalid range key format';
    } else {
      delete newErrors[name];
    }
    
    setErrors(newErrors);
  };

  // Manual validation for each field...
}
```

**After (useFormValidation Hook):**
```jsx
// New validation using custom hook
import { useFormValidation } from '../hooks/useFormValidation';

function MutationForm() {
  const {
    validate,
    errors,
    isFormValid,
    createValidationRules
  } = useFormValidation();
  
  const handleFieldChange = (fieldName, value) => {
    const rules = [
      createValidationRules.required('Field is required'),
      createValidationRules.custom(isValidRangeKey, 'Invalid range key format')
    ];
    
    validate(fieldName, value, rules, true); // debounced
  };

  // Validation is now centralized and reusable
}
```

**Migration Steps:**
1. Replace manual validation logic with `useFormValidation` hook
2. Convert validation rules to the new rule format
3. Update error display to use hook's error state
4. Implement debounced validation for better UX

## Migration Checklist

### Pre-Migration Assessment
- [ ] Identify components using local state for data fetching
- [ ] Catalog direct API calls that need client migration
- [ ] List large components that need extraction
- [ ] Document current validation patterns

### State Management Migration
- [ ] Install and configure Redux store (if not already done)
- [ ] Replace data fetching useEffect with custom hooks
- [ ] Update component props to remove now-unnecessary state passing
- [ ] Test that data flow works correctly

### API Client Migration
- [ ] Replace fetch calls with unified API client methods
- [ ] Update error handling to use new error types
- [ ] Remove custom authentication logic
- [ ] Test API integration with new client

### Component Extraction
- [ ] Extract reusable UI patterns to dedicated components
- [ ] Move inline styles to component-level constants
- [ ] Update parent components to use extracted components
- [ ] Ensure accessibility attributes are preserved

### Validation Migration
- [ ] Replace manual validation with `useFormValidation` hook
- [ ] Convert validation rules to new format
- [ ] Implement debounced validation where appropriate
- [ ] Test form validation edge cases

### Constants and Configuration
- [ ] Move hardcoded values to constants files
- [ ] Update imports to use centralized constants
- [ ] Ensure consistent naming conventions
- [ ] Document configuration options

## Common Migration Issues

### Issue 1: State Not Updating
**Problem**: Component not re-rendering after migrating to Redux.
**Solution**: Ensure component is properly connected to Redux store using hooks.

```jsx
// Wrong: Not using Redux selector
function Component() {
  const [data, setData] = useState([]);
  // Component won't update when Redux state changes
}

// Right: Using Redux selector
function Component() {
  const data = useAppSelector(selectApprovedSchemas);
  // Component updates automatically with Redux changes
}
```

### Issue 2: API Errors Not Handled
**Problem**: API errors not displayed to user after client migration.
**Solution**: Use error types from unified client for user-friendly messages.

```jsx
// Wrong: Generic error handling
catch (error) {
  setError(error.message); // May not be user-friendly
}

// Right: Using API client error methods
catch (error) {
  setError(error.toUserMessage()); // Always user-friendly
}
```

### Issue 3: Form Validation Not Working
**Problem**: Validation not triggering after migration to hooks.
**Solution**: Ensure proper rule format and hook integration.

```jsx
// Wrong: Old validation format
const rules = { required: true, type: 'string' };

// Right: New rule format
const rules = [
  { type: 'required', value: true },
  { type: 'type', value: 'string' }
];
```

## Testing After Migration

### Unit Tests
1. Test custom hooks in isolation
2. Verify API client error handling
3. Validate component rendering with new props
4. Check form validation edge cases

### Integration Tests
1. Test complete user workflows
2. Verify Redux state updates correctly
3. Check API client integration
4. Validate SCHEMA-002 compliance

### Manual Testing
1. Check all user interactions still work
2. Verify error messages are user-friendly
3. Test accessibility with screen readers
4. Validate performance improvements

## Performance Considerations

### Optimizations Gained
- **Reduced Re-renders**: Redux prevents unnecessary component updates
- **Request Deduplication**: API client prevents duplicate requests
- **Caching**: Automatic response caching reduces API calls
- **Code Splitting**: Modular components enable better bundling

### Monitoring
- Watch for memory leaks in Redux store
- Monitor API request patterns
- Check component render frequency
- Validate bundle size improvements

## Support Resources

### Documentation
- [Architecture Guide](./architecture.md) - Detailed architectural overview
- [Testing Guide](./testing.md) - Testing strategies and utilities
- [API Documentation](./docs/) - Complete API reference

### Common Patterns
- Custom hooks usage examples
- Component composition patterns
- Error handling best practices
- Performance optimization techniques

### Getting Help
1. Check this migration guide for common issues
2. Review architecture documentation for design patterns
3. Examine test files for usage examples
4. Consult API client documentation for endpoint details

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2025-06-24 | Initial migration guide for React simplification |

For questions or issues during migration, refer to the [architecture.md](./architecture.md) for detailed implementation guidance.