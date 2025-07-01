/**
 * QueryActions Component Tests
 * Tests for UCR-1-6: QueryActions component for execution controls
 * Part of UTC-1 Test Coverage Enhancement - UCR-1 Component Testing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import QueryActions from '../../../components/query/QueryActions';

describe('QueryActions Component', () => {
  let mockProps;
  let user;

  beforeEach(() => {
    user = userEvent.setup();
    mockProps = {
      onExecute: vi.fn(),
      onValidate: vi.fn(),
      onClear: vi.fn(),
      disabled: false,
      showValidation: true,
      showClear: true,
      className: '',
      queryData: {
        schema: 'TestSchema',
        queryFields: ['field1', 'field2'],
        fields: { field1: 'value1', field2: 'value2' }
      }
    };
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render all action buttons when enabled', () => {
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /clear/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /validate/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should hide validation button when showValidation is false', () => {
      mockProps.showValidation = false;
      render(<QueryActions {...mockProps} />);

      expect(screen.queryByRole('button', { name: /validate/i })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /clear/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should hide clear button when showClear is false', () => {
      mockProps.showClear = false;
      render(<QueryActions {...mockProps} />);

      expect(screen.queryByRole('button', { name: /clear/i })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /validate/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should apply custom className', () => {
      mockProps.className = 'custom-class';
      const { container } = render(<QueryActions {...mockProps} />);

      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('button states', () => {
    it('should disable all buttons when disabled prop is true', () => {
      mockProps.disabled = true;
      render(<QueryActions {...mockProps} />);

      const buttons = screen.getAllByRole('button');
      buttons.forEach(button => {
        expect(button).toBeDisabled();
      });
    });

    it('should disable execute and validate buttons when query is invalid', () => {
      mockProps.queryData = null;
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /clear/i })).toBeEnabled();
      expect(screen.getByRole('button', { name: /validate/i })).toBeDisabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeDisabled();
    });

    it('should disable execute and validate buttons when schema is missing', () => {
      mockProps.queryData = { queryFields: ['field1'] };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /validate/i })).toBeDisabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeDisabled();
    });

    it('should disable execute and validate buttons when no fields selected', () => {
      mockProps.queryData = { schema: 'TestSchema', queryFields: [] };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /validate/i })).toBeDisabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeDisabled();
    });
  });

  describe('query validation', () => {
    it('should validate query with queryFields array', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        queryFields: ['field1', 'field2']
      };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should validate query with fields array', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: ['field1', 'field2']
      };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should validate query with fields object', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: { field1: 'value1', field2: 'value2' }
      };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should invalidate query with empty fields object', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: {}
      };
      render(<QueryActions {...mockProps} />);

      expect(screen.getByRole('button', { name: /execute query/i })).toBeDisabled();
    });
  });

  describe('action handling', () => {
    it('should call onExecute with queryData when execute button is clicked', async () => {
      render(<QueryActions {...mockProps} />);

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      expect(mockProps.onExecute).toHaveBeenCalledWith(mockProps.queryData);
    });

    it('should call onValidate with queryData when validate button is clicked', async () => {
      render(<QueryActions {...mockProps} />);

      const validateButton = screen.getByRole('button', { name: /validate/i });
      await user.click(validateButton);

      expect(mockProps.onValidate).toHaveBeenCalledWith(mockProps.queryData);
    });

    it('should call onClear when clear button is clicked', async () => {
      render(<QueryActions {...mockProps} />);

      const clearButton = screen.getByRole('button', { name: /clear/i });
      await user.click(clearButton);

      expect(mockProps.onClear).toHaveBeenCalledWith();
    });

    it('should not call handlers when buttons are disabled', async () => {
      mockProps.disabled = true;
      render(<QueryActions {...mockProps} />);

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      const validateButton = screen.getByRole('button', { name: /validate/i });
      const clearButton = screen.getByRole('button', { name: /clear/i });

      await user.click(executeButton);
      await user.click(validateButton);
      await user.click(clearButton);

      expect(mockProps.onExecute).not.toHaveBeenCalled();
      expect(mockProps.onValidate).not.toHaveBeenCalled();
      expect(mockProps.onClear).not.toHaveBeenCalled();
    });
  });

  describe('loading states', () => {
    it('should show loading spinner on execute button during execution', async () => {
      let executeResolve;
      const executePromise = new Promise(resolve => {
        executeResolve = resolve;
      });
      mockProps.onExecute = vi.fn(() => executePromise);

      render(<QueryActions {...mockProps} />);

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      // Should show loading spinner
      expect(executeButton.querySelector('.animate-spin')).toBeInTheDocument();

      // Resolve the promise
      executeResolve();
      await waitFor(() => {
        expect(executeButton.querySelector('.animate-spin')).not.toBeInTheDocument();
      });
    });

    it('should show loading spinner on validate button during validation', async () => {
      let validateResolve;
      const validatePromise = new Promise(resolve => {
        validateResolve = resolve;
      });
      mockProps.onValidate = vi.fn(() => validatePromise);

      render(<QueryActions {...mockProps} />);

      const validateButton = screen.getByRole('button', { name: /validate/i });
      await user.click(validateButton);

      // Should show loading spinner
      expect(validateButton.querySelector('.animate-spin')).toBeInTheDocument();

      // Resolve the promise
      validateResolve();
      await waitFor(() => {
        expect(validateButton.querySelector('.animate-spin')).not.toBeInTheDocument();
      });
    });

    it('should handle action errors gracefully', async () => {
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockProps.onExecute = vi.fn(() => Promise.reject(new Error('Execute failed')));

      render(<QueryActions {...mockProps} />);

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('execute action failed:', expect.any(Error));
      });

      consoleError.mockRestore();
    });
  });

  describe('optional handlers', () => {
    it('should work when onValidate is not provided', async () => {
      mockProps.onValidate = undefined;
      render(<QueryActions {...mockProps} />);

      // Should not show validate button when onValidate is not provided
      expect(screen.queryByRole('button', { name: /validate/i })).not.toBeInTheDocument();
    });

    it('should work when onClear is not provided', async () => {
      mockProps.onClear = undefined;
      render(<QueryActions {...mockProps} />);

      const clearButton = screen.getByRole('button', { name: /clear/i });
      await user.click(clearButton);

      // Should not throw error
      expect(clearButton).toBeInTheDocument();
    });

    it('should not execute when onExecute is not provided', async () => {
      mockProps.onExecute = undefined;
      render(<QueryActions {...mockProps} />);

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      // Should not throw error
      expect(executeButton).toBeInTheDocument();
    });
  });
});