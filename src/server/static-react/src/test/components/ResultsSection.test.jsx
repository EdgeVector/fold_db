/**
 * @fileoverview Tests for ResultsSection component
 * 
 * Tests the ResultsSection component including result display,
 * error handling, and different data types.
 * Updated for terminal theme styling.
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

  it('renders successful results with terminal structure', () => {
    const mockResults = {
      data: { users: [{ id: 1, name: 'John' }] },
      status: 200
    };

    render(<ResultsSection results={mockResults} />);
    
    // Terminal-styled output header
    expect(screen.getByText('OUTPUT')).toBeInTheDocument();
    // Multiple elements may contain 'json' text
    expect(screen.getAllByText(/json/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/200/).length).toBeGreaterThan(0);
  });

  it('renders error results with error styling', () => {
    const mockErrorResults = {
      error: 'Database connection failed',
      status: 500
    };

    render(<ResultsSection results={mockErrorResults} />);
    
    // Error styling should be visible
    expect(screen.getByText('ERROR')).toBeInTheDocument();
    // Multiple elements may contain status text
    expect(screen.getAllByText(/500/).length).toBeGreaterThan(0);
    expect(screen.getByText('Database connection failed')).toBeInTheDocument();
  });

  it('renders string results correctly', () => {
    const stringResults = 'Simple text result';

    render(<ResultsSection results={stringResults} />);
    
    expect(screen.getByText('OUTPUT')).toBeInTheDocument();
    expect(screen.getByText('Simple text result')).toBeInTheDocument();
  });

  it('handles results with status 400+ as errors', () => {
    const errorResults = {
      data: null,
      status: 404
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('ERROR')).toBeInTheDocument();
    expect(screen.getByText(/404/)).toBeInTheDocument();
  });

  it('handles results with error property as errors', () => {
    const errorResults = {
      error: 'Validation failed',
      status: 200 // Even with 200 status, error property makes it an error
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('ERROR')).toBeInTheDocument();
    expect(screen.getByText('Validation failed')).toBeInTheDocument();
  });

  it('shows unknown error message when error property is missing', () => {
    const errorResults = {
      status: 500
    };

    render(<ResultsSection results={errorResults} />);
    
    expect(screen.getByText('ERROR')).toBeInTheDocument();
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

    render(<ResultsSection results={hr} />);
    // Hash-range structure starts in structured view, button shows option to view JSON
    const toggleButton = screen.getByRole('button', { name: /view json/i });
    expect(toggleButton).toBeInTheDocument();
    
    // Click to switch view to JSON
    fireEvent.click(toggleButton);
    // After toggle, should show option to view structured
    expect(screen.getByRole('button', { name: /view structured/i })).toBeInTheDocument();
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

  it('renders output header for success results', () => {
    const mockResults = {
      data: { success: true },
      status: 200
    };

    render(<ResultsSection results={mockResults} />);
    
    expect(screen.getByText('OUTPUT')).toBeInTheDocument();
  });

  it('renders error header for error results', () => {
    const mockResults = {
      error: 'Test error',
      status: 500
    };

    render(<ResultsSection results={mockResults} />);
    
    expect(screen.getByText('ERROR')).toBeInTheDocument();
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
    
    // Should still render output container
    expect(screen.getByText('OUTPUT')).toBeInTheDocument();
  });

  it('renders terminal-styled container', () => {
    const mockResults = { data: { test: true } };

    render(<ResultsSection results={mockResults} />);

    // Check the container uses card classes
    const outputTitle = screen.getByText('OUTPUT');
    expect(outputTitle.closest('.card')).toBeInTheDocument();
  });
});
