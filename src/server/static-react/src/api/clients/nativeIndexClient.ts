import { ApiClient, getSharedClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES, API_CACHE_TTL } from '../../constants/api';
import type { EnhancedApiResponse } from '../core/types';

export interface NativeIndexResult {
  schema_name: string;
  field: string;
  key_value: { hash?: string | null; range?: string | null };
  value: unknown;
  metadata?: Record<string, unknown> | null;
}

export class NativeIndexClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || getSharedClient();
  }

  async search(term: string): Promise<EnhancedApiResponse<NativeIndexResult[]>> {
    if (!term || typeof term !== 'string') {
      return { success: false, error: 'Search term is required', status: 400, data: [] };
    }
    const url = `${API_ENDPOINTS.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(term)}`;
    return this.client.get<NativeIndexResult[]>(url, {
      timeout: API_TIMEOUTS.STANDARD,
      retries: API_RETRIES.STANDARD,
      cacheable: true,
      cacheTtl: API_CACHE_TTL.QUERY_RESULTS,
    });
  }
}

export const nativeIndexClient = new NativeIndexClient();
export default nativeIndexClient;

