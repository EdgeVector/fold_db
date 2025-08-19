/**
 * @fileoverview Transform Queue Hook
 * 
 * Provides functionality for managing data transformation queue operations.
 * Used by the TransformsTab component to handle transform operations.
 * 
 * @module useTransformQueue
 * @since 2.0.0
 */

import { useState, useCallback, useRef } from 'react';
import { useApiClient } from './useApiClient';

/**
 * Hook for managing data transformation queue
 * @returns {object} Transform queue state and methods
 */
export function useTransformQueue() {
  const [queue, setQueue] = useState([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState(null);
  const [processingProgress, setProcessingProgress] = useState(0);
  const processingRef = useRef(false);
  const apiClient = useApiClient();

  /**
   * Add a transform operation to the queue
   * @param {object} transform - Transform configuration
   * @param {string} transform.type - Transform type (filter, map, aggregate, etc.)
   * @param {object} transform.config - Transform configuration
   * @param {string} transform.name - Transform name
   * @returns {string} Transform ID
   */
  const addToQueue = useCallback((transform) => {
    const transformId = `transform_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    const queueItem = {
      id: transformId,
      type: transform.type,
      name: transform.name || `${transform.type}_${transformId}`,
      config: transform.config || {},
      status: 'pending',
      createdAt: new Date().toISOString(),
      priority: transform.priority || 'normal',
    };

    setQueue(prev => [...prev, queueItem]);
    setError(null);
    
    return transformId;
  }, []);

  /**
   * Remove a transform from the queue
   * @param {string} transformId - ID of transform to remove
   */
  const removeFromQueue = useCallback((transformId) => {
    setQueue(prev => prev.filter(item => item.id !== transformId));
  }, []);

  /**
   * Update transform status in queue
   * @param {string} transformId - Transform ID
   * @param {string} status - New status
   * @param {object} result - Transform result (optional)
   */
  const updateTransformStatus = useCallback((transformId, status, result = null) => {
    setQueue(prev => prev.map(item => 
      item.id === transformId 
        ? { ...item, status, result, updatedAt: new Date().toISOString() }
        : item
    ));
  }, []);

  /**
   * Process all pending transforms in the queue
   * @param {object} options - Processing options
   * @returns {Promise<object>} Processing result
   */
  const processQueue = useCallback(async (options = {}) => {
    if (processingRef.current) {
      throw new Error('Queue is already being processed');
    }

    const pendingTransforms = queue.filter(item => item.status === 'pending');
    if (pendingTransforms.length === 0) {
      return { success: true, processed: 0, errors: [] };
    }

    setIsProcessing(true);
    setError(null);
    setProcessingProgress(0);
    processingRef.current = true;

    const errors = [];
    let processed = 0;

    try {
      for (let i = 0; i < pendingTransforms.length; i++) {
        const transform = pendingTransforms[i];
        
        try {
          updateTransformStatus(transform.id, 'processing');
          
          const response = await apiClient.post('/api/transforms/execute', {
            type: transform.type,
            config: transform.config,
            options: {
              ...options,
              transformId: transform.id,
            },
          });

          updateTransformStatus(transform.id, 'completed', response.data);
          processed++;
          
        } catch (err) {
          updateTransformStatus(transform.id, 'failed', { error: err.message });
          errors.push({
            transformId: transform.id,
            error: err.message,
          });
        }

        setProcessingProgress(((i + 1) / pendingTransforms.length) * 100);
      }

      return {
        success: errors.length === 0,
        processed,
        errors,
        total: pendingTransforms.length,
      };

    } catch (err) {
      setError(err.message || 'Queue processing failed');
      throw err;
    } finally {
      setIsProcessing(false);
      processingRef.current = false;
      setProcessingProgress(0);
    }
  }, [queue, apiClient, updateTransformStatus]);

  /**
   * Clear all transforms from the queue
   * @param {boolean} onlyCompleted - Only clear completed transforms
   */
  const clearQueue = useCallback((onlyCompleted = false) => {
    if (onlyCompleted) {
      setQueue(prev => prev.filter(item => 
        item.status !== 'completed' && item.status !== 'failed'
      ));
    } else {
      setQueue([]);
    }
    setError(null);
  }, []);

  /**
   * Reprocess failed transforms
   * @returns {Promise<object>} Reprocessing result
   */
  const retryFailed = useCallback(async () => {
    const failedTransforms = queue.filter(item => item.status === 'failed');
    
    // Reset failed transforms to pending
    setQueue(prev => prev.map(item => 
      item.status === 'failed' 
        ? { ...item, status: 'pending', result: null }
        : item
    ));

    // Process the queue again
    return await processQueue();
  }, [queue, processQueue]);

  /**
   * Get queue statistics
   * @returns {object} Queue statistics
   */
  const getQueueStats = useCallback(() => {
    const stats = queue.reduce((acc, item) => {
      acc[item.status] = (acc[item.status] || 0) + 1;
      acc.total++;
      return acc;
    }, { pending: 0, processing: 0, completed: 0, failed: 0, total: 0 });

    return stats;
  }, [queue]);

  return {
    // State
    queue,
    isProcessing,
    error,
    processingProgress,

    // Methods
    addToQueue,
    removeFromQueue,
    processQueue,
    clearQueue,
    retryFailed,
    updateTransformStatus,
    getQueueStats,
  };
}

export default useTransformQueue;