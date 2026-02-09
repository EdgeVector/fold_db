/**
 * QueryTab Component - Refactored for UCR-1-7
 * Orchestrates child components and Redux state management
 *
 * REFACTORED: Now uses extracted components following established patterns:
 * - useQueryState hook for state management
 * - QueryForm for form UI
 * - QueryActions for action controls
 * - useQueryBuilder for query construction
 * - QueryPreview for query visualization
 */

import { useCallback, useState, useEffect } from 'react';
import { mutationClient } from '../../api/clients/mutationClient';
import { useQueryState } from '../../hooks/useQueryState';
import { useQueryBuilder } from '../../hooks/useQueryBuilder';
import QueryForm from '../query/QueryForm';
import QueryActions from '../query/QueryActions';
import QueryPreview from '../query/QueryPreview';
// import { useAppSelector } from '../../store/hooks';

function QueryTab({ onResult }) {
  // UCR-1-7: Refactored to use extracted components and hooks
  // Use the extracted query state management hook
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
        error: `Network error: ${error.message}`,
        details: error
      });
    } finally {
      setIsExecuting(false);
    }
  }, [onResult, isValid]);

  /**
   * Handle query validation (optional feature)
   */
  const handleValidateQuery = useCallback(async (queryData) => {
    // Future enhancement: add query validation endpoint
  }, []);

  /**
   * Handle save query functionality
   */
  const handleSaveQuery = useCallback(async (queryData) => {
    if (!queryData || !isValid) {
      console.warn('Cannot save invalid query');
      return;
    }

    try {
      // Future enhancement: implement save query API endpoint
      
      // For now, just store in localStorage as a demo
      const savedQueries = JSON.parse(localStorage.getItem('savedQueries') || '[]');
      const newQuery = {
        id: Date.now(),
        name: `Query ${savedQueries.length + 1}`,
        data: queryData,
        createdAt: new Date().toISOString()
      };
      savedQueries.push(newQuery);
      localStorage.setItem('savedQueries', JSON.stringify(savedQueries));
      
    } catch (error) {
      console.error('Failed to save query:', error);
    }
  }, [isValid]);

  // UI does not require authentication

  return (
    <div className="p-6">
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
            onValidate={() => handleValidateQuery(query)}
            onSave={() => handleSaveQuery(query)}
            onClear={clearState}
            queryData={query}
            disabled={!isValid}
            isExecuting={isExecuting}
            showValidation={false} // Can be enabled for debugging
            showSave={true}
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