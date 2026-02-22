/**
 * QueryTab Component
 * Orchestrates query form, actions, preview, and execution.
 */

import { useCallback, useState, useEffect } from 'react';
import { mutationClient } from '../../api/clients/mutationClient';
import { useQueryState } from '../../hooks/useQueryState';
import { useQueryBuilder } from '../../hooks/useQueryBuilder';
import QueryForm from '../query/QueryForm';
import QueryActions from '../query/QueryActions';
import QueryPreview from '../query/QueryPreview';

function QueryTab({ onResult }) {
  // Query state management
  const {
    state: queryState,
    handleSchemaChange,
    toggleField: handleFieldToggle,
    handleFieldValueChange,
    handleRangeFilterChange,
    setRangeSchemaFilter,
    setHashKeyValue,
    clearState,
    refetchSchemas,
    approvedSchemas,
    schemasLoading,
    selectedSchemaObj,
    isRangeSchema,
    isHashRangeSchema,
    rangeKey
  } = useQueryState();

  // Fetch schema states from backend when tab is activated
  useEffect(() => {
    refetchSchemas();
  }, [refetchSchemas]);

  // Execution state management
  const [isExecuting, setIsExecuting] = useState(false);

  // Use the extracted query builder for query construction
  const { query, isValid } = useQueryBuilder({
    schema: queryState.selectedSchema,
    queryState,
    schemas: { [queryState.selectedSchema]: selectedSchemaObj }
  });

  /**
   * Handle query execution - follows original QueryTab pattern
   */
  const handleExecuteQuery = useCallback(async (queryData) => {
    // Backend handles all validation
    if (!queryData) {
      onResult({
        error: 'No query data provided'
      });
      return;
    }

    setIsExecuting(true);
    try {
      const response = await mutationClient.executeQuery(queryData);
      
      if (!response.success) {
        console.error('Query failed:', response.error);
        onResult({
          error: response.error || 'Query execution failed',
          details: response
        });
        return;
      }
      
      // Pass the actual query data from response.data
      // API returns { ok: true, results: [...] } in data, extract results array
      onResult({
        success: true,
        data: response.data?.results || response.data
      });
    } catch (error) {
      console.error('Failed to execute query:', error);
      onResult({
        error: `Network error: ${error instanceof Error ? error.message : String(error)}`,
        details: error
      });
    } finally {
      setIsExecuting(false);
    }
  }, [onResult]);


  // UI does not require authentication

  return (
    <div>
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Query Form */}
        <div className="lg:col-span-2 space-y-6">
          <QueryForm
            queryState={queryState}
            onSchemaChange={handleSchemaChange}
            onFieldToggle={handleFieldToggle}
            onFieldValueChange={handleFieldValueChange}
            onRangeFilterChange={handleRangeFilterChange}
            onRangeSchemaFilterChange={setRangeSchemaFilter}
            onHashKeyChange={setHashKeyValue}
            approvedSchemas={approvedSchemas}
            schemasLoading={schemasLoading}
            isRangeSchema={isRangeSchema}
            isHashRangeSchema={isHashRangeSchema}
            rangeKey={rangeKey}
          />

          {/* Query Actions */}
          <QueryActions
            onExecute={() => handleExecuteQuery(query)}
            onClear={clearState}
            queryData={query}
            disabled={!isValid}
            isExecuting={isExecuting}
            showValidation={false}
            showSave={false}
            showClear={true}
          />
        </div>

        {/* Query Preview Sidebar */}
        <div className="lg:col-span-1">
          <QueryPreview
            query={query}
            showJson={false} // Can be toggled for debugging
            title="Query Preview"
          />
        </div>
      </div>
    </div>
  );
}

export default QueryTab