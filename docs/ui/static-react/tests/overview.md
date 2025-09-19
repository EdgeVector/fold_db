# UI Testing Documentation

This directory contains comprehensive UI tests for the DataFold Node React application using Vitest and React Testing Library.

## Test Structure

### Test Categories

1. **Unit Tests** (`src/test/components/`)
   - Individual component testing
   - Component behavior and rendering
   - Props handling and state management

2. **Integration Tests** (`src/test/integration/`)
   - Full application workflow testing
   - Component interaction testing
   - API integration testing


4. **Utility Tests** (`tests/`)
   - Pure function testing
   - Helper utility testing

### Test Files

- `App.test.jsx` - Main application component tests
- `components/Header.test.jsx` - Header component tests
- `components/StatusSection.test.jsx` - Status display tests
- `components/tabs/SchemaTab.test.jsx` - Schema management tests
- `integration/AppIntegration.test.jsx` - End-to-end workflow tests
- `accessibility/AccessibilityTests.test.jsx` - Accessibility compliance tests
- `../tests/dependencyUtils.test.js` - Utility function tests

## Testing Framework

### Technologies Used

- **Vitest** - Fast unit test framework
- **React Testing Library** - React component testing utilities
- **@testing-library/user-event** - User interaction simulation
- **@testing-library/jest-dom** - Custom Jest matchers
- **jsdom** - DOM environment for testing

### Configuration

- `vitest.config.js` - Vitest configuration
- `src/test/setup.js` - Global test setup and mocks

## Running Tests

```bash
# Run all tests
npm test

# Run tests with UI
npm run test:ui

# Run tests with coverage
npm run test:coverage

# Run specific test file
npm test -- App.test.jsx

# Run tests in watch mode
npm test -- --watch
```

## Test Patterns

### Component Testing

```javascript
import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import Component from '../Component'

describe('Component', () => {
  it('renders correctly', () => {
    render(<Component />)
    expect(screen.getByText('Expected Text')).toBeInTheDocument()
  })
})
```

### User Interaction Testing

```javascript
import userEvent from '@testing-library/user-event'

it('handles user interactions', async () => {
  const user = userEvent.setup()
  render(<Component />)
  
  await user.click(screen.getByText('Button'))
  expect(screen.getByText('Result')).toBeInTheDocument()
})
```

### API Mocking

```javascript
beforeEach(() => {
  fetch.mockResolvedValue({
    ok: true,
    json: async () => ({ data: [] })
  })
})
```

## Test Coverage

The test suite covers:

- ✅ Component rendering and styling
- ✅ User interactions (clicks, form inputs, navigation)
- ✅ API integration and error handling
- ✅ State management and data flow
- ✅ Accessibility compliance
- ✅ Loading states and error states
- ✅ Tab navigation and routing
- ✅ Schema management workflows
- ✅ Sample data loading
- ✅ Responsive design elements

## Best Practices

### Writing Tests

1. **Test behavior, not implementation**
   - Focus on what the user sees and does
   - Avoid testing internal component state directly

2. **Use semantic queries**
   - Prefer `getByRole`, `getByLabelText`, `getByText`
   - Avoid `getByTestId` unless necessary

3. **Mock external dependencies**
   - Mock API calls with realistic responses
   - Mock complex child components when testing parent logic

4. **Test error states**
   - Verify graceful error handling
   - Test loading states and edge cases

### Accessibility Testing

1. **Semantic HTML**
   - Test for proper heading hierarchy
   - Verify landmark roles (banner, main, etc.)

2. **Keyboard Navigation**
   - Test tab order and focus management
   - Verify keyboard shortcuts work

3. **Screen Reader Support**
   - Test ARIA attributes
   - Verify meaningful text content

## Local Development

Tests are designed for local development:

- Fast execution with parallel test running
- Comprehensive coverage reporting
- Clear failure messages and debugging info
- No external dependencies required

## Debugging Tests

### Common Issues

1. **Async operations not awaited**
   ```javascript
   // Wrong
   fireEvent.click(button)
   expect(screen.getByText('Result')).toBeInTheDocument()
   
   // Correct
   fireEvent.click(button)
   await waitFor(() => {
     expect(screen.getByText('Result')).toBeInTheDocument()
   })
   ```

2. **Missing mocks**
   ```javascript
   // Mock fetch before using components that make API calls
   fetch.mockResolvedValue({ ok: true, json: async () => ({}) })
   ```

3. **Component not found**
   ```javascript
   // Use screen.debug() to see current DOM
   render(<Component />)
   screen.debug()
   ```

### Debugging Commands

```bash
# Run single test with verbose output
npm test -- --reporter=verbose App.test.jsx

# Run tests with DOM debugging
npm test -- --reporter=verbose --no-coverage

# Generate coverage report
npm run test:coverage
```

## Future Enhancements

Potential improvements to the test suite:

1. **Visual Regression Testing**
   - Screenshot comparison tests
   - CSS regression detection

2. **Performance Testing**
   - Component render time testing
   - Memory leak detection

3. **E2E Testing**
   - Full browser automation with Playwright
   - Cross-browser compatibility testing

4. **API Contract Testing**
   - Schema validation testing
   - API response format verification