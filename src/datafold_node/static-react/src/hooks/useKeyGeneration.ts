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
  const [result, setResult] = useState<KeyGenerationResult>(INITIAL_RESULT);

  const generateKeyPair = useCallback(async () => {
    setResult(prev => ({ ...prev, isGenerating: true, error: null }));
    
    try {
      const keyPair = await generateEd25519KeyPair();
      const publicKeyBase64 = bytesToBase64(keyPair.publicKey);

      setResult({
        keyPair,
        publicKeyBase64,
        error: null,
        isGenerating: false,
      });
    } catch (error) {
      setResult({
        keyPair: null,
        publicKeyBase64: null,
        error: error instanceof Error ? error.message : 'Failed to generate keypair',
        isGenerating: false,
      });
    }
  }, []);

  const clearKeys = useCallback(() => {
    setResult(INITIAL_RESULT);
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
    result,
    generateKeyPair,
    clearKeys,
    registerPublicKey,
  };
}