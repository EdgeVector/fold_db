/**
 * Indexing Status Client
 * 
 * Provides methods for querying the background indexing system status.
 */

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


