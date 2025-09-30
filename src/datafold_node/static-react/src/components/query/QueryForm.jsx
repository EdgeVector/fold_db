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
   * Validate query form data
   */
  const _validateForm = useCallback(() => {
    const errors = {};

    // Schema validation
    if (!queryState?.selectedSchema) {
      errors.schema = 'Schema selection is required';
    }

    // Fields validation
    if (!queryState?.queryFields || queryState.queryFields.length === 0) {
      errors.fields = 'At least one field must be selected';
    }

    // Range validation for range schemas
    if (isRangeSchema && queryState?.rangeSchemaFilter) {
      const filter = queryState.rangeSchemaFilter;
      if (filter.start && filter.end && filter.start >= filter.end) {
        errors.rangeFilter = 'Start key must be less than end key';
      }
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  }, [queryState, isRangeSchema]);

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

  const selectedSchemaFields = queryState?.selectedSchema && approvedSchemas
    ? approvedSchemas.find(s => s.name === queryState.selectedSchema)?.fields || {}
    : {};

  const rangeFields = Object.entries(selectedSchemaFields)
    .filter(([__name, field]) => field.field_type === 'Range');

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
            label: schema.name
          }))}
          placeholder="Select a schema..."
          emptyMessage={FORM_LABELS.schemaEmpty || 'No schemas available'}
          loading={schemasLoading}
        />
      </FieldWrapper>

      {/* Field Selection - Show for all field types including HashRange */}
      {queryState?.selectedSchema && Object.entries(selectedSchemaFields).length > 0 && (
        <FieldWrapper
          label="Field Selection"
          name="fields"
          required
          error={validationErrors.fields}
          helpText="Select fields to include in your query"
        >
          <div className="bg-gray-50 rounded-md p-4">
            <div className="space-y-3">
              {Object.entries(selectedSchemaFields)
                .map(([fieldName, field]) => (
                <label key={fieldName} className="relative flex items-start">
                  <div className="flex items-center h-5">
                    <input
                      type="checkbox"
                      className="h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary"
                      checked={queryState?.queryFields?.includes(fieldName) || false}
                      onChange={() => handleFieldToggle(fieldName)}
                    />
                  </div>
                  <div className="ml-3 flex items-center">
                    <span className="text-sm font-medium text-gray-700">{fieldName}</span>
                    <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600">
                      {field.field_type || 'HashRange'}
                    </span>
                    {field.required && (
                      <span className="ml-1 text-red-500 text-xs">*</span>
                    )}
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
          <div className="bg-purple-50 rounded-md p-4 space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* Hash Key Input */}
              <div className="space-y-2">
                <label className="block text-sm font-medium text-gray-700">
                  Hash Key
                </label>
                <input
                  type="text"
                  placeholder="Enter hash key value"
                  className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                  value={queryState?.hashKeyValue || ''}
                  onChange={(e) => onHashKeyChange(e.target.value)}
                />
                <div className="text-xs text-gray-500">
                  Hash field: {getHashKey(approvedSchemas.find(s => s.name === queryState?.selectedSchema)) || 'N/A'}
                </div>
              </div>

              {/* Range Key Input */}
              <div className="space-y-2">
                <label className="block text-sm font-medium text-gray-700">
                  Range Key
                </label>
                <input
                  type="text"
                  placeholder="Enter range key value"
                  className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                  value={queryState?.rangeKeyValue || ''}
                  onChange={(e) => onRangeSchemaFilterChange({ key: e.target.value })}
                />
                <div className="text-xs text-gray-500">
                  Range field: {getRangeKey(approvedSchemas.find(s => s.name === queryState?.selectedSchema)) || 'N/A'}
                </div>
              </div>
            </div>
            
            <div className="text-xs text-gray-500">
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

      {/* Regular Range Field Filters - only show for non-range schemas */}
      {!isRangeSchema && rangeFields.length > 0 && queryState?.queryFields?.some(fieldName =>
        selectedSchemaFields[fieldName]?.field_type === 'Range'
      ) && (
        <FieldWrapper
          label="Range Field Filters"
          name="rangeFieldFilters"
          helpText="Configure filters for range fields"
        >
          <div className="bg-blue-50 rounded-md p-4 space-y-4">
            {rangeFields
              .filter(([fieldName]) => queryState?.queryFields?.includes(fieldName))
              .map(([fieldName]) => (
                <div key={fieldName} className="border-b border-blue-200 pb-4 last:border-b-0 last:pb-0">
                  <h4 className="text-sm font-medium text-gray-800 mb-3">{fieldName}</h4>
                  
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    {/* Key Range Filter */}
                    <div className="space-y-2">
                      <label className="block text-xs font-medium text-gray-600">Key Range</label>
                      <input
                        type="text"
                        placeholder="Start key"
                        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                        value={queryState?.rangeFilters?.[fieldName]?.start || ''}
                        onChange={(e) => onRangeFilterChange(fieldName, 'start', e.target.value)}
                      />
                      <input
                        type="text"
                        placeholder="End key"
                        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                        value={queryState?.rangeFilters?.[fieldName]?.end || ''}
                        onChange={(e) => onRangeFilterChange(fieldName, 'end', e.target.value)}
                      />
                    </div>

                    {/* Single Key Filter */}
                    <div className="space-y-2">
                      <label className="block text-xs font-medium text-gray-600">Exact Key</label>
                      <input
                        type="text"
                        placeholder="Exact key to match"
                        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                        value={queryState?.rangeFilters?.[fieldName]?.key || ''}
                        onChange={(e) => onRangeFilterChange(fieldName, 'key', e.target.value)}
                      />
                    </div>

                    {/* Key Prefix Filter */}
                    <div className="space-y-2">
                      <label className="block text-xs font-medium text-gray-600">Key Prefix</label>
                      <input
                        type="text"
                        placeholder="Key prefix (e.g., 'user:')"
                        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                        value={queryState?.rangeFilters?.[fieldName]?.keyPrefix || ''}
                        onChange={(e) => onRangeFilterChange(fieldName, 'keyPrefix', e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="mt-3 text-xs text-gray-500">
                    <p><strong>Key Range:</strong> Matches keys between start and end (inclusive start, exclusive end)</p>
                    <p><strong>Exact Key:</strong> Matches a specific key exactly</p>
                    <p><strong>Key Prefix:</strong> Matches all keys starting with the prefix</p>
                  </div>
                </div>
              ))}
          </div>
        </FieldWrapper>
      )}
    </div>
  );
}

export default QueryForm;
export { QueryForm };