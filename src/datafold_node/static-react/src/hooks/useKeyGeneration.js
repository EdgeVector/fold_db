/**
 * @fileoverview Key Generation Hook
 * 
 * Provides functionality for generating cryptographic key pairs and managing
 * key generation state. Used by KeyManagement components.
 * 
 * @module useKeyGeneration
 * @since 2.0.0
 */

import { useState, useCallback } from 'react';
import { generateKeyPair, signPayload, verifySignature } from '../utils/cryptoUtils';
import { Buffer } from 'buffer';
import { registerPublicKey as registerPublicKeyApi } from '../api/clients/securityClient';

/**
 * Hook for managing key generation operations
 * @returns {object} Key generation state and methods
 */
export function useKeyGeneration() {
  const [keyPair, setKeyPair] = useState(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState(null);
  const [generationHistory, setGenerationHistory] = useState([]);

  /**
   * Generate a new key pair
   * @param {object} options - Generation options
   * @returns {Promise<object>} Generated key pair
   */
  const generateKeys = useCallback(async (options = {}) => {
    setIsGenerating(true);
    setError(null);

    try {
      const newKeyPair = await generateKeyPair();
      
      const keyPairWithMetadata = {
        ...newKeyPair,
        id: `keypair_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        createdAt: new Date().toISOString(),
        algorithm: 'Ed25519',
        ...options,
      };

      setKeyPair(keyPairWithMetadata);
      
      // Add to history
      setGenerationHistory(prev => [...prev, {
        id: keyPairWithMetadata.id,
        createdAt: keyPairWithMetadata.createdAt,
        algorithm: keyPairWithMetadata.algorithm,
      }].slice(-10)); // Keep last 10 generations

      return keyPairWithMetadata;
    } catch (err) {
      const errorMessage = err.message || 'Key generation failed';
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      setIsGenerating(false);
    }
  }, []);

  /**
   * Clear the current key pair
   */
  const clearKeys = useCallback(() => {
    setKeyPair(null);
    setError(null);
  }, []);

  /**
   * Export key pair in various formats
   * @param {string} format - Export format ('pem', 'base64', 'hex')
   * @returns {object} Exported keys
   */
  const exportKeys = useCallback((format = 'base64') => {
    if (!keyPair) {
      throw new Error('No key pair available for export');
    }

    switch (format) {
      case 'base64':
        return {
          privateKey: keyPair.privateKeyBase64,
          publicKey: keyPair.publicKeyBase64,
          format: 'base64',
        };
      case 'hex':
        return {
          privateKey: Buffer.from(keyPair.privateKey).toString('hex'),
          publicKey: Buffer.from(keyPair.publicKey).toString('hex'),
          format: 'hex',
        };
      case 'raw':
        return {
          privateKey: keyPair.privateKey,
          publicKey: keyPair.publicKey,
          format: 'raw',
        };
      default:
        throw new Error(`Unsupported export format: ${format}`);
    }
  }, [keyPair]);

  /**
   * Validate key pair integrity
   * @returns {Promise<boolean>} True if keys are valid
   */
  const validateKeys = useCallback(async () => {
    if (!keyPair) {
      return false;
    }

    try {
      // Test signing and verification to validate key pair
      const testMessage = 'key_validation_test';

      
      const signature = await signPayload(testMessage, keyPair.privateKeyBase64);
      const isValid = await verifySignature(signature, testMessage, keyPair.publicKeyBase64);
      
      return isValid;
    } catch (err) {
      console.error('Key validation failed:', err);
      return false;
    }
  }, [keyPair]);

  /**
   * Get key pair information
   * @returns {object} Key pair metadata
   */
  const getKeyInfo = useCallback(() => {
    if (!keyPair) {
      return null;
    }

    return {
      id: keyPair.id,
      algorithm: keyPair.algorithm,
      createdAt: keyPair.createdAt,
      hasPrivateKey: !!keyPair.privateKey,
      hasPublicKey: !!keyPair.publicKey,
      keySize: keyPair.privateKey ? keyPair.privateKey.length * 8 : null,
    };
  }, [keyPair]);

  /**
   * Register a public key with the system
   * @param {string} publicKeyBase64 - Base64 encoded public key
   * @returns {Promise<boolean>} True if registration was successful
   */
  const registerPublicKey = useCallback(async (publicKeyBase64) => {
    try {
      const requestBody = {
        public_key: publicKeyBase64,
        owner_id: 'web-user', // Default owner ID for web interface
        permissions: ['read', 'write'], // Default permissions
        metadata: {
          generated_by: 'web-interface',
          generation_time: new Date().toISOString(),
          key_type: 'ed25519'
        },
        expires_at: null // No expiration by default
      };

      const response = await registerPublicKeyApi(requestBody);
      console.log('registerPublicKey response:', response);
      
      // Check for success in the API client response structure
      const success = response.success ?? false;
      
      // Additional validation: check if backend also reports success
      if (success && response.data && typeof response.data === 'object' && 'success' in response.data) {
        const backendSuccess = response.data.success ?? false;
        console.log('Backend success:', backendSuccess, 'API success:', success);
        return success && backendSuccess;
      }
      
      return success;
    } catch (error) {
      console.error('registerPublicKey error:', error);
      return false;
    }
  }, []);

  return {
    // State
    keyPair,
    isGenerating,
    error,
    generationHistory,

    // Methods
    generateKeys,
    clearKeys,
    exportKeys,
    validateKeys,
    getKeyInfo,
    registerPublicKey,
  };
}

export default useKeyGeneration;