import type { ApiResponse } from '../types/api';
import { signPayload } from './authenticationWrapper';

/**
 * Shared HTTP client utilities for consistent API communication
 * Consolidates duplicate GET and POST logic from API clients
 */

export async function get<T>(baseUrl: string, endpoint: string): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${baseUrl}${endpoint}`, {
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

export async function post<T>(baseUrl: string, endpoint: string, body: any): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${baseUrl}${endpoint}`, {
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
    
    // The backend sometimes returns success without a data field
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
 * Performs a signed POST request with authentication
 * Consolidates the signing logic used across API clients
 */
export async function signedPost<T>(baseUrl: string, endpoint: string, body: any): Promise<ApiResponse<T>> {
  try {
    // Sign the payload using the authentication wrapper
    const signedMessage = await signPayload(body);
    
    const response = await fetch(`${baseUrl}${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Signed-Request': 'true',
      },
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
 * Performs a signed message POST request (for mutations/queries)
 * Handles pre-signed messages (SignedMessage objects)
 */
export async function signedMessagePost<T>(endpoint: string, signedMessage: any): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(endpoint, {
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