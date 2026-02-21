/**
 * QueryPreview Component Tests
 * Tests for UCR-1-5: QueryPreview component for query visualization
 * Part of UTC-1 Test Coverage Enhancement - UCR-1 Component Testing
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import QueryPreview, { formatQueryDisplay } from '../../../components/query/QueryPreview';

describe('QueryPreview Component', () => {
  let mockProps;

  beforeEach(() => {
    mockProps = {
      query: {
        schema: 'UserSchema',
        fields: ['id', 'name', 'email'],
        filter: null
      },
      showJson: false,
      collapsible: true,
      className: '',
      title: 'Query Preview'
    };
  });

  describe('rendering with valid query', () => {
    it('should render query preview with basic query', () => {
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Query Preview')).toBeInTheDocument();
      expect(screen.getByText('Schema')).toBeInTheDocument();
      expect(screen.getByText('UserSchema')).toBeInTheDocument();
      expect(screen.getByText('Fields (3)')).toBeInTheDocument();
      expect(screen.getByText('id')).toBeInTheDocument();
      expect(screen.getByText('name')).toBeInTheDocument();
      expect(screen.getByText('email')).toBeInTheDocument();
    });

    it('should apply custom title', () => {
      mockProps.title = 'Custom Query Preview';
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Custom Query Preview')).toBeInTheDocument();
    });

    it('should apply custom className', () => {
      mockProps.className = 'custom-preview-class';
      const { container } = render(<QueryPreview {...mockProps} />);

      expect(container.firstChild).toHaveClass('custom-preview-class');
    });

    it('should show field count correctly', () => {
      mockProps.query.fields = ['field1', 'field2', 'field3', 'field4', 'field5'];
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Fields (5)')).toBeInTheDocument();
    });
  });

  describe('rendering with null/empty query', () => {
    it('should render empty state when query is null', () => {
      mockProps.query = null;
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Query Preview')).toBeInTheDocument();
      expect(screen.getByText('No query to preview')).toBeInTheDocument();
      expect(screen.queryByText('Schema')).not.toBeInTheDocument();
    });

    it('should render empty state when query is undefined', () => {
      mockProps.query = undefined;
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('No query to preview')).toBeInTheDocument();
    });

    it('should apply custom title in empty state', () => {
      mockProps.query = null;
      mockProps.title = 'Custom Empty Title';
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Custom Empty Title')).toBeInTheDocument();
    });
  });

  describe('filter rendering', () => {
    it('should render range schema filters', () => {
      mockProps.query.filter = {
        range_filter: {
          user_id: {
            KeyRange: {
              start: 'user_001',
              end: 'user_999'
            }
          },
          exact_field: 'exact_value',
          prefix_field: {
            KeyPrefix: 'prefix:'
          }
        }
      };

      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByText('user_id')).toBeInTheDocument();
      expect(screen.getByText('exact_field')).toBeInTheDocument();
      expect(screen.getByText('prefix_field')).toBeInTheDocument();

      expect(screen.getByText('user_001 → user_999')).toBeInTheDocument();
      expect(screen.getByText('exact_value')).toBeInTheDocument();
      expect(screen.getByText('prefix:')).toBeInTheDocument();
    });

    it('should render regular field filters', () => {
      mockProps.query.filter = {
        field: 'age',
        range_filter: {
          KeyRange: {
            start: '18',
            end: '65'
          }
        }
      };

      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByText('age')).toBeInTheDocument();
      expect(screen.getByText('18 → 65')).toBeInTheDocument();
    });

    it('should render exact key filters', () => {
      mockProps.query.filter = {
        field: 'status',
        range_filter: {
          Key: 'active'
        }
      };

      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByText('Exact key:')).toBeInTheDocument();
      expect(screen.getByText('active')).toBeInTheDocument();
    });

    it('should render key prefix filters', () => {
      mockProps.query.filter = {
        field: 'category',
        range_filter: {
          KeyPrefix: 'electronics:'
        }
      };

      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByText('Key prefix:')).toBeInTheDocument();
      expect(screen.getByText('electronics:')).toBeInTheDocument();
    });

    it('should not render filters section when no filters exist', () => {
      render(<QueryPreview {...mockProps} />);

      expect(screen.queryByText('Filters')).not.toBeInTheDocument();
    });
  });

  describe('JSON display', () => {
    it('should show JSON when showJson is true', () => {
      mockProps.showJson = true;
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Raw JSON')).toBeInTheDocument();
      
      // Check for JSON content
      const jsonElement = screen.getByText((content, element) => {
        return element?.tagName === 'PRE' && content.includes('"schema": "UserSchema"');
      });
      expect(jsonElement).toBeInTheDocument();
    });

    it('should not show JSON when showJson is false', () => {
      mockProps.showJson = false;
      render(<QueryPreview {...mockProps} />);

      expect(screen.queryByText('Raw JSON')).not.toBeInTheDocument();
    });

    it('should format JSON correctly', () => {
      mockProps.showJson = true;
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Raw JSON')).toBeInTheDocument();
      const preElement = screen.getByText('Raw JSON').nextElementSibling;
      expect(preElement.tagName).toBe('PRE');
    });
  });

  describe('edge cases', () => {
    it('should handle empty fields array', () => {
      mockProps.query.fields = [];
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Fields (0)')).toBeInTheDocument();
    });

    it('should handle query without fields property', () => {
      delete mockProps.query.fields;
      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('Fields (0)')).toBeInTheDocument();
    });

    it('should handle complex nested filters', () => {
      mockProps.query.filter = {
        range_filter: {
          'complex.field': {
            KeyRange: {
              start: 'a',
              end: 'z'
            }
          }
        }
      };

      render(<QueryPreview {...mockProps} />);

      expect(screen.getByText('complex.field')).toBeInTheDocument();
      expect(screen.getByText('a → z')).toBeInTheDocument();
    });

    it('should handle malformed filter objects gracefully', () => {
      mockProps.query.filter = {
        range_filter: {
          malformed_field: {
            // Missing required properties
          }
        }
      };

      // Should not crash
      render(<QueryPreview {...mockProps} />);
      expect(screen.getByText('Query Preview')).toBeInTheDocument();
    });
  });
});

describe('formatQueryDisplay utility function', () => {
  it('should return null for null query', () => {
    const result = formatQueryDisplay(null);
    expect(result).toBeNull();
  });

  it('should return null for undefined query', () => {
    const result = formatQueryDisplay(undefined);
    expect(result).toBeNull();
  });

  it('should format basic query correctly', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1', 'field2']
    };

    const result = formatQueryDisplay(query);
    
    expect(result).toEqual({
      schema: 'TestSchema',
      fields: ['field1', 'field2'],
      filters: {},
      fieldValues: {},
      orderBy: undefined,
      rangeKey: undefined
    });
  });

  it('should handle missing fields property', () => {
    const query = {
      schema: 'TestSchema'
    };

    const result = formatQueryDisplay(query);
    
    expect(result).toEqual({
      schema: 'TestSchema',
      fields: [],
      filters: {},
      fieldValues: {},
      orderBy: undefined,
      rangeKey: undefined
    });
  });

  it('should format range schema filters correctly', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1'],
      filter: {
        range_filter: {
          user_id: {
            KeyRange: { start: 'a', end: 'z' }
          },
          exact_field: 'exact_value',
          prefix_field: {
            KeyPrefix: 'prefix:'
          }
        }
      }
    };

    const result = formatQueryDisplay(query);
    
    expect(result.filters).toEqual({
      user_id: { keyRange: 'a → z' },
      exact_field: { exactKey: 'exact_value' },
      prefix_field: { keyPrefix: 'prefix:' }
    });
  });

  it('should format regular field filters correctly', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1'],
      filter: {
        field: 'age',
        range_filter: {
          Key: 'exact_value'
        }
      }
    };

    const result = formatQueryDisplay(query);
    
    expect(result.filters).toEqual({
      age: { exactKey: 'exact_value' }
    });
  });

  it('should handle filters with KeyRange in regular fields', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1'],
      filter: {
        field: 'range_field',
        range_filter: {
          KeyRange: { start: '1', end: '10' }
        }
      }
    };

    const result = formatQueryDisplay(query);
    
    expect(result.filters).toEqual({
      range_field: { keyRange: '1 → 10' }
    });
  });

  it('should handle filters with KeyPrefix in regular fields', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1'],
      filter: {
        field: 'category',
        range_filter: {
          KeyPrefix: 'electronics:'
        }
      }
    };

    const result = formatQueryDisplay(query);
    
    expect(result.filters).toEqual({
      category: { keyPrefix: 'electronics:' }
    });
  });

  it('should handle empty filter objects', () => {
    const query = {
      schema: 'TestSchema',
      fields: ['field1'],
      filter: {}
    };

    const result = formatQueryDisplay(query);
    
    expect(result.filters).toEqual({});
  });
});