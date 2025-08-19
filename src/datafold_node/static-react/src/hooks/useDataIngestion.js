/**
 * @fileoverview Data Ingestion Hook
 * 
 * Provides functionality for data ingestion, processing, and schema inference.
 * Used by the IngestionTab component to handle data uploads and processing.
 * 
 * @module useDataIngestion
 * @since 2.0.0
 */

import { useState, useCallback } from 'react';
import { useApiClient } from './useApiClient';

/**
 * Hook for managing data ingestion workflow
 * @returns {object} Data ingestion state and methods
 */
export function useDataIngestion() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [suggestedSchema, setSuggestedSchema] = useState(null);
  const [processingProgress, setProcessingProgress] = useState(0);
  const [error, setError] = useState(null);
  const apiClient = useApiClient();

  /**
   * Process uploaded data and infer schema
   * @param {File|string} data - Data to process
   * @param {object} options - Processing options
   * @returns {Promise<object>} Processing result
   */
  const processData = useCallback(async (data, options = {}) => {
    setIsProcessing(true);
    setError(null);
    setProcessingProgress(0);

    try {
      // Simulate processing progress
      const progressInterval = setInterval(() => {
        setProcessingProgress(prev => Math.min(prev + 10, 90));
      }, 100);

      const formData = new FormData();
      if (data instanceof File) {
        formData.append('file', data);
      } else {
        formData.append('data', data);
      }
      
      if (options.inferSchema !== false) {
        formData.append('inferSchema', 'true');
      }

      const response = await apiClient.post('/api/data/ingest', formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      });

      clearInterval(progressInterval);
      setProcessingProgress(100);

      const result = {
        success: true,
        recordsProcessed: response.data.recordsProcessed || 0,
        suggestedSchema: response.data.suggestedSchema || null,
        errors: response.data.errors || [],
        warnings: response.data.warnings || [],
      };

      if (result.suggestedSchema) {
        setSuggestedSchema(result.suggestedSchema);
      }

      return result;
    } catch (err) {
      setError(err.message || 'Data processing failed');
      throw err;
    } finally {
      setIsProcessing(false);
    }
  }, [apiClient]);

  /**
   * Approve and save a suggested schema
   * @param {object} schema - Schema to approve
   * @param {string} name - Schema name
   * @returns {Promise<object>} Save result
   */
  const approveSchema = useCallback(async (schema, name) => {
    try {
      const response = await apiClient.post('/api/schemas', {
        name,
        schema,
        source: 'inferred',
      });

      setSuggestedSchema(null);
      return response.data;
    } catch (err) {
      setError(err.message || 'Schema approval failed');
      throw err;
    }
  }, [apiClient]);

  /**
   * Clear the suggested schema
   */
  const clearSuggestedSchema = useCallback(() => {
    setSuggestedSchema(null);
  }, []);

  /**
   * Validate data against a schema
   * @param {any} data - Data to validate
   * @param {object} schema - Schema to validate against
   * @returns {Promise<object>} Validation result
   */
  const validateData = useCallback(async (data, schema) => {
    try {
      const response = await apiClient.post('/api/data/validate', {
        data,
        schema,
      });

      return {
        isValid: response.data.isValid,
        errors: response.data.errors || [],
        warnings: response.data.warnings || [],
      };
    } catch (err) {
      setError(err.message || 'Data validation failed');
      throw err;
    }
  }, [apiClient]);

  /**
   * Get processing statistics
   * @returns {Promise<object>} Processing stats
   */
  const getProcessingStats = useCallback(async () => {
    try {
      const response = await apiClient.get('/api/data/stats');
      return response.data;
    } catch (err) {
      setError(err.message || 'Failed to fetch processing stats');
      throw err;
    }
  }, [apiClient]);

  return {
    // State
    isProcessing,
    suggestedSchema,
    processingProgress,
    error,

    // Methods
    processData,
    approveSchema,
    clearSuggestedSchema,
    validateData,
    getProcessingStats,
  };
}

export default useDataIngestion;