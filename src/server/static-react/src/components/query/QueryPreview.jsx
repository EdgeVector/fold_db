/**
 * QueryPreview Component
 * Displays formatted query preview for visualization and validation
 * Part of UCR-1-5: Create QueryPreview component for query visualization
 * Extracts query preview and visualization logic into dedicated component
 */

import { useMemo } from 'react';

/**
 * @typedef {Object} QueryPreviewProps
 * @property {Object|null} query - Query object to preview
 * @property {Object|null} queryState - Query state for field values
 * @property {Array} [validationErrors] - Validation errors to display
 * @property {boolean} [isExecuting] - Whether query is executing
 * @property {boolean} [showJson] - Whether to show raw JSON
 * @property {boolean} [collapsible] - Whether preview is collapsible
 * @property {string} [className] - Additional CSS classes
 * @property {string} [title] - Preview section title
 */

/**
 * Format query object for human-readable display
 */
const formatQueryDisplay = (query, queryState) => {
  if (!query && !queryState) return null;

  // Combine query and queryState data
  const combined = { ...query, ...queryState };

  // Handle both array format (queryFields) and object format (fieldValues)
  let fields = [];
  let fieldValues = {};
  
  if (Array.isArray(combined.fields)) {
    fields = combined.fields;
  } else if (combined.fields && typeof combined.fields === 'object') {
    // If fields is an object (fieldValues), extract both keys and values
    fields = Object.keys(combined.fields);
    fieldValues = combined.fields;
  } else if (combined.queryFields && Array.isArray(combined.queryFields)) {
    // Fallback to queryFields if available
    fields = combined.queryFields;
  }

  // Include fieldValues from queryState if available
  if (combined.fieldValues && typeof combined.fieldValues === 'object') {
    fieldValues = { ...fieldValues, ...combined.fieldValues };
  }

  const display = {
    schema: combined.schema || combined.selectedSchema,
    fields: fields,
    fieldValues: fieldValues,
    filters: combined.filters || {}, // Include filters from test mocks
    orderBy: combined.orderBy, // Include orderBy from test mocks
    rangeKey: combined.rangeKey // Include rangeKey from test mocks
  };

  // Format range filters (legacy format)
  if (query && query.filter) {
    if (query.filter.field && query.filter.range_filter) {
      // Regular field range filters
      const fieldName = query.filter.field;
      const filter = query.filter.range_filter;
      
      if (filter.Key) {
        display.filters[fieldName] = { exactKey: filter.Key };
      } else if (filter.KeyRange) {
        display.filters[fieldName] = {
          keyRange: `${filter.KeyRange.start} → ${filter.KeyRange.end}`
        };
      } else if (filter.KeyPrefix) {
        display.filters[fieldName] = { keyPrefix: filter.KeyPrefix };
      }
    } else if (query.filter.range_filter) {
      // Range schema filters
      Object.entries(query.filter.range_filter).forEach(([key, filter]) => {
        if (typeof filter === 'string') {
          display.filters[key] = { exactKey: filter };
        } else if (filter.KeyRange) {
          display.filters[key] = {
            keyRange: `${filter.KeyRange.start} → ${filter.KeyRange.end}`
          };
        } else if (filter.KeyPrefix) {
          display.filters[key] = { keyPrefix: filter.KeyPrefix };
        }
      });
    }
  }

  return display;
};

/**
 * QueryPreview component for query visualization
 * 
 * @param {QueryPreviewProps} props
 * @returns {JSX.Element}
 */
function QueryPreview({
  query,
  queryState,
  validationErrors = [],
  isExecuting = false,
  showJson = false,
  collapsible: _collapsible = true,
  className = '',
  title = 'Query Preview'
}) {
  const formattedQuery = useMemo(() => formatQueryDisplay(query, queryState), [query, queryState]);

  if (!query && !queryState) {
    return (
      <div className={`card p-4 ${className}`}>
        <h3 className="text-sm font-medium text-secondary mb-2">{title}</h3>
        <p className="text-sm text-tertiary italic">No query to preview</p>
      </div>
    );
  }

  return (
    <div className={`card ${className}`}>
      <div className="px-4 py-3 border-b border-border">
        <h3 className="text-sm font-medium text-primary">{title}</h3>
      </div>
      
      <div className="p-4 space-y-4">
        {/* Validation Errors */}
        {validationErrors && validationErrors.length > 0 && (
          <div className="card card-error p-3">
            <div className="flex items-center mb-2">
              <svg className="h-4 w-4 text-gruvbox-red mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
              <span className="text-sm font-medium text-gruvbox-red">Validation Errors</span>
            </div>
            <ul className="space-y-1">
              {validationErrors.map((error) => (
                <li key={error} className="text-sm text-gruvbox-red">
                  {error}
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Executing Status */}
        {isExecuting && (
          <div className="card card-info p-3">
            <div className="flex items-center">
              <svg className="animate-spin h-4 w-4 text-gruvbox-blue mr-2" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span className="text-sm font-medium text-gruvbox-blue">Executing query...</span>
            </div>
          </div>
        )}

        {/* Human-readable format */}
        <div className="space-y-3">
          {/* Schema */}
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">
              Schema
            </label>
            <div className="inline-flex items-center badge badge-info text-sm font-medium">
              {formattedQuery?.schema || ''}
            </div>
          </div>

          {/* Fields with Values */}
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">
              Fields ({formattedQuery?.fields ? formattedQuery.fields.length : 0})
            </label>
            <div className="flex flex-wrap gap-1">
              {formattedQuery?.fields && formattedQuery.fields.length > 0 ? (
                formattedQuery.fields.map((field) => {
                  const fieldValue = formattedQuery.fieldValues?.[field];
                  return (
                    <div key={field} className="inline-flex flex-col items-start">
                      <span className="inline-flex items-center badge badge-success text-sm">
                        {field}
                      </span>
                      {fieldValue && (
                        <span className="text-xs text-secondary mt-1 px-2">
                          {fieldValue}
                        </span>
                      )}
                    </div>
                  );
                })
              ) : (
                <span className="text-sm text-secondary italic">No fields selected</span>
              )}
            </div>
          </div>

          {/* Filters */}
          {((formattedQuery.filters && Array.isArray(formattedQuery.filters) && formattedQuery.filters.length > 0) ||
            (formattedQuery.filters && !Array.isArray(formattedQuery.filters) && Object.keys(formattedQuery.filters).length > 0)) && (
            <div>
              <label className="block text-xs font-medium text-secondary mb-1">
                Filters
              </label>
              <div className="space-y-2">
                {Array.isArray(formattedQuery.filters) ? (
                  // Handle filters as array (from test mocks)
                  formattedQuery.filters.map((filter) => (
                    <div key={`${filter.field}-${filter.operator}-${filter.value}`} className="card card-warning p-3">
                      <div className="text-sm text-gruvbox-yellow">
                        {filter.field} {filter.operator} "{filter.value}"
                      </div>
                    </div>
                  ))
                ) : (
                  // Handle filters as object (existing format)
                  Object.entries(formattedQuery.filters).map(([fieldName, filter]) => (
                    <div key={fieldName} className="card card-warning p-3">
                      <div className="font-medium text-sm text-gruvbox-yellow mb-1">
                        {fieldName}
                      </div>
                      <div className="text-sm text-gruvbox-yellow">
                        {filter.exactKey && (
                          <span>Exact key: <code className="bg-gruvbox-elevated px-1 rounded">{filter.exactKey}</code></span>
                        )}
                        {filter.keyRange && (
                          <span>Key range: <code className="bg-gruvbox-elevated px-1 rounded">{filter.keyRange}</code></span>
                        )}
                        {filter.keyPrefix && (
                          <span>Key prefix: <code className="bg-gruvbox-elevated px-1 rounded">{filter.keyPrefix}</code></span>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {/* OrderBy */}
          {formattedQuery.orderBy && (
            <div>
              <label className="block text-xs font-medium text-secondary mb-1">
                OrderBy
              </label>
              <div className="card card-info p-3">
                <div className="text-sm text-gruvbox-blue">
                  {formattedQuery.orderBy.field} {formattedQuery.orderBy.direction}
                </div>
              </div>
            </div>
          )}

          {/* Range Key (for range schemas) */}
          {formattedQuery.rangeKey && (
            <div>
              <label className="block text-xs font-medium text-secondary mb-1">
                RangeKey
              </label>
              <div className="bg-gruvbox-elevated border border-border p-3">
                <div className="text-sm text-gruvbox-blue">
                  <code className="bg-gruvbox-elevated px-1">{formattedQuery.rangeKey}</code>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* JSON format toggle */}
        {showJson && (
          <div className="pt-4 border-t border-border">
            <label className="block text-xs font-medium text-secondary uppercase tracking-wide mb-2">
              Raw JSON
            </label>
            <pre className="text-xs p-3 overflow-x-auto bg-surface-secondary text-primary">
              {JSON.stringify(query, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

export default QueryPreview;
// eslint-disable-next-line react-refresh/only-export-components
export { QueryPreview, formatQueryDisplay };