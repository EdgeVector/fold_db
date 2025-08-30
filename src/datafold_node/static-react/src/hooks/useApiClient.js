/**
 * @fileoverview API Client Hook
 * 
 * Provides a configured API client for making HTTP requests.
 * Handles authentication, error handling, and request/response interceptors.
 * 
 * @module useApiClient
 * @since 2.0.0
 */

import { useMemo } from 'react';
import axios from 'axios';
import { useAuth } from './useAuth';

/**
 * Hook for getting a configured API client
 * @returns {object} Configured axios instance
 */
export function useApiClient() {
  const { token } = useAuth?.() || { token: null };

  const apiClient = useMemo(() => {
    const client = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || '/api',
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Request interceptor to add auth token
    client.interceptors.request.use(
      (config) => {
        if (token) {
          config.headers.Authorization = `Bearer ${token}`;
        }
        return config;
      },
      (error) => {
        return Promise.reject(error);
      }
    );

    // Response interceptor for error handling
    client.interceptors.response.use(
      (response) => {
        return response;
      },
      (error) => {
        if (error.response?.status === 401) {
          // Handle unauthorized - could dispatch logout action
          console.warn('Unauthorized request, token may be expired');
        }
        
        // Preserve error structure for tests
        if (error.response?.data) {
          const customError = new Error(error.response.data.message || error.message);
          customError.response = error.response;
          customError.status = error.response.status;
          return Promise.reject(customError);
        }
        
        return Promise.reject(error);
      }
    );

    return client;
  }, [token]);

  return apiClient;
}

export default useApiClient;