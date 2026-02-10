/**
 * QueryForm Component
 * Provides form structure for query building with input validation
 * Part of UCR-1-4: Create QueryForm component for input validation
 * Follows form patterns established in components/form/ directory
 */

import { useState, useCallback } from 'react';
import { useQueryState } from '../../hooks/useQueryState.js';
import FieldWrapper from '../form/FieldWrapper';
import SelectField from '../form/SelectField';
import RangeField from '../form/RangeField';
import { FORM_LABELS } from '../../constants/ui.js';
import { SCHEMA_ERROR_MESSAGES } from '../../constants/redux.js';
import { getHashKey, getRangeKey } from '../../utils/rangeSchemaHelpers.js';

/**
 * @typedef {Object} QueryFormProps
 * @property {Object} queryState - Current query state from useQueryState hook
 * @property {function} onSchemaChange - Handle schema selection change
 * @property {function} onFieldToggle - Handle field selection toggle
 * @property {function} onFieldValueChange - Handle field value changes
 * @property {function} onRangeFilterChange - Handle range filter changes
 * @property {function} onRangeSchemaFilterChange - Handle range schema filter changes
 * @property {function} onHashKeyChange - Handle hash key changes for HashRange schemas
 * @property {Object[]} approvedSchemas - Array of approved schemas
 * @property {boolean} schemasLoading - Loading state for schemas
 * @property {boolean} isRangeSchema - Whether selected schema is range schema
 * @property {boolean} isHashRangeSchema - Whether selected schema is HashRange schema
 * @property {string|null} rangeKey - Range key for selected schema
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Query form component with validation following form patterns
 * 
 * @param {QueryFormProps} props
 * @returns {JSX.Element}
 */
function QueryForm({
  queryState,
  onSchemaChange,
  onFieldToggle,
  onFieldValueChange: _onFieldValueChange,
  onRangeFilterChange,
  onRangeSchemaFilterChange,
  onHashKeyChange,
  approvedSchemas,
  schemasLoading,
  isRangeSchema,
  isHashRangeSchema,
  rangeKey,
  className = ''
}) {
  const [validationErrors, setValidationErrors] = useState({});
  const { clearQuery } = useQueryState();

  /**
   * No validation - backend handles all checks
   */
  const _validateForm = useCallback(() => {
    setValidationErrors({});
    return true; // Always valid - backend validates
  }, []);

  /**
   * Handle schema change with validation
   */
  const handleSchemaChange = useCallback((value) => {
    onSchemaChange(value);
    // Clear query state when schema changes
    if (clearQuery) {
      clearQuery();
    }
    // Clear schema validation error
    setValidationErrors(prev => {
      const { schema: _schema, ...rest } = prev;
      return rest;
    });
  }, [onSchemaChange, clearQuery]);

  /**
   * Handle field toggle with validation
   */
  const handleFieldToggle = useCallback((fieldName) => {
    onFieldToggle(fieldName);
    // Clear fields validation error
    setValidationErrors(prev => {
      const { fields: _fields, ...rest } = prev;
      return rest;
    });
  }, [onFieldToggle]);

  const selectedSchema = queryState?.selectedSchema && approvedSchemas
    ? approvedSchemas.find(s => s.name === queryState.selectedSchema)
    : null;

  // Backend sends fields as an array of strings for regular schemas,
  // or transform_fields as an object for transform schemas
  const selectedSchemaFields = selectedSchema?.fields || selectedSchema?.transform_fields || [];
  const fieldNames = Array.isArray(selectedSchemaFields) 
    ? selectedSchemaFields 
    : Object.keys(selectedSchemaFields);

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Schema Selection */}
      <FieldWrapper
        label={FORM_LABELS.schema || 'Schema'}
        name="schema"
        required
        error={validationErrors.schema}
        helpText={FORM_LABELS.schemaHelp || 'Select a schema to work with'}
      >
        <SelectField
          name="schema"
          value={queryState?.selectedSchema || ''}
          onChange={handleSchemaChange}
          options={approvedSchemas.map(schema => ({
            value: schema.name,
            label: schema.descriptive_name || schema.name
          }))}
          placeholder="Select a schema..."
          emptyMessage={FORM_LABELS.schemaEmpty || 'No schemas available'}
          loading={schemasLoading}
        />
      </FieldWrapper>

      {/* Field Selection - Show for all field types including HashRange */}
      {queryState?.selectedSchema && fieldNames.length > 0 && (
        <FieldWrapper
          label="Field Selection"
          name="fields"
          required
          error={validationErrors.fields}
          helpText="Select fields to include in your query"
        >
          <div className="minimal-card p-4">
            <div className="space-y-3">
              {fieldNames.map(fieldName => (
                <label key={fieldName} className="relative flex items-start">
                  <div className="flex items-center h-5">
                    <input
                      type="checkbox"
                      className="h-4 w-4 text-primary rounded focus:ring-primary" style={{borderColor: 'var(--color-border)'}}
                      checked={queryState?.queryFields?.includes(fieldName) || false}
                      onChange={() => handleFieldToggle(fieldName)}
                    />
                  </div>
                  <div className="ml-3 flex items-center">
                    <span className="text-sm font-medium text-primary">{fieldName}</span>
                  </div>
                </label>
              ))}
            </div>
          </div>
        </FieldWrapper>
      )}


      {/* HashRange Schema Filter - only show for HashRange schemas */}
      {isHashRangeSchema && (
        <FieldWrapper
          label="HashRange Filter"
          name="hashRangeFilter"
          helpText="Filter data by hash and range key values"
        >
          <div className="minimal-section-purple p-4 space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* Hash Key Input */}
              <div className="space-y-2">
                <label className="block text-sm font-medium text-primary">
                  Hash Key
                </label>
                <input
                  type="text"
                  placeholder="Enter hash key value"
                  className="w-full px-3 py-2 text-sm border border-default rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                  value={queryState?.hashKeyValue || ''}
                  onChange={(e) => onHashKeyChange(e.target.value)}
                />
                <div className="text-xs text-secondary">
                  Hash field: {getHashKey(approvedSchemas.find(s => s.name === queryState?.selectedSchema)) || 'N/A'}
                </div>
              </div>

              {/* Range Key Input */}
              <div className="space-y-2">
                <label className="block text-sm font-medium text-primary">
                  Range Key
                </label>
                <input
                  type="text"
                  placeholder="Enter range key value"
                  className="w-full px-3 py-2 text-sm border border-default rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                  value={queryState?.rangeKeyValue || ''}
                  onChange={(e) => onRangeSchemaFilterChange({ key: e.target.value })}
                />
                <div className="text-xs text-secondary">
                  Range field: {getRangeKey(approvedSchemas.find(s => s.name === queryState?.selectedSchema)) || 'N/A'}
                </div>
              </div>
            </div>
            
            <div className="text-xs text-secondary">
              <p><strong>Hash Key:</strong> Used for partitioning data across multiple nodes</p>
              <p><strong>Range Key:</strong> Used for ordering and range queries within a partition</p>
            </div>
          </div>
        </FieldWrapper>
      )}

      {/* Range Schema Filter - only show for range schemas */}
      {isRangeSchema && rangeKey && (
        <FieldWrapper
          label="Range Filter"
          name="rangeSchemaFilter"
          error={validationErrors.rangeFilter}
          helpText="Filter data by range key values"
        >
          <RangeField
            name="rangeSchemaFilter"
            value={queryState?.rangeSchemaFilter || {}}
            onChange={(value) => {
              onRangeSchemaFilterChange(value);
              // Clear range filter validation error
              setValidationErrors(prev => {
                const { rangeFilter: _rangeFilter, ...rest } = prev;
                return rest;
              });
            }}
            rangeKeyName={rangeKey}
            mode="all"
          />
        </FieldWrapper>
      )}

      {/* Note: Regular Range Field Filters section removed - declarative schemas don't have field_type metadata */}
    </div>
  );
}

export default QueryForm;
export { QueryForm };