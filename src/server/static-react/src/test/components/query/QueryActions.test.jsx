/**
 * QueryActions Component Tests
 * Tests for UCR-1-6: QueryActions component for execution controls
 * Part of UTC-1 Test Coverage Enhancement - UCR-1 Component Testing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import QueryActions from '../../../components/query/QueryActions';
import { renderWithRedux, createAuthenticatedState } from '../../utils/testHelpers';

describe('QueryActions Component', () => {
  let mockProps;
  let user;
  let initialState;

  beforeEach(() => {
    user = userEvent.setup();
    initialState = createAuthenticatedState();
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
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /clear/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /validate/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should hide validation button when showValidation is false', () => {
      mockProps.showValidation = false;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.queryByRole('button', { name: /validate/i })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /clear/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should hide clear button when showClear is false', () => {
      mockProps.showClear = false;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.queryByRole('button', { name: /clear/i })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /validate/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeInTheDocument();
    });

    it('should apply custom className', () => {
      mockProps.className = 'custom-class';
      const { container } = renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('button states', () => {
    it('should disable all buttons when disabled prop is true', () => {
      mockProps.disabled = true;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const buttons = screen.getAllByRole('button');
      buttons.forEach(button => {
        expect(button).toBeDisabled();
      });
    });

    it('should keep buttons enabled even when query is null (no validation)', () => {
      mockProps.queryData = null;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /clear/i })).toBeEnabled();
      expect(screen.getByRole('button', { name: /validate/i })).toBeEnabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should keep buttons enabled even when schema is missing (no validation)', () => {
      mockProps.queryData = { queryFields: ['field1'] };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /validate/i })).toBeEnabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should keep buttons enabled even when no fields selected (no validation)', () => {
      mockProps.queryData = { schema: 'TestSchema', queryFields: [] };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /validate/i })).toBeEnabled();
      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });
  });

  describe('query validation', () => {
    it('should validate query with queryFields array', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        queryFields: ['field1', 'field2']
      };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should validate query with fields array', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: ['field1', 'field2']
      };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should validate query with fields object', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: { field1: 'value1', field2: 'value2' }
      };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });

    it('should keep buttons enabled even with empty fields object (no validation)', () => {
      mockProps.queryData = {
        schema: 'TestSchema',
        fields: {}
      };
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      expect(screen.getByRole('button', { name: /execute query/i })).toBeEnabled();
    });
  });

  describe('action handling', () => {
    it('should call onExecute with queryData when execute button is clicked', async () => {
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      expect(mockProps.onExecute).toHaveBeenCalledWith(mockProps.queryData);
    });

    it('should call onValidate with queryData when validate button is clicked', async () => {
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const validateButton = screen.getByRole('button', { name: /validate/i });
      await user.click(validateButton);

      expect(mockProps.onValidate).toHaveBeenCalledWith(mockProps.queryData);
    });

    it('should call onClear when clear button is clicked', async () => {
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const clearButton = screen.getByRole('button', { name: /clear/i });
      await user.click(clearButton);

      expect(mockProps.onClear).toHaveBeenCalledWith();
    });

    it('should not call handlers when buttons are disabled', async () => {
      mockProps.disabled = true;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

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

      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      // Should show loading spinner
      expect(executeButton.querySelector('.spinner')).toBeInTheDocument();

      // Resolve the promise
      executeResolve();
      await waitFor(() => {
        expect(executeButton.querySelector('.spinner')).not.toBeInTheDocument();
      });
    });

    it('should show loading spinner on validate button during validation', async () => {
      let validateResolve;
      const validatePromise = new Promise(resolve => {
        validateResolve = resolve;
      });
      mockProps.onValidate = vi.fn(() => validatePromise);

      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const validateButton = screen.getByRole('button', { name: /validate/i });
      await user.click(validateButton);

      // Should show loading spinner
      expect(validateButton.querySelector('.spinner')).toBeInTheDocument();

      // Resolve the promise
      validateResolve();
      await waitFor(() => {
        expect(validateButton.querySelector('.spinner')).not.toBeInTheDocument();
      });
    });

    it('should handle action errors gracefully', async () => {
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockProps.onExecute = vi.fn(() => Promise.reject(new Error('Execute failed')));

      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

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
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      // Should not show validate button when onValidate is not provided
      expect(screen.queryByRole('button', { name: /validate/i })).not.toBeInTheDocument();
    });

    it('should work when onClear is not provided', async () => {
      mockProps.onClear = undefined;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const clearButton = screen.getByRole('button', { name: /clear/i });
      await user.click(clearButton);

      // Should not throw error
      expect(clearButton).toBeInTheDocument();
    });

    it('should not execute when onExecute is not provided', async () => {
      mockProps.onExecute = undefined;
      renderWithRedux(<QueryActions {...mockProps} />, { initialState });

      const executeButton = screen.getByRole('button', { name: /execute query/i });
      await user.click(executeButton);

      // Should not throw error
      expect(executeButton).toBeInTheDocument();
    });
  });
});