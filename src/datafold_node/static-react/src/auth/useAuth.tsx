import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { getSystemPublicKey } from '../api/securityClient';
import { base64ToBytes } from '../utils/ed25519';
import * as ed from '@noble/ed25519';

export interface KeyAuthenticationState {
  isAuthenticated: boolean;
  systemPublicKey: string | null;
  systemKeyId: string | null;
  privateKey: Uint8Array | null;
  publicKeyId: string | null;
  isLoading: boolean;
  error: string | null;
  validatePrivateKey: (privateKeyBase64: string) => Promise<boolean>;
  clearAuthentication: () => void;
  refreshSystemKey: () => Promise<void>;
}

// Global instance for non-hook access (needed by AUTH-002)
let globalAuthInstance: KeyAuthenticationState | null = null;

// Create the context
const AuthenticationContext = createContext<KeyAuthenticationState | null>(null);

// Main authentication hook that provides all the logic
function useKeyAuthentication(): KeyAuthenticationState {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [systemPublicKey, setSystemPublicKey] = useState<string | null>(null);
  const [systemKeyId, setSystemKeyId] = useState<string | null>(null);
  const [privateKey, setPrivateKey] = useState<Uint8Array | null>(null);
  const [publicKeyId, setPublicKeyId] = useState<string | null>(null);
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
    console.log('🔑 validatePrivateKey called, systemPublicKey:', systemPublicKey, 'systemKeyId:', systemKeyId);
    if (!systemPublicKey || !systemKeyId) {
      console.log('🔑 Missing systemPublicKey or systemKeyId, returning false');
      return false;
    }

    try {
      // Convert base64 private key to bytes
      console.log('🔑 Converting private key from base64...');
      const privateKeyBytes = base64ToBytes(privateKeyBase64);
      
      // Generate public key from private key
      console.log('🔑 Generating public key from private key...');
      const derivedPublicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
      const derivedPublicKeyBase64 = btoa(String.fromCharCode(...derivedPublicKeyBytes));
      
      // Check if derived public key matches system public key
      const matches = derivedPublicKeyBase64 === systemPublicKey;
      console.log('🔑 Key comparison:', {
        derived: derivedPublicKeyBase64,
        system: systemPublicKey,
        matches
      });
      
      if (matches) {
        console.log('🔑 Keys match! Setting authentication state...');
        // Store private key and public key ID for signing operations
        setPrivateKey(privateKeyBytes);
        setPublicKeyId(systemKeyId);
        setIsAuthenticated(true);
        console.log('🔑 Authentication state set to true');
        
        // Force immediate state propagation with a small delay to ensure React state updates
        setTimeout(() => {
          console.log('🔑 Authentication state should now be propagated');
        }, 50);
      } else {
        console.log('🔑 Keys do not match, clearing authentication state');
        setPrivateKey(null);
        setPublicKeyId(null);
        setIsAuthenticated(false);
      }
      
      return matches;
    } catch (err) {
      console.error('Private key validation failed:', err);
      setPrivateKey(null);
      setPublicKeyId(null);
      setIsAuthenticated(false);
      return false;
    }
  }, [systemPublicKey, systemKeyId]);

  const clearAuthentication = useCallback(() => {
    setIsAuthenticated(false);
    setPrivateKey(null);
    setPublicKeyId(null);
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
    privateKey,
    publicKeyId,
    isLoading,
    error,
    validatePrivateKey,
    clearAuthentication,
    refreshSystemKey,
  };
}

interface AuthenticationProviderProps {
  children: ReactNode;
}

export function AuthenticationProvider({ children }: AuthenticationProviderProps) {
  // Use the internal useKeyAuthentication hook
  const authState = useKeyAuthentication();
  
  // Store global instance for non-hook access
  globalAuthInstance = authState;

  return (
    <AuthenticationContext.Provider value={authState}>
      {children}
    </AuthenticationContext.Provider>
  );
}

// Hook for React components - simple wrapper around context
export function useAuth(): KeyAuthenticationState {
  const context = useContext(AuthenticationContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthenticationProvider');
  }
  return context;
}

// Non-hook access function for AUTH-002's signedRequest() wrapper
export function getAuthContextInstance(): KeyAuthenticationState | null {
  return globalAuthInstance;
}