// No backend auth endpoints; UI does not require authentication.
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
  const login = useCallback(async (_username, _password) => {
    setIsLoading(true);
    try {
      // No-op login: store a session flag locally
      const pseudoUser = { name: 'local-user' };
      const pseudoToken = 'local-session';

      setToken(pseudoToken);
      setUser(pseudoUser);

      localStorage.setItem('auth_token', pseudoToken);
      localStorage.setItem('auth_user', JSON.stringify(pseudoUser));

      return { success: true, user: pseudoUser };
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
    // No backend token; just return the existing local token
    return token;
  }, [token]);

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