import { useState, useEffect, useCallback } from 'react';
import { getSystemPublicKey } from '../api/securityClient';
import { base64ToBytes } from '../utils/ed25519';
import * as ed from '@noble/ed25519';

interface KeyAuthenticationState {
  isAuthenticated: boolean;
  systemPublicKey: string | null;
  systemKeyId: string | null;
  isLoading: boolean;
  error: string | null;
  validatePrivateKey: (privateKeyBase64: string) => Promise<boolean>;
  clearAuthentication: () => void;
  refreshSystemKey: () => Promise<void>;
}

export function useKeyAuthentication(): KeyAuthenticationState {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [systemPublicKey, setSystemPublicKey] = useState<string | null>(null);
  const [systemKeyId, setSystemKeyId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch system public key on mount
  useEffect(() => {
    fetchSystemPublicKey();
  }, []);

  const fetchSystemPublicKey = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await getSystemPublicKey();
      
      if (response.success && (response as any).key && (response as any).key.public_key) {
        setSystemPublicKey((response as any).key.public_key);
        setSystemKeyId((response as any).key.id || null);
      } else {
        setSystemPublicKey(null);
        setSystemKeyId(null);
      }
    } catch (err) {
      console.error('Failed to fetch system public key:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch system key');
      setSystemPublicKey(null);
      setSystemKeyId(null);
    } finally {
      setIsLoading(false);
    }
  };

  const validatePrivateKey = useCallback(async (privateKeyBase64: string): Promise<boolean> => {
    if (!systemPublicKey) {
      return false;
    }

    try {
      // Convert base64 private key to bytes
      const privateKeyBytes = base64ToBytes(privateKeyBase64);
      
      // Generate public key from private key
      const derivedPublicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
      const derivedPublicKeyBase64 = btoa(String.fromCharCode(...derivedPublicKeyBytes));
      
      // Check if derived public key matches system public key
      const matches = derivedPublicKeyBase64 === systemPublicKey;
      setIsAuthenticated(matches);
      
      return matches;
    } catch (err) {
      console.error('Private key validation failed:', err);
      setIsAuthenticated(false);
      return false;
    }
  }, [systemPublicKey]);

  const clearAuthentication = useCallback(() => {
    setIsAuthenticated(false);
  }, []);

  const refreshSystemKey = useCallback(async () => {
    // Retry logic to handle race condition with backend key registration
    const maxRetries = 5;
    const retryDelay = 200; // Start with 200ms
    
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      setIsLoading(true);
      setError(null);
      try {
        const response = await getSystemPublicKey();
        
        if (response.success && (response as any).key && (response as any).key.public_key) {
          setSystemPublicKey((response as any).key.public_key);
          setSystemKeyId((response as any).key.id || null);
          setIsLoading(false);
          return; // Success, exit retry loop
        } else {
          if (attempt < maxRetries) {
            const delay = retryDelay * attempt; // Exponential backoff
            await new Promise(resolve => setTimeout(resolve, delay));
          }
        }
      } catch (err) {
        if (attempt === maxRetries) {
          setError(err instanceof Error ? err.message : 'Failed to fetch system key');
          setSystemPublicKey(null);
          setSystemKeyId(null);
        } else {
          const delay = retryDelay * attempt;
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }
    
    setSystemPublicKey(null);
    setSystemKeyId(null);
    setIsLoading(false);
  }, []);

  return {
    isAuthenticated,
    systemPublicKey,
    systemKeyId,
    isLoading,
    error,
    validatePrivateKey,
    clearAuthentication,
    refreshSystemKey,
  };
}