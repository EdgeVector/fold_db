/**
 * @fileoverview Tests for ResultsSection component
 * 
 * Tests the ResultsSection component including result display,
 * error handling, and different data types.
 */

import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ResultsSection from '../../components/ResultsSection.jsx';

describe('ResultsSection Component', () => {
  it('returns null when no results provided', () => {
    const { container } = render(<ResultsSection results={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('returns null when results is undefined', () => {
    const { container } = render(<ResultsSection />);
    expect(container.firstChild).toBeNull();
  });

  it('renders successful results with correct structure', () => {
    const mockResults = {
      data: { users: [{ id: 1, name: 'John' }] },
      status: 200
    };

    render(<ResultsSection results={mockResults} />);
    
    expect(screen.getByText('Results')).toBeInTheDocument();
    expect(screen.getByText('(JSON)')).toBeInTheDocument();
    expect(screen.getByText('Status: 200')).toBeInTheDocument();
  });

  it('renders error results with correct styling', () => {
    const mockErrorResults = {
      error: 'Database connection failed',
      status: 500
    };

    render(<ResultsSection results={mockErrorResults} />);
    
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('(JSON)')).toBeInTheDocument();
    expect(screen.getByText('Status: 500')).toBeInTheDocument();
    expect(screen.getByText('Query Execution Failed')).toBeInTheDocument();
    expect(screen.getByText('Database connection failed')).toBeInTheDocument();
  });

  it('renders string results correctly', () => {
    const stringResults = 'Simple text result';

    render(<ResultsSection results={stringResults} />);
    
    expect(screen.getByText('Results')).toBeInTheDocument();
    expect(screen.getByText('(Text)')).toBeInTheDocument();
    expect(screen.getByText('Simple text result')).toBeInTheDocument();
  });

  it('handles results with status 400+ as errors', () => {
    const errorResults = {
      data: null,
      status: 404
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('Status: 404')).toBeInTheDocument();
    expect(screen.getByText('Query Execution Failed')).toBeInTheDocument();
  });

  it('handles results with error property as errors', () => {
    const errorResults = {
      error: 'Validation failed',
      status: 200 // Even with 200 status, error property makes it an error
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('Query Execution Failed')).toBeInTheDocument();
    expect(screen.getByText('Validation failed')).toBeInTheDocument();
  });

  it('shows unknown error message when error property is missing', () => {
    const errorResults = {
      status: 500
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('Query Execution Failed')).toBeInTheDocument();
    expect(screen.getByText('An unknown error occurred')).toBeInTheDocument();
  });

  it('displays JSON data correctly formatted', () => {
    const mockResults = {
      data: { 
        users: [
          { id: 1, name: 'John', email: 'john@example.com' },
          { id: 2, name: 'Jane', email: 'jane@example.com' }
        ]
      }
    };

    render(<ResultsSection results={mockResults} />);
    
    const preElement = screen.getByText((content, element) => {
      return element?.tagName === 'PRE' && content.includes('"users"');
    });
    expect(preElement).toHaveTextContent('"users"');
    expect(preElement).toHaveTextContent('"id": 1');
    expect(preElement).toHaveTextContent('"name": "John"');
  });

  it('renders structured view toggle and switches modes when hash-range shape is detected', () => {
    const hr = {
      status: 200,
      data: {
        H1: { R1: { a: 1 } }
      }
    };

    const { getByText, getByRole } = render(<ResultsSection results={hr} />);
    // Header should reflect Structured
    expect(getByText('(Structured)')).toBeInTheDocument();
    // Toggle to JSON
    const toggle = getByRole('button', { name: /View JSON/ });
    fireEvent.click(toggle);
    expect(screen.getByText((content, element) => element?.textContent === '(JSON)')).toBeInTheDocument();
  });

  it('displays results without data property correctly', () => {
    const mockResults = {
      message: 'Success',
      count: 42
    };

    render(<ResultsSection results={mockResults} />);
    
    const preElement = screen.getByText((content, element) => {
      return element?.tagName === 'PRE' && content.includes('"message": "Success"');
    });
    expect(preElement).toHaveTextContent('"message": "Success"');
    expect(preElement).toHaveTextContent('"count": 42');
  });

  it('applies correct CSS classes for success results', () => {
    const mockResults = {
      data: { success: true },
      status: 200
    };

    render(<ResultsSection results={mockResults} />);
    
    const resultsTitle = screen.getByText('Results');
    expect(resultsTitle).toHaveClass('text-gray-900');
    
    const statusBadge = screen.getByText('Status: 200');
    expect(statusBadge).toHaveClass('bg-green-100', 'text-green-800');
  });

  it('applies correct CSS classes for error results', () => {
    const mockResults = {
      error: 'Test error',
      status: 500
    };

    render(<ResultsSection results={mockResults} />);
    
    const errorTitle = screen.getByText('Error');
    expect(errorTitle).toHaveClass('text-red-600');
    
    const statusBadge = screen.getByText('Status: 500');
    expect(statusBadge).toHaveClass('bg-red-100', 'text-red-800');
  });

  it('renders without crashing with complex nested data', () => {
    const complexResults = {
      data: {
        nested: {
          array: [1, 2, 3],
          object: { key: 'value' }
        }
      }
    };

    expect(() => render(<ResultsSection results={complexResults} />)).not.toThrow();
  });

  it('handles empty data gracefully', () => {
    const emptyResults = {
      data: null
    };

    render(<ResultsSection results={emptyResults} />);
    
    expect(screen.getByText('Results')).toBeInTheDocument();
    expect(screen.getByText('(JSON)')).toBeInTheDocument();
  });

  it('has correct container structure and classes', () => {
    const mockResults = { data: { test: true } };
    
    render(<ResultsSection results={mockResults} />);
    
    const container = screen.getByText('Results').closest('.bg-white.rounded-lg.shadow-sm.p-6.mt-6');
    expect(container).toBeInTheDocument();
  });
});
