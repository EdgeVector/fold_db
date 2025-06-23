import type { ApiResponse } from '../types/api';
import type { SignedMessage } from '../types/cryptography';
import { API_ENDPOINTS } from './endpoints';
import { signedMessagePost } from '../utils/httpClient';

interface MutationResponse {
  success: boolean;
  [key: string]: any;
}

/**
 * Centralized mutation API client - prevents endpoint drift
 */
export class MutationClient {
  
  /**
   * Execute a signed mutation
   */
  static async executeMutation(signedMessage: SignedMessage): Promise<ApiResponse<MutationResponse>> {
    return signedMessagePost<MutationResponse>(API_ENDPOINTS.MUTATION, signedMessage);
  }

  /**
   * Execute a query
   */
  static async executeQuery(signedMessage: SignedMessage): Promise<ApiResponse<any>> {
    return signedMessagePost<any>(API_ENDPOINTS.QUERY, signedMessage);
  }
}
