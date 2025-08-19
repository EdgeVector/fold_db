// Custom hook for Ed25519 key generation and management

import { useState, useCallback } from 'react';
import type {
  KeyGenerationState,
  KeyGenerationResult,
  KeyRegistrationRequest,
} from '../types/cryptography';
import type { ApiResponse } from '../types/api';
import { generateEd25519KeyPair, bytesToBase64 } from '../utils/ed25519';
import { registerPublicKey as registerPublicKeyApi } from '../api/securityClient';

const INITIAL_RESULT: KeyGenerationResult = {
  keyPair: null,
  publicKeyBase64: null,
  error: null,
  isGenerating: false,
};

export function useKeyGeneration(): KeyGenerationState {
  const [keyPair, setKeyPair] = useState(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState(null);
  const [generationHistory, setGenerationHistory] = useState([]);

  const generateKeys = useCallback(async () => {
    setIsGenerating(true);
    setError(null);

    try {
      const newKeyPair = await generateEd25519KeyPair();
      const publicKeyBase64 = bytesToBase64(newKeyPair.publicKey);
      
      const keyPairWithMetadata = {
        ...newKeyPair,
        id: `keypair_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        createdAt: new Date().toISOString(),
        algorithm: 'Ed25519',
        publicKeyBase64,
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

  const clearKeys = useCallback(() => {
    setKeyPair(null);
    setError(null);
  }, []);

  const registerPublicKey = useCallback(async (publicKeyBase64: string): Promise<boolean> => {
    try {
      const requestBody: KeyRegistrationRequest = {
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

      const response: ApiResponse = await registerPublicKeyApi(requestBody);
      console.log('registerPublicKey response:', response);
      
      // Check for success in the API client response structure
      const success = response.success ?? false;
      
      // Additional validation: check if backend also reports success
      if (success && response.data && typeof response.data === 'object' && 'success' in response.data) {
        const backendSuccess = (response.data as any).success ?? false;
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
    keyPair,
    isGenerating,
    error,
    generationHistory,
    generateKeys,
    clearKeys,
    registerPublicKey,
  };
}