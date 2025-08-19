/**
 * @fileoverview Key Lifecycle Hook
 * 
 * Provides functionality for managing cryptographic key lifecycle operations
 * including storage, rotation, expiration, and revocation.
 * 
 * @module useKeyLifecycle
 * @since 2.0.0
 */

import { useState, useCallback, useEffect } from 'react';
import { useApiClient } from './useApiClient';

/**
 * Hook for managing key lifecycle operations
 * @returns {object} Key lifecycle state and methods
 */
export function useKeyLifecycle() {
  const [keys, setKeys] = useState([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [operationHistory, setOperationHistory] = useState([]);
  const apiClient = useApiClient();

  /**
   * Store a key with lifecycle metadata
   * @param {object} keyData - Key data to store
   * @param {object} options - Storage options
   * @returns {Promise<object>} Stored key info
   */
  const storeKey = useCallback(async (keyData, options = {}) => {
    setIsLoading(true);
    setError(null);

    try {
      const keyRecord = {
        id: keyData.id || `key_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        publicKey: keyData.publicKeyBase64 || keyData.publicKey,
        algorithm: keyData.algorithm || 'Ed25519',
        status: 'active',
        createdAt: new Date().toISOString(),
        expiresAt: options.expirationDate || null,
        metadata: {
          purpose: options.purpose || 'general',
          description: options.description || '',
          tags: options.tags || [],
        },
      };

      const response = await apiClient.post('/api/keys', keyRecord);
      
      const storedKey = response.data;
      setKeys(prev => [...prev, storedKey]);
      
      // Add to operation history
      setOperationHistory(prev => [...prev, {
        operation: 'store',
        keyId: storedKey.id,
        timestamp: new Date().toISOString(),
        status: 'success',
      }].slice(-50)); // Keep last 50 operations

      return storedKey;
    } catch (err) {
      const errorMessage = err.message || 'Failed to store key';
      setError(errorMessage);
      
      setOperationHistory(prev => [...prev, {
        operation: 'store',
        keyId: keyData.id,
        timestamp: new Date().toISOString(),
        status: 'failed',
        error: errorMessage,
      }].slice(-50));
      
      throw new Error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [apiClient]);

  /**
   * Rotate a key (mark old as revoked, create new one)
   * @param {string} keyId - ID of key to rotate
   * @param {object} newKeyData - New key data
   * @returns {Promise<object>} New key record
   */
  const rotateKey = useCallback(async (keyId, newKeyData) => {
    setIsLoading(true);
    setError(null);

    try {
      // First revoke the old key
      await apiClient.patch(`/api/keys/${keyId}`, {
        status: 'revoked',
        revokedAt: new Date().toISOString(),
        revocationReason: 'rotation',
      });

      // Then store the new key
      const newKey = await storeKey(newKeyData, {
        purpose: 'rotation_replacement',
        description: `Rotation replacement for key ${keyId}`,
      });

      // Update local state
      setKeys(prev => prev.map(key => 
        key.id === keyId 
          ? { ...key, status: 'revoked', revokedAt: new Date().toISOString() }
          : key
      ));

      setOperationHistory(prev => [...prev, {
        operation: 'rotate',
        oldKeyId: keyId,
        newKeyId: newKey.id,
        timestamp: new Date().toISOString(),
        status: 'success',
      }].slice(-50));

      return newKey;
    } catch (err) {
      const errorMessage = err.message || 'Failed to rotate key';
      setError(errorMessage);
      
      setOperationHistory(prev => [...prev, {
        operation: 'rotate',
        keyId,
        timestamp: new Date().toISOString(),
        status: 'failed',
        error: errorMessage,
      }].slice(-50));
      
      throw new Error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [apiClient, storeKey]);

  /**
   * Revoke a key
   * @param {string} keyId - ID of key to revoke
   * @param {string} reason - Revocation reason
   * @returns {Promise<void>}
   */
  const revokeKey = useCallback(async (keyId, reason = 'manual_revocation') => {
    setIsLoading(true);
    setError(null);

    try {
      await apiClient.patch(`/api/keys/${keyId}`, {
        status: 'revoked',
        revokedAt: new Date().toISOString(),
        revocationReason: reason,
      });

      // Update local state
      setKeys(prev => prev.map(key => 
        key.id === keyId 
          ? { 
              ...key, 
              status: 'revoked', 
              revokedAt: new Date().toISOString(),
              revocationReason: reason,
            }
          : key
      ));

      setOperationHistory(prev => [...prev, {
        operation: 'revoke',
        keyId,
        reason,
        timestamp: new Date().toISOString(),
        status: 'success',
      }].slice(-50));

    } catch (err) {
      const errorMessage = err.message || 'Failed to revoke key';
      setError(errorMessage);
      
      setOperationHistory(prev => [...prev, {
        operation: 'revoke',
        keyId,
        timestamp: new Date().toISOString(),
        status: 'failed',
        error: errorMessage,
      }].slice(-50));
      
      throw new Error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [apiClient]);

  /**
   * Get all keys with optional filtering
   * @param {object} filters - Filter options
   * @returns {Promise<Array>} Filtered keys
   */
  const getKeys = useCallback(async (filters = {}) => {
    setIsLoading(true);
    setError(null);

    try {
      const params = new URLSearchParams();
      if (filters.status) params.append('status', filters.status);
      if (filters.purpose) params.append('purpose', filters.purpose);
      if (filters.algorithm) params.append('algorithm', filters.algorithm);

      const response = await apiClient.get(`/api/keys?${params.toString()}`);
      const fetchedKeys = response.data;
      
      setKeys(fetchedKeys);
      return fetchedKeys;
    } catch (err) {
      const errorMessage = err.message || 'Failed to fetch keys';
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [apiClient]);

  /**
   * Check for expired keys and handle them
   * @returns {Promise<Array>} List of expired keys
   */
  const checkExpiredKeys = useCallback(async () => {
    const now = new Date().toISOString();
    const expiredKeys = keys.filter(key => 
      key.expiresAt && 
      key.expiresAt < now && 
      key.status === 'active'
    );

    // Automatically mark expired keys as inactive
    for (const key of expiredKeys) {
      try {
        await apiClient.patch(`/api/keys/${key.id}`, {
          status: 'expired',
          expiredAt: now,
        });
      } catch (err) {
        console.error(`Failed to update expired key ${key.id}:`, err);
      }
    }

    if (expiredKeys.length > 0) {
      // Refresh keys after updating expired ones
      await getKeys();
    }

    return expiredKeys;
  }, [keys, apiClient, getKeys]);

  /**
   * Get key statistics
   * @returns {object} Key statistics
   */
  const getKeyStats = useCallback(() => {
    const stats = keys.reduce((acc, key) => {
      acc.total++;
      acc.byStatus[key.status] = (acc.byStatus[key.status] || 0) + 1;
      acc.byAlgorithm[key.algorithm] = (acc.byAlgorithm[key.algorithm] || 0) + 1;
      return acc;
    }, {
      total: 0,
      byStatus: {},
      byAlgorithm: {},
    });

    return stats;
  }, [keys]);

  // Auto-check for expired keys on mount and periodically
  useEffect(() => {
    const checkExpiration = () => {
      if (keys.length > 0) {
        checkExpiredKeys().catch(err => {
          console.error('Failed to check expired keys:', err);
        });
      }
    };

    // Check immediately
    checkExpiration();

    // Check every hour
    const interval = setInterval(checkExpiration, 60 * 60 * 1000);
    return () => clearInterval(interval);
  }, [keys.length, checkExpiredKeys]);

  return {
    // State
    keys,
    isLoading,
    error,
    operationHistory,

    // Methods
    storeKey,
    rotateKey,
    revokeKey,
    getKeys,
    checkExpiredKeys,
    getKeyStats,
  };
}

export default useKeyLifecycle;