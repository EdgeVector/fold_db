import type { ApiResponse } from '../types/api';

/**
 * @deprecated This module is deprecated as part of API-STD-1 TASK-005.
 * All consumers have been migrated to use specialized API clients.
 *
 * Migration guide:
 * - Basic GET/POST operations: Use appropriate specialized client (schemaClient, systemClient, etc.)
 * - Signed operations: Use securityClient or mutationClient
 * - Message operations: Use mutationClient for query/mutation execution
 *
 * This module will be removed in a future release.
 */

/**
 * @deprecated Use specialized API clients instead of httpClient utilities.
 * For basic GET operations, use the appropriate client (schemaClient, systemClient, etc.)
 */
export async function get<T>(_baseUrl: string, _endpoint: string): Promise<ApiResponse<T>> {
  throw new Error(
    'httpClient.get() is deprecated and removed. Use specialized API clients instead. ' +
    'See API-STD-1 migration guide for details.'
  );
}

/**
 * @deprecated Use specialized API clients instead of httpClient utilities.
 * For basic POST operations, use the appropriate client (schemaClient, systemClient, etc.)
 */
export async function post<T>(_baseUrl: string, _endpoint: string, _body: unknown): Promise<ApiResponse<T>> {
  throw new Error(
    'httpClient.post() is deprecated and removed. Use specialized API clients instead. ' +
    'See API-STD-1 migration guide for details.'
  );
}

/**
 * @deprecated Use securityClient or mutationClient for signed operations.
 */
export async function signedPost<T>(_baseUrl: string, _endpoint: string, _body: unknown): Promise<ApiResponse<T>> {
  throw new Error(
    'httpClient.signedPost() is deprecated and removed. Use securityClient or mutationClient instead. ' +
    'See API-STD-1 migration guide for details.'
  );
}

/**
 * @deprecated Use mutationClient.executeQuery() or mutationClient.executeMutation() for message operations.
 */
export async function signedMessagePost<T>(_endpoint: string, _signedMessage: unknown): Promise<ApiResponse<T>> {
  throw new Error(
    'httpClient.signedMessagePost() is deprecated and removed. Use mutationClient for query/mutation execution. ' +
    'See API-STD-1 migration guide for details.'
  );
}