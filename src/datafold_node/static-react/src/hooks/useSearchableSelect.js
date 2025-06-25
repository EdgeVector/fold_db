/**
 * useSearchableSelect Hook
 * TASK-009: Additional Simplification - Extracted from SelectField complexity
 * 
 * Custom hook for managing searchable select state and behavior.
 * This extraction reduces SelectField component complexity and improves reusability.
 */

import { useState, useCallback } from 'react';
import { filterOptions, groupOptions } from '../utils/selectFieldHelpers.js';

/**
 * @typedef {Object} SearchableSelectState
 * @property {string} searchTerm - Current search term
 * @property {boolean} isOpen - Whether dropdown is open
 * @property {Array} filteredOptions - Options filtered by search term
 * @property {Object} groupedOptions - Options grouped by category
 */

/**
 * @typedef {Object} SearchableSelectActions
 * @property {Function} setSearchTerm - Update search term
 * @property {Function} openDropdown - Open dropdown
 * @property {Function} closeDropdown - Close dropdown
 * @property {Function} toggleDropdown - Toggle dropdown state
 * @property {Function} selectOption - Select an option and close dropdown
 * @property {Function} clearSearch - Clear search term
 */

/**
 * @typedef {Object} UseSearchableSelectResult
 * @property {SearchableSelectState} state - Current state
 * @property {SearchableSelectActions} actions - Available actions
 * @property {Function} handleSearchChange - Handle search input change
 * @property {Function} handleOptionSelect - Handle option selection
 */

/**
 * Custom hook for managing searchable select functionality
 * 
 * Provides state management and actions for searchable select components,
 * including search term filtering, dropdown visibility, and option selection.
 * 
 * @param {Array} options - Array of select options
 * @param {Function} onChange - Callback when option is selected
 * @param {boolean} autoClose - Whether to auto-close on selection (default: true)
 * @returns {UseSearchableSelectResult} Hook result with state and actions
 * 
 * @example
 * ```jsx
 * function SearchableSelect({ options, onChange }) {
 *   const { state, actions, handleSearchChange, handleOptionSelect } = useSearchableSelect(
 *     options, 
 *     onChange
 *   );
 * 
 *   return (
 *     <div>
 *       <input
 *         value={state.searchTerm}
 *         onChange={handleSearchChange}
 *         onFocus={actions.openDropdown}
 *       />
 *       {state.isOpen && (
 *         <div>
 *           {Object.entries(state.groupedOptions).map(([group, groupOptions]) => (
 *             <div key={group}>
 *               {groupOptions.map(option => (
 *                 <button
 *                   key={option.value}
 *                   onClick={() => handleOptionSelect(option)}
 *                 >
 *                   {option.label}
 *                 </button>
 *               ))}
 *             </div>
 *           ))}
 *         </div>
 *       )}
 *     </div>
 *   );
 * }
 * ```
 */
export function useSearchableSelect(options = [], onChange, autoClose = true) {
  const [searchTerm, setSearchTerm] = useState('');
  const [isOpen, setIsOpen] = useState(false);

  // Filter and group options based on search term
  const filteredOptions = filterOptions(options, searchTerm);
  const groupedOptions = groupOptions(filteredOptions);

  /**
   * Handle search input change
   */
  const handleSearchChange = useCallback((e) => {
    setSearchTerm(e.target.value);
  }, []);

  /**
   * Handle option selection
   */
  const handleOptionSelect = useCallback((option) => {
    if (option.disabled) return;
    
    onChange(option.value);
    
    if (autoClose) {
      setIsOpen(false);
      setSearchTerm('');
    }
  }, [onChange, autoClose]);

  /**
   * Open dropdown
   */
  const openDropdown = useCallback(() => {
    setIsOpen(true);
  }, []);

  /**
   * Close dropdown
   */
  const closeDropdown = useCallback(() => {
    setIsOpen(false);
  }, []);

  /**
   * Toggle dropdown state
   */
  const toggleDropdown = useCallback(() => {
    setIsOpen(prev => !prev);
  }, []);

  /**
   * Select option and handle closing
   */
  const selectOption = useCallback((optionValue) => {
    const option = options.find(opt => opt.value === optionValue);
    if (option) {
      handleOptionSelect(option);
    }
  }, [options, handleOptionSelect]);

  /**
   * Clear search term
   */
  const clearSearch = useCallback(() => {
    setSearchTerm('');
  }, []);

  const state = {
    searchTerm,
    isOpen,
    filteredOptions,
    groupedOptions
  };

  const actions = {
    setSearchTerm,
    openDropdown,
    closeDropdown,
    toggleDropdown,
    selectOption,
    clearSearch
  };

  return {
    state,
    actions,
    handleSearchChange,
    handleOptionSelect
  };
}

export default useSearchableSelect;