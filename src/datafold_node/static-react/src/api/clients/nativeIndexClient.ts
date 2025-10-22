import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
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
    this.client = client || createApiClient({ enableCache: true, enableLogging: true });
  }

  async search(term: string): Promise<EnhancedApiResponse<NativeIndexResult[]>> {
    const url = `${API_ENDPOINTS.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(term)}`;
    return this.client.get<NativeIndexResult[]>(url, {
      timeout: 8000,
      retries: 2,
      cacheable: true,
      cacheTtl: 60000,
    });
  }
}

export const nativeIndexClient = new NativeIndexClient();
export default nativeIndexClient;
