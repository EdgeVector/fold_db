/**
 * QueryBuilder Component Tests
 * Tests for UCR-1-3: QueryBuilder component with Redux schema integration
 * Part of UTC-1 Test Coverage Enhancement - UCR-1 Component Testing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import QueryBuilder, { useQueryBuilder } from '../../../components/query/QueryBuilder';

// Mock the useQueryBuilder hook for component tests
vi.mock('../../../hooks/useQueryBuilder', () => ({
  useQueryBuilder: vi.fn()
}));

describe('QueryBuilder Component', () => {
  let mockQueryBuilderResult;
  let mockProps;

  beforeEach(() => {
    mockQueryBuilderResult = {
      query: {
        schema: 'TestSchema',
        fields: ['field1', 'field2'],
        filters: []
      },
      validationErrors: [],
      isValid: true,
      buildQuery: vi.fn(),
      validateQuery: vi.fn()
    };

    mockProps = {
      queryState: {
        selectedSchema: 'TestSchema',
        queryFields: ['field1', 'field2'],
        fieldValues: {},
        rangeFilters: {},
        filters: []
      },
      selectedSchemaObj: {
        name: 'TestSchema',
        fields: {
          field1: { field_type: 'String' },
          field2: { field_type: 'Number' }
        }
      },
      isRangeSchema: false,
      rangeKey: null
    };

    // Mock the hook to return our test result
    useQueryBuilder.mockReturnValue(mockQueryBuilderResult);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('render function pattern', () => {
    it('should render with children as render function', () => {
      const mockRenderFunction = vi.fn(() => <div data-testid="query-builder-content">Content</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      expect(mockRenderFunction).toHaveBeenCalledWith(mockQueryBuilderResult);
      expect(screen.getByTestId('query-builder-content')).toBeInTheDocument();
    });

    it('should call useQueryBuilder hook with resolved schema data', () => {
      const mockRenderFunction = vi.fn(() => <div>Content</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      expect(useQueryBuilder).toHaveBeenCalledWith(expect.objectContaining({
        schema: 'TestSchema',
        queryState: mockProps.queryState,
        schemas: { TestSchema: mockProps.selectedSchemaObj },
        selectedSchemaObj: mockProps.selectedSchemaObj,
        isRangeSchema: false,
        rangeKey: null
      }));
    });

    it('should return null when children is not a function', () => {
      const { container } = render(
        <QueryBuilder {...mockProps}>
          <div>Not a function</div>
        </QueryBuilder>
      );

      expect(container.firstChild).toBeNull();
    });

    it('should handle no children gracefully', () => {
      const { container } = render(<QueryBuilder {...mockProps} />);

      expect(container.firstChild).toBeNull();
    });
  });

  describe('hook integration', () => {
    it('should pass query builder result to render function', () => {
      const mockRenderFunction = vi.fn(() => <div>Content</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      const callArgs = mockRenderFunction.mock.calls[0][0];
      expect(callArgs).toEqual(mockQueryBuilderResult);
      expect(callArgs.query).toBeDefined();
      expect(callArgs.validationErrors).toBeDefined();
      expect(callArgs.isValid).toBeDefined();
      expect(callArgs.buildQuery).toBeDefined();
      expect(callArgs.validateQuery).toBeDefined();
    });

    it('should update when hook result changes', () => {
      const mockRenderFunction = vi.fn(() => <div>Content</div>);

      const { rerender } = render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      // Change the mock return value
      const newQueryBuilderResult = {
        ...mockQueryBuilderResult,
        isValid: false,
        validationErrors: ['Schema is required']
      };
      useQueryBuilder.mockReturnValue(newQueryBuilderResult);

      rerender(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      // Should be called twice - once for initial render, once for rerender
      expect(mockRenderFunction).toHaveBeenCalledTimes(2);
      
      // Latest call should have new result
      const latestCallArgs = mockRenderFunction.mock.calls[1][0];
      expect(latestCallArgs.isValid).toBe(false);
      expect(latestCallArgs.validationErrors).toEqual(['Schema is required']);
    });
  });

  describe('practical usage patterns', () => {
    it('should work with typical query building render function', () => {
      const TestComponent = ({ queryBuilder }) => (
        <div data-testid="test-component">
          <div data-testid="query-valid">{queryBuilder.isValid ? 'Valid' : 'Invalid'}</div>
          <div data-testid="error-count">{queryBuilder.validationErrors.length}</div>
          <button onClick={queryBuilder.buildQuery}>Build Query</button>
          <button onClick={queryBuilder.validateQuery}>Validate Query</button>
        </div>
      );

      render(
        <QueryBuilder {...mockProps}>
          {(queryBuilder) => <TestComponent queryBuilder={queryBuilder} />}
        </QueryBuilder>
      );

      expect(screen.getByTestId('test-component')).toBeInTheDocument();
      expect(screen.getByTestId('query-valid')).toHaveTextContent('Valid');
      expect(screen.getByTestId('error-count')).toHaveTextContent('0');
      expect(screen.getByRole('button', { name: /build query/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /validate query/i })).toBeInTheDocument();
    });

    it('should handle invalid query state in render function', () => {
      mockQueryBuilderResult.isValid = false;
      mockQueryBuilderResult.validationErrors = ['Field is required', 'Invalid range'];
      useQueryBuilder.mockReturnValue(mockQueryBuilderResult);

      const TestComponent = ({ queryBuilder }) => (
        <div data-testid="test-component">
          <div data-testid="query-valid">{queryBuilder.isValid ? 'Valid' : 'Invalid'}</div>
          <div data-testid="errors">
            {queryBuilder.validationErrors.map((error, index) => (
              <div key={index} data-testid={`error-${index}`}>{error}</div>
            ))}
          </div>
        </div>
      );

      render(
        <QueryBuilder {...mockProps}>
          {(queryBuilder) => <TestComponent queryBuilder={queryBuilder} />}
        </QueryBuilder>
      );

      expect(screen.getByTestId('query-valid')).toHaveTextContent('Invalid');
      expect(screen.getByTestId('error-0')).toHaveTextContent('Field is required');
      expect(screen.getByTestId('error-1')).toHaveTextContent('Invalid range');
    });
  });

  describe('error handling', () => {
    it('should handle hook returning error state gracefully', () => {
      const errorResult = {
        query: null,
        validationErrors: ['Hook error'],
        isValid: false,
        buildQuery: vi.fn(() => null),
        validateQuery: vi.fn(() => ['Hook error']),
        error: new Error('Hook error'),
      };
      useQueryBuilder.mockReturnValue(errorResult);

      const mockRenderFunction = vi.fn(() => <div data-testid="error-content">Error handled</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      // Should render content
      expect(screen.getByTestId('error-content')).toBeInTheDocument();

      // Should pass error state to render function
      const callArgs = mockRenderFunction.mock.calls[0][0];
      expect(callArgs.isValid).toBe(false);
      expect(callArgs.validationErrors).toContain('Hook error');
      expect(callArgs.error).toBeInstanceOf(Error);
      expect(callArgs.error.message).toBe('Hook error');
    });

    it('should handle undefined hook result', () => {
      useQueryBuilder.mockReturnValue(undefined);

      const mockRenderFunction = vi.fn(() => <div>Content</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      expect(mockRenderFunction).toHaveBeenCalledWith(undefined);
    });

    it('should handle hook returning null gracefully', () => {
      useQueryBuilder.mockReturnValue(null);

      const mockRenderFunction = vi.fn(() => <div>Content</div>);

      render(
        <QueryBuilder {...mockProps}>
          {mockRenderFunction}
        </QueryBuilder>
      );

      expect(mockRenderFunction).toHaveBeenCalledWith(null);
    });
  });

  describe('prop forwarding', () => {
    it('should forward all props to useQueryBuilder hook', () => {
      const complexProps = {
        queryState: {
          selectedSchema: 'ComplexSchema',
          queryFields: ['field1', 'field2', 'field3'],
          fieldValues: { field1: 'value1', field2: 'value2' },
          rangeFilters: { range_field: { start: 'a', end: 'z' } },
          filters: [{ field: 'field1', operator: 'eq', value: 'test' }]
        },
        selectedSchemaObj: {
          name: 'ComplexSchema',
          schema_type: 'Range',
          fields: {
            field1: { field_type: 'String', required: true },
            field2: { field_type: 'Number' },
            field3: { field_type: 'Range' }
          }
        },
        isRangeSchema: true,
        rangeKey: 'field3'
      };

      render(
        <QueryBuilder {...complexProps}>
          {() => <div>Content</div>}
        </QueryBuilder>
      );

      expect(useQueryBuilder).toHaveBeenCalledWith(expect.objectContaining({
        schema: 'ComplexSchema',
        queryState: complexProps.queryState,
        schemas: { ComplexSchema: complexProps.selectedSchemaObj },
        selectedSchemaObj: complexProps.selectedSchemaObj,
        isRangeSchema: true,
        rangeKey: 'field3'
      }));
    });

    it('should not pass children prop to useQueryBuilder hook', () => {
      const propsWithoutChildren = { ...mockProps };

      render(
        <QueryBuilder {...mockProps}>
          {() => <div>Content</div>}
        </QueryBuilder>
      );

      expect(useQueryBuilder).toHaveBeenCalledWith(expect.objectContaining({
        schema: 'TestSchema',
        queryState: propsWithoutChildren.queryState,
        selectedSchemaObj: propsWithoutChildren.selectedSchemaObj
      }));
      expect(useQueryBuilder).not.toHaveBeenCalledWith(
        expect.objectContaining({ children: expect.any(Function) })
      );
    });
  });

  describe('schema resolution', () => {
    it('derives schema name from query state when not provided', () => {
      const props = {
        queryState: {
          selectedSchema: 'DerivedSchema',
          queryFields: [],
          fieldValues: {}
        },
        selectedSchemaObj: {
          name: 'DerivedSchema',
          fields: {
            id: { field_type: 'String' }
          }
        }
      };

      render(
        <QueryBuilder {...props}>
          {() => <div>Content</div>}
        </QueryBuilder>
      );

      expect(useQueryBuilder).toHaveBeenCalledWith(expect.objectContaining({
        schema: 'DerivedSchema',
        selectedSchemaObj: props.selectedSchemaObj,
        schemas: { DerivedSchema: props.selectedSchemaObj }
      }));
    });

    it('derives schema details from provided schemas map when object not given', () => {
      const props = {
        queryState: {
          selectedSchema: 'BlogPost',
          queryFields: [],
          fieldValues: {}
        },
        schemas: {
          BlogPost: {
            name: 'BlogPost',
            schema_type: 'Range',
            key: { range_field: 'publish_date' },
            fields: ['publish_date', 'title', 'content']
          }
        }
      };

      render(
        <QueryBuilder {...props}>
          {() => <div>Content</div>}
        </QueryBuilder>
      );

      expect(useQueryBuilder).toHaveBeenCalledWith(expect.objectContaining({
        schema: 'BlogPost',
        selectedSchemaObj: props.schemas.BlogPost,
        schemas: props.schemas,
        isRangeSchema: true,
        rangeKey: 'publish_date'
      }));
    });
  });
});
