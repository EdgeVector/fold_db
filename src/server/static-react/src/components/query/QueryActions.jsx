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
          className={`btn-terminal px-4 py-2 text-sm font-medium ${
            disabled ? 'opacity-50 cursor-not-allowed' : ''
          }`}
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
          className={`btn-terminal px-4 py-2 text-sm font-medium ${
            disabled ? 'opacity-50 cursor-not-allowed' : 'btn-terminal-primary'
          }`}
        >
          {loadingAction === 'validate' && (
            <span className="spinner-terminal"></span>
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
          className={`btn-terminal px-4 py-2 text-sm font-medium ${
            disabled || isSaving ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        >
          {(loadingAction === 'save' || isSaving) && (
            <span className="spinner-terminal"></span>
          )}
          {BUTTON_TEXT.saveQuery || 'Save Query'}
        </button>
      )}

      {/* Execute Button */}
      <button
        type="button"
        onClick={handleExecute}
        disabled={disabled || isExecuting}
        className={`btn-terminal px-6 py-2 text-sm font-medium ${
          disabled || isExecuting ? 'opacity-50 cursor-not-allowed' : 'btn-terminal-primary'
        }`}
      >
        {(loadingAction === 'execute' || isExecuting) && (
          <span className="spinner-terminal"></span>
        )}
        {(loadingAction === 'execute' || isExecuting)
          ? 'Executing...'
          : (BUTTON_TEXT.executeQuery || '→ Execute Query')}
      </button>
    </div>
  );
}

export default QueryActions;