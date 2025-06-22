import type { ApiResponse } from '../types/api';

const API_BASE_URL = '/api/schemas';

interface Schema {
  name: string;
  fields: Record<string, any>;
  payment_config: any;
  hash?: string;
}

interface SchemaWithState {
  name: string;
  state: 'available' | 'approved' | 'blocked';
  schema?: Schema;
}

interface SchemasByStateResponse {
  data: string[];
  state: string;
}

interface SchemasWithStateResponse {
  data: Record<string, string>;
}

async function get<T>(endpoint: string): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      try {
        const errorData = await response.json();
        return {
          success: false,
          error: errorData.error || `HTTP error! status: ${response.status}`,
        };
      } catch (e) {
        return {
          success: false,
          error: `HTTP error! status: ${response.status}`,
        };
      }
    }
    
    const responseData = await response.json();
    return {
      success: true,
      ...responseData,
    };

  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'An unknown network error occurred',
    };
  }
}

async function post<T>(endpoint: string, body: any): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      try {
        const errorData = await response.json();
        return {
          success: false,
          error: errorData.error || `HTTP error! status: ${response.status}`,
        };
      } catch (e) {
        return {
          success: false,
          error: `HTTP error! status: ${response.status}`,
        };
      }
    }
    
    const responseData = await response.json();
    return {
      success: true,
      ...responseData,
    };

  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'An unknown network error occurred',
    };
  }
}

export async function getSchemasByState(
  state: 'available' | 'approved' | 'blocked'
): Promise<ApiResponse<SchemasByStateResponse>> {
  return get<SchemasByStateResponse>(`/state/${state}`);
}

export async function getAllSchemasWithState(): Promise<ApiResponse<SchemasWithStateResponse>> {
  return get<SchemasWithStateResponse>('');
}

export async function getSchema(name: string): Promise<ApiResponse<Schema>> {
  return get<Schema>(`/${name}`);
}

export async function approveSchema(name: string): Promise<ApiResponse<any>> {
  return post(`/${name}/approve`, {});
}

export async function blockSchema(name: string): Promise<ApiResponse<any>> {
  return post(`/${name}/block`, {});
}

export async function getSchemaStatus(): Promise<ApiResponse<any>> {
  return get('/status');
}