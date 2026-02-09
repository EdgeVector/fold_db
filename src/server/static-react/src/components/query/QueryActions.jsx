/**
 * QueryActions Component
 * Provides execute, validate, and clear actions for query management
 * Part of UCR-1-6: Create QueryActions component for execution controls
 * Follows established action component patterns
 */

import { useState } from 'react';
import { useQueryState } from '../../hooks/useQueryState.js';
import {
  BUTTON_TEXT,
  UI_STATES
} from '../../constants/ui.js';
import { COMPONENT_STYLES } from '../../constants/styling.js';

/**
 * @typedef {Object} QueryActionsProps
 * @property {function} onExecute - Execute query callback (queryData) => Promise<void>
 * @property {function} [onValidate] - Validate query callback (queryData) => Promise<void>
 * @property {function} [onSave] - Save query callback (queryData) => Promise<void>
 * @property {function} [onClear] - Clear query callback () => void
 * @property {boolean} [disabled] - Whether actions are disabled
 * @property {boolean} [showValidation] - Whether to show validation button
 * @property {boolean} [showSave] - Whether to show save button
 * @property {boolean} [showClear] - Whether to show clear button
 * @property {string} [className] - Additional CSS classes
 * @property {Object} queryData - Current query data for validation
 */

/**
 * Query execution actions component following established patterns
 * 
 * @param {QueryActionsProps} props
 * @returns {JSX.Element}
 */
function QueryActions({
  onExecute,
  onExecuteQuery,
  onValidate,
  onSave,
  onSaveQuery,
  onClear,
  onClearQuery,
  disabled = false,
  isExecuting = false,
  isSaving = false,
  showValidation = false,
  showSave = true,
  showClear = true,
  className = '',
  queryData
}) {
  const [loadingAction, setLoadingAction] = useState(null);
  const [_confirmAction, setConfirmAction] = useState(null);
  const { clearQuery } = useQueryState();

  /**
   * Handle action execution with loading state
   * Follows established handleAction pattern
   */
  const handleAction = async (action, actionFn, data = null) => {
    if (!actionFn || disabled) return;

    try {
      setLoadingAction(action);
      await actionFn(data);
    } catch (error) {
      console.error(`${action} action failed:`, error);
    } finally {
      setLoadingAction(null);
      setConfirmAction(null);
    }
  };

  /**
   * Handle execute action
   */
  const handleExecute = () => {
    const executeHandler = onExecuteQuery || onExecute;
    handleAction('execute', executeHandler, queryData);
  };

  /**
   * Handle validate action
   */
  const handleValidate = () => {
    handleAction('validate', onValidate, queryData);
  };

  /**
   * Handle save action
   */
  const handleSave = () => {
    const saveHandler = onSaveQuery || onSave;
    handleAction('save', saveHandler, queryData);
  };

  /**
   * Handle clear action
   */
  const handleClear = () => {
    const clearHandler = onClearQuery || onClear;
    if (clearHandler) {
      clearHandler();
    }
    if (clearQuery) {
      clearQuery();
    }
  };

  return (
    <div className={`flex justify-end space-x-3 ${className}`}>
      {/* Clear Button */}
      {showClear && (
        <button
          type="button"
          onClick={handleClear}
          disabled={disabled}
          className={`
            inline-flex items-center px-4 py-2 border text-sm font-medium
            ${disabled
              ? 'border-gray-200 text-gray-400 cursor-not-allowed bg-white'
              : 'border-gray-300 text-gray-700 bg-white hover:border-gray-900 hover:text-gray-900'
            }
          `}
        >
          {BUTTON_TEXT.clearQuery || 'Clear Query'}
        </button>
      )}

      {/* Validate Button */}
      {showValidation && onValidate && (
        <button
          type="button"
          onClick={handleValidate}
          disabled={disabled}
          className={`
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${disabled
              ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
              : 'bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500'
            }
          `}
        >
          {loadingAction === 'validate' && (
            <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          )}
          {BUTTON_TEXT.validateQuery || 'Validate'}
        </button>
      )}

      {/* Save Button */}
      {showSave && (onSave || onSaveQuery) && (
        <button
          type="button"
          onClick={handleSave}
          disabled={disabled || isSaving}
          className={`
            inline-flex items-center px-4 py-2 border text-sm font-medium
            ${disabled || isSaving
              ? 'border-gray-200 text-gray-400 cursor-not-allowed bg-white'
              : 'border-gray-300 text-gray-700 bg-white hover:border-gray-900 hover:text-gray-900'
            }
          `}
        >
          {(loadingAction === 'save' || isSaving) && (
            <svg className="animate-spin -ml-1 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          )}
          {BUTTON_TEXT.saveQuery || 'Save Query'}
        </button>
      )}

      {/* Execute Button */}
      <button
        type="button"
        onClick={handleExecute}
        disabled={disabled || isExecuting}
        className={`
          inline-flex items-center px-4 py-2 border text-sm font-medium
          ${disabled || isExecuting
            ? 'border-gray-200 bg-gray-100 text-gray-400 cursor-not-allowed'
            : 'border-gray-900 bg-gray-900 text-white hover:bg-gray-700'
          }
        `}
      >
        {(loadingAction === 'execute' || isExecuting) && (
          <svg className="animate-spin -ml-1 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        )}
        {(loadingAction === 'execute' || isExecuting)
          ? 'Executing...'
          : (BUTTON_TEXT.executeQuery || 'Execute Query')}
      </button>
    </div>
  );
}

export default QueryActions;