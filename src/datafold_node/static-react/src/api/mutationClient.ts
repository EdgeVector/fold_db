import type { ApiResponse } from '../types/api';
import type { SignedMessage } from '../types/cryptography';
import { API_ENDPOINTS } from './endpoints';

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
    try {
      const response = await fetch(API_ENDPOINTS.MUTATION, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(signedMessage),
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

  /**
   * Execute a query
   */
  static async executeQuery(signedMessage: SignedMessage): Promise<ApiResponse<any>> {
    try {
      const response = await fetch(API_ENDPOINTS.QUERY, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(signedMessage),
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
}

// Legacy exports for backward compatibility
export const executeMutation = MutationClient.executeMutation;
export const executeQuery = MutationClient.executeQuery;