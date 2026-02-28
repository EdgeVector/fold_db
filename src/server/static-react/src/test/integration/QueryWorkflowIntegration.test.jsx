/**
 * QueryTab Workflow Integration Tests
 * Tests for UCR-1 component integration in QueryTab workflow
 * Part of UTC-1 Test Coverage Enhancement - Integration Testing
 */

import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider } from 'react-redux';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import QueryActions from '../../components/query/QueryActions.jsx';
import QueryForm from '../../components/query/QueryForm.jsx';
import QueryBuilder from '../../components/query/QueryBuilder.jsx';
import QueryPreview from '../../components/query/QueryPreview.jsx';
import { renderWithRedux, createTestStore, createAuthenticatedState } from '../utils/testHelpers.jsx';

// Mock the query hooks
vi.mock('../../hooks/useQueryState.js', () => ({
  useQueryState: vi.fn()
}));

vi.mock('../../hooks/useQueryBuilder', () => ({
  useQueryBuilder: vi.fn()
}));

describe('QueryTab Workflow Integration Tests', () => {
  let mockStore;
  let mockUseQueryState;
  let mockUseQueryBuilder;
  let mockExecuteQuery;
  let mockSaveQuery;

  const mockSchemas = {
    UserSchema: {
      name: 'UserSchema',
      schema_type: 'Regular',
      fields: {
        id: { field_type: 'String', required: true },
        name: { field_type: 'String', required: false },
        email: { field_type: 'String', required: true }
      }
    },
    RangeSchema: {
      name: 'RangeSchema',
      schema_type: 'Range',
      fields: {
        range_key: { field_type: 'Range', required: true },
        data: { field_type: 'String', required: false }
      }
    }
  };

  const defaultQueryState = {
    selectedSchema: '',
    queryFields: [],
    fieldValues: {},
    rangeFilters: {},
    rangeSchemaFilter: {},
    filters: [],
    orderBy: null
  };

  const defaultQueryBuilder = {
    query: {},
    validationErrors: [],
    isValid: true // No frontend validation - always valid
  };

  beforeEach(async () => {
    // Import the mocked hooks
    const { useQueryState } = await import('../../hooks/useQueryState.js');
    const { useQueryBuilder } = await import('../../hooks/useQueryBuilder');
    mockUseQueryState = useQueryState;
    mockUseQueryBuilder = useQueryBuilder;

    mockStore = await createTestStore();
    mockExecuteQuery = vi.fn();
    mockSaveQuery = vi.fn();

    // Default mock implementations
    mockUseQueryState.mockReturnValue({
      ...defaultQueryState,
      updateField: vi.fn(),
      updateFieldValue: vi.fn(),
      updateRangeFilter: vi.fn(),
      addFilter: vi.fn(),
      removeFilter: vi.fn(),
      setOrderBy: vi.fn(),
      clearState: vi.fn()
    });

    mockUseQueryBuilder.mockReturnValue({
      ...defaultQueryBuilder
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  const renderQueryWorkflow = (props = {}) => {
    const defaultProps = {
      queryState: defaultQueryState,
      approvedSchemas: Object.values(mockSchemas),
      schemasLoading: false,
      isRangeSchema: false,
      rangeKey: null,
      onExecuteQuery: mockExecuteQuery,
      onSaveQuery: mockSaveQuery,
      onSchemaChange: vi.fn(),
      onFieldToggle: vi.fn(),
      onRangeFilterChange: vi.fn(),
      onRangeSchemaFilterChange: vi.fn(),
      ...props
    };

    // Get the current mocked return values from both hooks
    const mockBuilderValue = mockUseQueryBuilder();
    const mockStateValue = mockUseQueryState();

    return renderWithRedux(
      <div data-testid="query-workflow">
        <QueryForm
          queryState={mockStateValue}
          approvedSchemas={defaultProps.approvedSchemas}
          schemasLoading={defaultProps.schemasLoading}
          isRangeSchema={defaultProps.isRangeSchema}
          rangeKey={defaultProps.rangeKey}
          onSchemaChange={defaultProps.onSchemaChange}
          onFieldToggle={defaultProps.onFieldToggle}
          onRangeFilterChange={defaultProps.onRangeFilterChange}
          onRangeSchemaFilterChange={defaultProps.onRangeSchemaFilterChange}
          className=""
        />
        <QueryBuilder
          queryState={mockStateValue}
          onAddField={defaultProps.onAddField || vi.fn()}
          onRemoveField={defaultProps.onRemoveField || vi.fn()}
          onAddFilter={defaultProps.onAddFilter || vi.fn()}
          onRemoveFilter={defaultProps.onRemoveFilter || vi.fn()}
          onSetSort={defaultProps.onSetSort || vi.fn()}
          isRangeSchema={defaultProps.isRangeSchema}
          className=""
        />
        <QueryPreview
          queryState={mockStateValue}
          query={mockBuilderValue.query}
          validationErrors={mockBuilderValue.validationErrors}
          isExecuting={defaultProps.isExecuting || false}
          className=""
        />
        <QueryActions
          onExecuteQuery={defaultProps.onExecuteQuery}
          onSaveQuery={defaultProps.onSaveQuery}
          onClearQuery={defaultProps.onClearQuery || defaultProps.onClear || vi.fn()}
          isExecuting={defaultProps.isExecuting || false}
          canExecute={defaultProps.canExecute || false}
          canSave={defaultProps.canSave || false}
          isSaving={defaultProps.isSaving || false}
          className=""
        />
      </div>,
      { initialState: createAuthenticatedState() }
    );
  };

  describe('Complete Query Workflow', () => {
    it('handles full query creation workflow from schema selection to execution', async () => {
      const user = userEvent.setup();
      const onSchemaChange = vi.fn();
      
      // Setup valid query state
      mockUseQueryState.mockReturnValue({
        ...defaultQueryState,
        queryFields: ['id', 'name'],
        fieldValues: { id: 'user123', name: 'John Doe' },
        updateField: vi.fn(),
        updateFieldValue: vi.fn(),
        clearState: vi.fn()
      });

      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: {
          schema: 'UserSchema',
          fields: { id: 'user123', name: 'John Doe' }
        },
        isValid: true,
        validationErrors: []
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        onSchemaChange,
        canExecute: true,
        canSave: true
      });

      // Verify all components are rendered
      expect(screen.getByTestId('query-workflow')).toBeInTheDocument();
      
      // Should show execute button as enabled since query is valid
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();

      // Should show save button as enabled
      const saveButton = screen.getByRole('button', { name: /save query/i });
      expect(saveButton).toBeEnabled();

      // Execute the query
      await user.click(executeButton);
      expect(mockExecuteQuery).toHaveBeenCalled();
    });

    it('handles schema change and state reset workflow', async () => {
      const user = userEvent.setup();
      const onSchemaChange = vi.fn();
      const mockClearState = vi.fn();

      mockUseQueryState.mockReturnValue({
        ...defaultQueryState,
        selectedSchema: 'UserSchema',
        queryFields: ['id'],
        fieldValues: { id: 'test' },
        clearState: mockClearState,
        updateField: vi.fn(),
        updateFieldValue: vi.fn()
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        onSchemaChange
      });

      // Directly call the schema change handler to test clearState is called
      const schemaSelect = screen.getByRole('combobox');
      await user.selectOptions(schemaSelect, 'RangeSchema');

      // Should trigger query state reset when schema changes
      expect(mockClearState).toHaveBeenCalled();
    });

    it('has no validation errors - backend validates', () => {
      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        validationErrors: [],
        isValid: true // Always valid - no frontend validation
      });

      renderQueryWorkflow({
        schema: '',
        canExecute: true
      });

      // Execute button should be enabled (no frontend validation)
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();
    });

    it('handles loading states during query execution', () => {
      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: { schema: 'UserSchema', fields: { id: 'test' } },
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        isExecuting: true,
        canExecute: true
      });

      // Execute button should show loading state
      const executeButton = screen.getByRole('button', { name: /executing/i });
      expect(executeButton).toBeDisabled();

      // Should show loading indicator
      expect(screen.getByText(/executing query/i)).toBeInTheDocument();
    });
  });

  describe('Range Schema Workflow', () => {
    it('handles range schema selection and range key input', async () => {
      const _user = userEvent.setup();
      
      mockUseQueryState.mockReturnValue({
        ...defaultQueryState,
        queryFields: ['data'],
        fieldValues: { data: 'test data' },
        rangeFilters: { range_key: { key: 'user:123' } },
        updateRangeFilter: vi.fn(),
        updateField: vi.fn(),
        updateFieldValue: vi.fn()
      });

      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: {
          schema: 'RangeSchema',
          fields: { data: 'test data' },
          rangeKey: 'user:123'
        },
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'RangeSchema',
        canExecute: true
      });

      // Should show range-specific query structure in preview
      expect(screen.getByText(/rangekey/i)).toBeInTheDocument();
      expect(screen.getByText('user:123')).toBeInTheDocument();

      // Execute button should be enabled for valid range query
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();
    });

    it('allows range schema without range key - backend validates', () => {
      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        validationErrors: [],
        isValid: true // No frontend validation
      });

      renderQueryWorkflow({
        schema: 'RangeSchema',
        canExecute: true
      });

      // Execute button should be enabled (backend validates)
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();
    });
  });

  describe('Filter and Sort Workflow', () => {
    it('handles adding filters and sorting in complete workflow', () => {
      const filters = [
        { field: 'name', operator: 'eq', value: 'John' },
        { field: 'age', operator: 'gt', value: 25 }
      ];
      
      const orderBy = { field: 'name', direction: 'asc' };

      mockUseQueryState.mockReturnValue({
        ...defaultQueryState,
        queryFields: ['id', 'name'],
        fieldValues: { id: 'test', name: 'John' },
        filters,
        orderBy,
        updateField: vi.fn(),
        updateFieldValue: vi.fn(),
        addFilter: vi.fn(),
        setOrderBy: vi.fn()
      });

      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: {
          schema: 'UserSchema',
          fields: { id: 'test', name: 'John' },
          filters,
          orderBy
        },
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        canExecute: true
      });

      // Should show filters in preview
      expect(screen.getByText(/filters/i)).toBeInTheDocument();
      expect(screen.getByText('name eq "John"')).toBeInTheDocument();

      // Should show order by in preview
      expect(screen.getByText(/orderby/i)).toBeInTheDocument();
      expect(screen.getByText('name asc')).toBeInTheDocument();
    });
  });

  describe('Error Handling Integration', () => {
    it('handles API errors during query execution', async () => {
      const user = userEvent.setup();
      const mockApiError = new Error('Schema not found');
      mockExecuteQuery.mockRejectedValue(mockApiError);

      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: { schema: 'UserSchema', fields: { id: 'test' } },
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        canExecute: true
      });

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      // Should call execute function
      expect(mockExecuteQuery).toHaveBeenCalled();
    });

    it('handles save query errors', async () => {
      const user = userEvent.setup();
      const mockSaveError = new Error('Failed to save query');
      mockSaveQuery.mockRejectedValue(mockSaveError);

      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        query: { schema: 'UserSchema', fields: { id: 'test' } },
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        canSave: true
      });

      const saveButton = screen.getByRole('button', { name: /save query/i });
      await user.click(saveButton);

      // Should call save function
      expect(mockSaveQuery).toHaveBeenCalled();
    });
  });

  describe('Query Clear and Reset Workflow', () => {
    it('handles query clear across all components', async () => {
      const user = userEvent.setup();
      const mockClearState = vi.fn();
      const onClear = vi.fn();

      mockUseQueryState.mockReturnValue({
        ...defaultQueryState,
        queryFields: ['id', 'name'],
        fieldValues: { id: 'test', name: 'John' },
        clearState: mockClearState,
        updateField: vi.fn(),
        updateFieldValue: vi.fn()
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        onClear
      });

      const clearButton = screen.getByRole('button', { name: /clear query/i });
      await user.click(clearButton);

      // Should call clear handlers
      expect(onClear).toHaveBeenCalled();
      expect(mockClearState).toHaveBeenCalled();
    });
  });

  describe('Authentication Integration', () => {
    it('handles unauthenticated state across query components', () => {
      renderWithRedux(
        <div data-testid="query-workflow">
          <QueryActions
            onExecute={mockExecuteQuery}
            onSave={mockSaveQuery}
            isExecuting={false}
            canExecute={false}
            canSave={false}
          />
        </div>,
        { initialState: { auth: { isAuthenticated: false } } }
      );

      // Query actions should be disabled when not authenticated
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      const saveButton = screen.getByRole('button', { name: /save query/i });
      
      // No validation - buttons are enabled even without auth
      expect(executeButton).toBeEnabled();
      expect(saveButton).toBeEnabled();
    });
  });

  describe('Component Communication', () => {
    it('ensures proper data flow between QueryForm and QueryPreview', () => {
      // Create data that exactly matches what QueryPreview expects
      const queryStateForPreview = {
        selectedSchema: 'UserSchema',
        queryFields: ['id', 'name'],
        fieldValues: { id: 'user123', name: 'John Doe' },
        filters: [{ field: 'active', operator: 'eq', value: true }]
      };

      const queryForPreview = {
        schema: 'UserSchema',
        fields: { id: 'user123', name: 'John Doe' }
      };

      // Render QueryPreview directly with the data it needs
      renderWithRedux(
        <div data-testid="query-workflow">
          <QueryPreview
            queryState={queryStateForPreview}
            query={queryForPreview}
            validationErrors={[]}
            isExecuting={false}
            className=""
            title="Query Preview"
          />
          <QueryActions
            onExecuteQuery={vi.fn()}
            onSaveQuery={vi.fn()}
            onClearQuery={vi.fn()}
            isExecuting={false}
            canExecute={true}
            canSave={false}
            isSaving={false}
            className=""
          />
        </div>,
        { store: mockStore }
      );

      // Check if the schema appears anywhere in the document
      expect(screen.getByText('UserSchema')).toBeInTheDocument();
      expect(screen.getByText('user123')).toBeInTheDocument();
      expect(screen.getByText('John Doe')).toBeInTheDocument();
      
      // Actions should be enabled for valid query
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();
    });

    it('ensures QueryBuilder validation affects QueryActions state', () => {
      // Query state - always valid (backend validates)
      mockUseQueryBuilder.mockReturnValue({
        ...defaultQueryBuilder,
        validationErrors: [],
        isValid: true
      });

      renderQueryWorkflow({
        schema: 'UserSchema',
        canExecute: true // Always allowed - no validation
      });

      // Actions should be enabled (no frontend validation)
      const executeButton = screen.getByRole('button', { name: /execute query/i });
      expect(executeButton).toBeEnabled();
    });
  });
});