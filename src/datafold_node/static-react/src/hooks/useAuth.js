/**
 * @fileoverview Authentication Hook
 * 
 * Provides authentication state and methods for login/logout functionality.
 * Manages auth tokens and user authentication status.
 * 
 * @module useAuth
 * @since 2.0.0
 */

import { useState, useCallback, useEffect } from 'react';

/**
 * Hook for managing authentication state
 * @returns {object} Authentication state and methods
 */
export function useAuth() {
  const [token, setToken] = useState(() => {
    return localStorage.getItem('auth_token') || null;
  });
  const [user, setUser] = useState(() => {
    const stored = localStorage.getItem('auth_user');
    return stored ? JSON.parse(stored) : null;
  });
  const [isLoading, setIsLoading] = useState(false);

  /**
   * Login with credentials
   * @param {string} username - Username
   * @param {string} password - Password
   * @returns {Promise<object>} Login result
   */
  const login = useCallback(async (username, password) => {
    setIsLoading(true);
    try {
      // In a real app, this would make an API call
      const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ username, password }),
      });

      if (!response.ok) {
        throw new Error('Login failed');
      }

      const data = await response.json();
      
      setToken(data.token);
      setUser(data.user);
      
      localStorage.setItem('auth_token', data.token);
      localStorage.setItem('auth_user', JSON.stringify(data.user));

      return { success: true, user: data.user };
    } catch (error) {
      throw new Error(error.message || 'Login failed');
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Logout and clear auth state
   */
  const logout = useCallback(() => {
    setToken(null);
    setUser(null);
    localStorage.removeItem('auth_token');
    localStorage.removeItem('auth_user');
  }, []);

  /**
   * Check if user is authenticated
   * @returns {boolean}
   */
  const isAuthenticated = useCallback(() => {
    return !!token;
  }, [token]);

  /**
   * Refresh the auth token
   * @returns {Promise<string>} New token
   */
  const refreshToken = useCallback(async () => {
    if (!token) {
      throw new Error('No token to refresh');
    }

    try {
      const response = await fetch('/api/auth/refresh', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const data = await response.json();
      
      setToken(data.token);
      localStorage.setItem('auth_token', data.token);

      return data.token;
    } catch (error) {
      logout(); // Clear invalid token
      throw error;
    }
  }, [token, logout]);

  // Check token validity on mount
  useEffect(() => {
    if (token) {
      // In a real app, you might validate the token here
      // For now, we'll assume it's valid if it exists
    }
  }, [token]);

  return {
    // State
    token,
    user,
    isLoading,

    // Methods
    login,
    logout,
    isAuthenticated,
    refreshToken,
  };
}

export default useAuth;