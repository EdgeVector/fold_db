/**
 * SchemaActions Component
 * Provides approve, block, and unload actions for schema management
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState } from 'react';
import { 
  BUTTON_TEXT, 
  COMPONENT_STYLES,
  UI_STATES 
} from '../../constants/ui.js';

/**
 * @typedef {Object} Schema
 * @property {string} name - Schema name
 * @property {string} state - Schema state (approved, available, blocked)
 * @property {Object} [fields] - Schema fields
 */

/**
 * @typedef {Object} SchemaActionsProps
 * @property {Schema} schema - Schema object
 * @property {function} onApprove - Approve action callback (name) => Promise<void>
 * @property {function} onBlock - Block action callback (name) => Promise<void>
 * @property {function} onUnload - Unload action callback (name) => Promise<void>
 * @property {boolean} [disabled] - Whether actions are disabled
 * @property {boolean} [showConfirmation] - Whether to show confirmation dialogs
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Schema management actions component with SCHEMA-002 compliance
 * 
 * @param {SchemaActionsProps} props
 * @returns {JSX.Element}
 */
function SchemaActions({
  schema,
  onApprove,
  onBlock,
  onUnload,
  disabled = false,
  showConfirmation = true,
  className = ''
}) {
  const [loadingAction, setLoadingAction] = useState(null);
  const [confirmAction, setConfirmAction] = useState(null);

  const handleAction = async (action, actionFn) => {
    if (disabled) return;

    // Show confirmation dialog if enabled
    if (showConfirmation) {
      setConfirmAction(action);
      return;
    }

    await executeAction(action, actionFn);
  };

  const executeAction = async (action, actionFn) => {
    setLoadingAction(action);
    setConfirmAction(null);

    try {
      await actionFn(schema.name);
    } catch (error) {
      console.error(`Failed to ${action} schema:`, error);
      // Error handling should be done by parent component
    } finally {
      setLoadingAction(null);
    }
  };

  const confirmAndExecute = async (action) => {
    const actionMap = {
      approve: () => executeAction('approve', onApprove),
      block: () => executeAction('block', onBlock),
      unload: () => executeAction('unload', onUnload)
    };

    if (actionMap[action]) {
      await actionMap[action]();
    }
  };

  const cancelConfirmation = () => {
    setConfirmAction(null);
  };

  // Determine which actions are available based on schema state
  const getAvailableActions = () => {
    const state = schema.state?.toLowerCase() || 'available';
    
    switch (state) {
      case 'approved':
        return [
          { key: 'block', label: BUTTON_TEXT.block, style: 'danger', fn: onBlock },
          { key: 'unload', label: BUTTON_TEXT.unload, style: 'secondary', fn: onUnload }
        ];
      case 'blocked':
        return [
          { key: 'approve', label: BUTTON_TEXT.approve, style: 'primary', fn: onApprove },
          { key: 'unload', label: BUTTON_TEXT.unload, style: 'secondary', fn: onUnload }
        ];
      case 'available':
      default:
        return [
          { key: 'approve', label: BUTTON_TEXT.approve, style: 'primary', fn: onApprove },
          { key: 'block', label: BUTTON_TEXT.block, style: 'danger', fn: onBlock }
        ];
    }
  };

  const getButtonStyles = (style, isLoading, isDisabled) => {
    const baseClasses = 'inline-flex items-center px-3 py-1.5 text-xs font-medium rounded-md transition-colors duration-200';
    
    if (isDisabled || isLoading) {
      return `${baseClasses} ${COMPONENT_STYLES.button.disabled}`;
    }
    
    switch (style) {
      case 'primary':
        return `${baseClasses} ${COMPONENT_STYLES.button.primary}`;
      case 'danger':
        return `${baseClasses} ${COMPONENT_STYLES.button.danger}`;
      case 'secondary':
      default:
        return `${baseClasses} ${COMPONENT_STYLES.button.secondary}`;
    }
  };

  const availableActions = getAvailableActions();

  if (availableActions.length === 0) {
    return null;
  }

  return (
    <div className={`space-y-2 ${className}`}>
      {/* Action Buttons */}
      <div className="flex space-x-2">
        {availableActions.map((action) => {
          const isLoading = loadingAction === action.key;
          const isDisabled = disabled || loadingAction !== null;
          
          return (
            <button
              key={action.key}
              onClick={() => handleAction(action.key, action.fn)}
              disabled={isDisabled}
              className={getButtonStyles(action.style, isLoading, isDisabled)}
              aria-label={`${action.label} schema ${schema.name}`}
            >
              {isLoading && (
                <div className="animate-spin h-3 w-3 border border-current border-t-transparent rounded-full mr-1"></div>
              )}
              {isLoading ? UI_STATES.loading : action.label}
            </button>
          );
        })}
      </div>

      {/* Confirmation Dialog */}
      {confirmAction && (
        <div className="bg-yellow-50 border border-yellow-200 rounded-md p-3">
          <div className="flex items-start">
            <div className="flex-shrink-0">
              <span className="text-yellow-600">⚠️</span>
            </div>
            <div className="ml-3 flex-1">
              <h4 className="text-sm font-medium text-yellow-800">
                Confirm Action
              </h4>
              <p className="mt-1 text-sm text-yellow-700">
                Are you sure you want to {confirmAction} schema "{schema.name}"?
                {confirmAction === 'block' && ' This will prevent it from being used in queries and mutations.'}
                {confirmAction === 'unload' && ' This will remove it from the system.'}
              </p>
              <div className="mt-3 flex space-x-2">
                <button
                  onClick={() => confirmAndExecute(confirmAction)}
                  className={getButtonStyles(
                    confirmAction === 'block' || confirmAction === 'unload' ? 'danger' : 'primary',
                    false,
                    false
                  )}
                >
                  {BUTTON_TEXT.confirm}
                </button>
                <button
                  onClick={cancelConfirmation}
                  className={getButtonStyles('secondary', false, false)}
                >
                  {BUTTON_TEXT.cancel}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default SchemaActions;