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

import { useCallback } from 'react';
import { mutationClient } from '../../api/clients/mutationClient';
import { API_ENDPOINTS } from '../../api/endpoints';
import { useQueryState } from '../../hooks/useQueryState';
import { useQueryBuilder } from '../query/QueryBuilder';
import QueryForm from '../query/QueryForm';
import QueryActions from '../query/QueryActions';
import QueryPreview from '../query/QueryPreview';
import { useAppSelector } from '../../store/hooks';

function QueryTab({ onResult }) {
  // Authentication check - prevent access to query functionality without proper auth
  const authState = useAppSelector(state => state.auth);
  const { isAuthenticated } = authState;

  // Early return with authentication message if not authenticated
  if (!isAuthenticated) {
    return (
      <div className="p-6">
        <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
          <div className="flex items-center">
            <div className="flex-shrink-0">
              <svg className="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
            </div>
            <div className="ml-3">
              <h3 className="text-sm font-medium text-yellow-800">
                Authentication Required
              </h3>
              <div className="mt-2 text-sm text-yellow-700">
                <p>Please authenticate using the Keys tab before accessing query functionality.</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // UCR-1-7: Refactored to use extracted components and hooks
  // Use the extracted query state management hook
  const {
    state: queryState,
    handleSchemaChange,
    toggleField: handleFieldToggle,
    handleRangeFilterChange,
    setRangeSchemaFilter,
    clearState,
    approvedSchemas,
    schemasLoading,
    selectedSchemaObj,
    isRangeSchema,
    rangeKey
  } = useQueryState();

  // Use the extracted query builder for query construction
  const { query, isValid } = useQueryBuilder({
    queryState,
    selectedSchemaObj,
    isRangeSchema,
    rangeKey
  });

  /**
   * Handle query execution - follows original QueryTab pattern
   */
  const handleExecuteQuery = useCallback(async (queryData) => {
    if (!queryData || !isValid) {
      onResult({
        error: 'Invalid query configuration',
        details: { queryData, isValid }
      });
      return;
    }

    try {
      // Use core API client to post directly to /query endpoint
      const response = await mutationClient.client.post(API_ENDPOINTS.QUERY, queryData, {
        requiresAuth: true,
        timeout: 10000,
        retries: 2,
        cacheable: true,
        cacheTtl: 60000
      });
      
      if (!response.success) {
        console.error('Query failed:', response.error);
        onResult({
          error: response.error || 'Query execution failed',
          details: response
        });
        return;
      }
      
      // Pass the actual query data from response.data
      onResult({
        success: true,
        data: response.data // The actual query results are directly in response.data
      });
    } catch (error) {
      console.error('Failed to execute query:', error);
      onResult({
        error: `Network error: ${error.message}`,
        details: error
      });
    }
  }, [onResult, isValid]);

  /**
   * Handle query validation (optional feature)
   */
  const handleValidateQuery = useCallback(async (queryData) => {
    // Future enhancement: add query validation endpoint
    console.log('Validating query:', queryData);
  }, []);

  return (
    <div className="p-6">
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Query Form */}
        <div className="lg:col-span-2 space-y-6">
          <QueryForm
            queryState={queryState}
            onSchemaChange={handleSchemaChange}
            onFieldToggle={handleFieldToggle}
            onRangeFilterChange={handleRangeFilterChange}
            onRangeSchemaFilterChange={setRangeSchemaFilter}
            approvedSchemas={approvedSchemas}
            schemasLoading={schemasLoading}
            isRangeSchema={isRangeSchema}
            rangeKey={rangeKey}
          />

          {/* Query Actions */}
          <QueryActions
            onExecute={() => handleExecuteQuery(query)}
            onValidate={() => handleValidateQuery(query)}
            onClear={clearState}
            queryData={query}
            disabled={!isValid}
            showValidation={false} // Can be enabled for debugging
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