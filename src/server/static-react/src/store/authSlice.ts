import { createSlice, createAsyncThunk, PayloadAction } from '@reduxjs/toolkit';
import { getNodePrivateKey } from '../api/clients/systemClient';
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
      // Fetch the private key directly from the node
      const response = await getNodePrivateKey();
      console.log('initializeSystemKey thunk response:', response);
      
      if (response.success && response.data && response.data.private_key) {
        // Convert base64 private key to bytes
        const privateKeyBytes = base64ToBytes(response.data.private_key);
        
        // Generate public key from private key for verification
        const derivedPublicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
        const derivedPublicKeyBase64 = btoa(String.fromCharCode(...derivedPublicKeyBytes));
        
        return {
          systemPublicKey: derivedPublicKeyBase64,
          systemKeyId: 'node-private-key',
          privateKey: privateKeyBytes,
          isAuthenticated: true
        };
      } else {
        return {
          systemPublicKey: null,
          systemKeyId: null,
          privateKey: null,
          isAuthenticated: false
        };
      }
    } catch (err) {
      console.error('Failed to fetch node private key:', err);
      return rejectWithValue(err instanceof Error ? err.message : 'Failed to fetch node private key');
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
        const response = await getNodePrivateKey();
        
        if (response.success && response.data && response.data.private_key) {
          // Convert base64 private key to bytes
          const privateKeyBytes = base64ToBytes(response.data.private_key);
          
          // Generate public key from private key for verification
          const derivedPublicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
          const derivedPublicKeyBase64 = btoa(String.fromCharCode(...derivedPublicKeyBytes));
          
          return {
            systemPublicKey: derivedPublicKeyBase64,
            systemKeyId: 'node-private-key',
            privateKey: privateKeyBytes,
            isAuthenticated: true
          };
        } else {
          if (attempt < maxRetries) {
            const delay = retryDelay * attempt; // Exponential backoff
            await new Promise(resolve => setTimeout(resolve, delay));
          }
        }
      } catch (err) {
        if (attempt === maxRetries) {
          return rejectWithValue(err instanceof Error ? err.message : 'Failed to fetch node private key');
        } else {
          const delay = retryDelay * attempt;
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }
    
    return rejectWithValue('Failed to fetch node private key after multiple attempts');
  }
);

// Async thunk for fetching node private key from backend
export const fetchNodePrivateKey = createAsyncThunk(
  'auth/fetchNodePrivateKey',
  async (_, { rejectWithValue }) => {
    try {
      const response = await getNodePrivateKey();
      console.log('fetchNodePrivateKey thunk response:', response);
      
      if (response.success && response.data && response.data.private_key) {
        // Convert base64 private key to bytes
        const privateKeyBytes = base64ToBytes(response.data.private_key);
        
        return {
          privateKey: privateKeyBytes,
          publicKeyId: 'node-private-key', // Use a consistent identifier
          isAuthenticated: true
        };
      } else {
        return rejectWithValue('Failed to fetch private key from backend');
      }
    } catch (err) {
      console.error('Failed to fetch node private key:', err);
      return rejectWithValue(err instanceof Error ? err.message : 'Failed to fetch node private key');
    }
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
        state.privateKey = action.payload.privateKey;
        state.isAuthenticated = action.payload.isAuthenticated;
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
        state.privateKey = action.payload.privateKey;
        state.isAuthenticated = action.payload.isAuthenticated;
        state.error = null;
      })
      .addCase(refreshSystemKey.rejected, (state, action) => {
        state.isLoading = false;
        state.systemPublicKey = null;
        state.systemKeyId = null;
        state.error = action.payload as string;
      })
      // fetchNodePrivateKey cases
      .addCase(fetchNodePrivateKey.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(fetchNodePrivateKey.fulfilled, (state, action) => {
        state.isLoading = false;
        state.isAuthenticated = action.payload.isAuthenticated;
        state.privateKey = action.payload.privateKey;
        state.publicKeyId = action.payload.publicKeyId;
        state.error = null;
      })
      .addCase(fetchNodePrivateKey.rejected, (state, action) => {
        state.isLoading = false;
        state.isAuthenticated = false;
        state.privateKey = null;
        state.publicKeyId = null;
        state.error = action.payload as string;
      });
  },
});

export const { clearAuthentication, setError, clearError, updateSystemKey } = authSlice.actions;

export default authSlice.reducer;