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
 * @property {boolean} [showJson] - Whether to show raw JSON
 * @property {boolean} [collapsible] - Whether preview is collapsible
 * @property {string} [className] - Additional CSS classes
 * @property {string} [title] - Preview section title
 */

/**
 * Format query object for human-readable display
 */
const formatQueryDisplay = (query) => {
  if (!query) return null;

  const display = {
    schema: query.schema,
    fields: query.fields || [],
    filters: {}
  };

  // Format range filters
  if (query.filter) {
    if (query.filter.range_filter) {
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
    } else if (query.filter.field && query.filter.range_filter) {
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
  showJson = false,
  collapsible = true,
  className = '',
  title = 'Query Preview'
}) {
  const formattedQuery = useMemo(() => formatQueryDisplay(query), [query]);

  if (!query) {
    return (
      <div className={`bg-gray-50 rounded-md p-4 ${className}`}>
        <h3 className="text-sm font-medium text-gray-500 mb-2">{title}</h3>
        <p className="text-sm text-gray-400 italic">No query to preview</p>
      </div>
    );
  }

  return (
    <div className={`bg-white border border-gray-200 rounded-lg shadow-sm ${className}`}>
      <div className="px-4 py-3 border-b border-gray-200">
        <h3 className="text-sm font-medium text-gray-900">{title}</h3>
      </div>
      
      <div className="p-4 space-y-4">
        {/* Human-readable format */}
        <div className="space-y-3">
          {/* Schema */}
          <div>
            <label className="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
              Schema
            </label>
            <div className="inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm font-medium">
              {formattedQuery.schema}
            </div>
          </div>

          {/* Fields */}
          <div>
            <label className="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
              Fields ({formattedQuery.fields.length})
            </label>
            <div className="flex flex-wrap gap-1">
              {formattedQuery.fields.map((field, index) => (
                <span
                  key={index}
                  className="inline-flex items-center px-2 py-1 rounded-md bg-green-100 text-green-800 text-sm"
                >
                  {field}
                </span>
              ))}
            </div>
          </div>

          {/* Filters */}
          {Object.keys(formattedQuery.filters).length > 0 && (
            <div>
              <label className="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                Filters
              </label>
              <div className="space-y-2">
                {Object.entries(formattedQuery.filters).map(([fieldName, filter]) => (
                  <div key={fieldName} className="bg-yellow-50 rounded-md p-3">
                    <div className="font-medium text-sm text-yellow-800 mb-1">
                      {fieldName}
                    </div>
                    <div className="text-sm text-yellow-700">
                      {filter.exactKey && (
                        <span>Exact key: <code className="bg-yellow-200 px-1 rounded">{filter.exactKey}</code></span>
                      )}
                      {filter.keyRange && (
                        <span>Key range: <code className="bg-yellow-200 px-1 rounded">{filter.keyRange}</code></span>
                      )}
                      {filter.keyPrefix && (
                        <span>Key prefix: <code className="bg-yellow-200 px-1 rounded">{filter.keyPrefix}</code></span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* JSON format toggle */}
        {showJson && (
          <div className="border-t border-gray-200 pt-4">
            <label className="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">
              Raw JSON
            </label>
            <pre className="bg-gray-900 text-gray-100 text-xs p-3 rounded-md overflow-x-auto">
              {JSON.stringify(query, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

export default QueryPreview;
export { QueryPreview, formatQueryDisplay };