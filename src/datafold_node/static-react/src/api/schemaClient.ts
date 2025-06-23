import type { ApiResponse } from '../types/api';
import { signedRequest } from '../utils/authenticationWrapper';
import { get as httpGet, signedPost as httpSignedPost } from '../utils/httpClient';

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
  return httpGet<T>(API_BASE_URL, endpoint);
}


async function signedPost<T>(endpoint: string, body: any): Promise<ApiResponse<T>> {
  return httpSignedPost<T>(API_BASE_URL, endpoint, body);
}

// UNPROTECTED OPERATIONS - No authentication required
export async function getSchemasByState(
  state: 'available' | 'approved' | 'blocked'
): Promise<ApiResponse<SchemasByStateResponse>> {
  return get<SchemasByStateResponse>(`/state/${state}`);
}

export async function getAllSchemasWithState(): Promise<ApiResponse<SchemasWithStateResponse>> {
  return get<SchemasWithStateResponse>('');
}

export async function getSchemaStatus(): Promise<ApiResponse<any>> {
  return get('/status');
}

// PROTECTED OPERATIONS - Require authentication and signing
export async function getSchema(name: string): Promise<ApiResponse<Schema>> {
  return await signedRequest(() => get<Schema>(`/${name}`));
}

export async function approveSchema(name: string): Promise<ApiResponse<any>> {
  return await signedRequest(() => signedPost(`/${name}/approve`, {}));
}

export async function blockSchema(name: string): Promise<ApiResponse<any>> {
  return await signedRequest(() => signedPost(`/${name}/block`, {}));
}
