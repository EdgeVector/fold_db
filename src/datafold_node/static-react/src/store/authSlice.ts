import { createSlice, createAsyncThunk, PayloadAction } from '@reduxjs/toolkit';
import { getSystemPublicKey } from '../api/clients/securityClient';
import { base64ToBytes } from '../utils/cryptoUtils';
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Set up SHA-512 hash function for ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export interface KeyAuthenticationState {
  isAuthenticated: boolean;
  systemPublicKey: string | null;
  systemKeyId: string | null;
  privateKey: Uint8Array | null;
  publicKeyId: string | null;
  isLoading: boolean;
  error: string | null;
}

const initialState: KeyAuthenticationState = {
  isAuthenticated: false,
  systemPublicKey: null,
  systemKeyId: null,
  privateKey: null,
  publicKeyId: null,
  isLoading: false,
  error: null,
};

// Async thunk for initializing system key on startup
export const initializeSystemKey = createAsyncThunk(
  'auth/initializeSystemKey',
  async (_, { rejectWithValue }) => {
    try {
      const response = await getSystemPublicKey();
      console.log('initializeSystemKey thunk response:', response);
      if (response.success && (response as any).data && (response as any).data.key && (response as any).data.key.public_key) {
        return {
          systemPublicKey: (response as any).data.key.public_key,
          systemKeyId: (response as any).data.key.id || null,
        };
      } else {
        return {
          systemPublicKey: null,
          systemKeyId: null,
        };
      }
    } catch (err) {
      console.error('Failed to fetch system public key:', err);
      return rejectWithValue(err instanceof Error ? err.message : 'Failed to fetch system key');
    }
  }
);

// Async thunk for validating private key
export const validatePrivateKey = createAsyncThunk(
  'auth/validatePrivateKey',
  async (privateKeyBase64: string, { getState, rejectWithValue }) => {
    const state = getState() as { auth: KeyAuthenticationState };
    const { systemPublicKey, systemKeyId } = state.auth;

    if (!systemPublicKey || !systemKeyId) {
      return rejectWithValue('System public key not available');
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
        return {
          privateKey: privateKeyBytes,
          publicKeyId: systemKeyId,
          isAuthenticated: true
        };
      } else {
        return rejectWithValue('Private key does not match system public key');
      }
    } catch (err) {
      console.error('Private key validation failed:', err);
      return rejectWithValue(err instanceof Error ? err.message : 'Private key validation failed');
    }
  }
);

// Async thunk for refreshing system key
export const refreshSystemKey = createAsyncThunk(
  'auth/refreshSystemKey',
  async (_, { rejectWithValue }) => {
    // Retry logic to handle race condition with backend key registration
    const maxRetries = 5;
    const retryDelay = 200; // Start with 200ms
    
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        const response = await getSystemPublicKey();
        
        if (response.success && (response as any).data && (response as any).data.key && (response as any).data.key.public_key) {
          return {
            systemPublicKey: (response as any).data.key.public_key,
            systemKeyId: (response as any).data.key.id || null,
          };
        } else {
          if (attempt < maxRetries) {
            const delay = retryDelay * attempt; // Exponential backoff
            await new Promise(resolve => setTimeout(resolve, delay));
          }
        }
      } catch (err) {
        if (attempt === maxRetries) {
          return rejectWithValue(err instanceof Error ? err.message : 'Failed to fetch system key');
        } else {
          const delay = retryDelay * attempt;
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }
    
    return rejectWithValue('Failed to fetch system key after multiple attempts');
  }
);

const authSlice = createSlice({
  name: 'auth',
  initialState,
  reducers: {
    clearAuthentication: (state) => {
      state.isAuthenticated = false;
      state.privateKey = null;
      state.publicKeyId = null;
      state.error = null;
    },
    setError: (state, action: PayloadAction<string>) => {
      state.error = action.payload;
    },
    clearError: (state) => {
      state.error = null;
    },
    updateSystemKey: (state, action: PayloadAction<{ systemPublicKey: string; systemKeyId: string }>) => {
      state.systemPublicKey = action.payload.systemPublicKey;
      state.systemKeyId = action.payload.systemKeyId;
      state.error = null;
    },
  },
  extraReducers: (builder) => {
    builder
      // initializeSystemKey cases
      .addCase(initializeSystemKey.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(initializeSystemKey.fulfilled, (state, action) => {
        state.isLoading = false;
        state.systemPublicKey = action.payload.systemPublicKey;
        state.systemKeyId = action.payload.systemKeyId;
        state.error = null;
      })
      .addCase(initializeSystemKey.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      // validatePrivateKey cases
      .addCase(validatePrivateKey.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(validatePrivateKey.fulfilled, (state, action) => {
        state.isLoading = false;
        state.isAuthenticated = action.payload.isAuthenticated;
        state.privateKey = action.payload.privateKey;
        state.publicKeyId = action.payload.publicKeyId;
        state.error = null;
      })
      .addCase(validatePrivateKey.rejected, (state, action) => {
        state.isLoading = false;
        state.isAuthenticated = false;
        state.privateKey = null;
        state.publicKeyId = null;
        state.error = action.payload as string;
      })
      // refreshSystemKey cases
      .addCase(refreshSystemKey.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(refreshSystemKey.fulfilled, (state, action) => {
        state.isLoading = false;
        state.systemPublicKey = action.payload.systemPublicKey;
        state.systemKeyId = action.payload.systemKeyId;
        state.error = null;
      })
      .addCase(refreshSystemKey.rejected, (state, action) => {
        state.isLoading = false;
        state.systemPublicKey = null;
        state.systemKeyId = null;
        state.error = action.payload as string;
      });
  },
});

export const { clearAuthentication, setError, clearError, updateSystemKey } = authSlice.actions;

export default authSlice.reducer;