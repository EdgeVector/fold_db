/**
 * Indexing Status Client
 * 
 * Provides methods for querying the background indexing system status.
 */

import * as React from 'react';
import { defaultApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';

export interface IndexingStatus {
  state: 'Idle' | 'Indexing';
  operations_in_progress: number;
  total_operations_processed: number;
  operations_queued: number;
  last_operation_time: number | null;
  avg_processing_time_ms: number;
  operations_per_second: number;
  current_batch_size: number | null;
  current_batch_start_time: number | null;
}

/**
 * Get the current indexing status
 */
export async function getIndexingStatus(): Promise<IndexingStatus> {
  const response = await defaultApiClient.get<IndexingStatus>(
    API_ENDPOINTS.GET_INDEXING_STATUS,
    { cacheable: false } // Don't cache - we need real-time status updates
  );
  return response.data;
}

/**
 * Hook to poll indexing status at regular intervals
 */
export function useIndexingStatus(pollInterval: number = 1000) {
  const [status, setStatus] = React.useState<IndexingStatus | null>(null);
  const [error, setError] = React.useState<Error | null>(null);

  React.useEffect(() => {
    let mounted = true;
    let timeoutId: NodeJS.Timeout;

    const fetchStatus = async () => {
      try {
        const newStatus = await getIndexingStatus();
        if (mounted) {
          setStatus(newStatus);
          setError(null);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err : new Error('Failed to fetch indexing status'));
        }
      }
    };

    // Initial fetch
    fetchStatus();

    // Set up polling
    const poll = async () => {
      await fetchStatus();
      if (mounted) {
        // Adjust polling based on state
        const interval = status?.state === 'Indexing' ? pollInterval : pollInterval * 5;
        timeoutId = setTimeout(poll, interval);
      }
    };

    timeoutId = setTimeout(poll, pollInterval);

    return () => {
      mounted = false;
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
    };
  }, [pollInterval, status?.state]);

  return { status, error };
}

