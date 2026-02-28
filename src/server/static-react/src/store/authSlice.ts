import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import { getNodePrivateKey } from "../api/clients/systemClient";
import { base64ToBytes } from "../utils/cryptoUtils";
import { BROWSER_CONFIG } from "../constants/config";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// Set up SHA-512 hash function for ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export interface KeyAuthenticationState {
  isAuthenticated: boolean;
  user?: {
    id: string;
    hash: string;
  };
  systemPublicKey: string | null;
  systemKeyId: string | null;
  privateKey: string | null; // Base64-encoded private key (serializable)
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
  "auth/initializeSystemKey",
  async (_, { rejectWithValue }) => {
    try {
      // Fetch the private key directly from the node
      const response = await getNodePrivateKey();

      if (response.success && response.data && response.data.private_key) {
        // Convert base64 private key to bytes for key derivation
        const privateKeyBytes = base64ToBytes(response.data.private_key);

        // Generate public key from private key for verification
        const derivedPublicKeyBytes =
          await ed.getPublicKeyAsync(privateKeyBytes);
        const derivedPublicKeyBase64 = btoa(
          String.fromCharCode(...derivedPublicKeyBytes),
        );

        return {
          systemPublicKey: derivedPublicKeyBase64,
          systemKeyId: "node-private-key",
          privateKey: response.data.private_key, // Store as base64 string (serializable)
          isSystemReady: true,
        };
      } else {
        return {
          systemPublicKey: null,
          systemKeyId: null,
          privateKey: null,
          isSystemReady: false,
        };
      }
    } catch (err) {
      console.error("Failed to fetch node private key:", err);
      return rejectWithValue(
        err instanceof Error ? err.message : "Failed to fetch node private key",
      );
    }
  },
);

// Async thunk for validating private key
export const validatePrivateKey = createAsyncThunk(
  "auth/validatePrivateKey",
  async (privateKeyBase64: string, { getState, rejectWithValue }) => {
    const state = getState() as { auth: KeyAuthenticationState };
    const { systemPublicKey, systemKeyId } = state.auth;

    if (!systemPublicKey || !systemKeyId) {
      return rejectWithValue("System public key not available");
    }

    try {
      // Convert base64 private key to bytes
      const privateKeyBytes = base64ToBytes(privateKeyBase64);

      // Generate public key from private key
      const derivedPublicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
      const derivedPublicKeyBase64 = btoa(
        String.fromCharCode(...derivedPublicKeyBytes),
      );

      // Check if derived public key matches system public key
      const matches = derivedPublicKeyBase64 === systemPublicKey;

      if (matches) {
        return {
          privateKey: privateKeyBase64, // Store as base64 string (serializable)
          publicKeyId: systemKeyId,
          isAuthenticated: true,
        };
      } else {
        return rejectWithValue("Private key does not match system public key");
      }
    } catch (err) {
      console.error("Private key validation failed:", err);
      return rejectWithValue(
        err instanceof Error ? err.message : "Private key validation failed",
      );
    }
  },
);

// Async thunk for refreshing system key
export const refreshSystemKey = createAsyncThunk(
  "auth/refreshSystemKey",
  async (_, { rejectWithValue }) => {
    // Retry logic to handle race condition with backend key registration
    const maxRetries = 5;
    const retryDelay = 200; // Start with 200ms

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        const response = await getNodePrivateKey();

        if (response.success && response.data && response.data.private_key) {
          // Convert base64 private key to bytes for key derivation
          const privateKeyBytes = base64ToBytes(response.data.private_key);

          // Generate public key from private key for verification
          const derivedPublicKeyBytes =
            await ed.getPublicKeyAsync(privateKeyBytes);
          const derivedPublicKeyBase64 = btoa(
            String.fromCharCode(...derivedPublicKeyBytes),
          );

          return {
            systemPublicKey: derivedPublicKeyBase64,
            systemKeyId: "node-private-key",
            privateKey: response.data.private_key, // Store as base64 string (serializable)
            isSystemReady: true,
          };
        } else {
          if (attempt < maxRetries) {
            const delay = retryDelay * attempt; // Exponential backoff
            await new Promise((resolve) => setTimeout(resolve, delay));
          }
        }
      } catch (err) {
        if (attempt === maxRetries) {
          return rejectWithValue(
            err instanceof Error
              ? err.message
              : "Failed to fetch node private key",
          );
        } else {
          const delay = retryDelay * attempt;
          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }

    return rejectWithValue(
      "Failed to fetch node private key after multiple attempts",
    );
  },
);

// Async thunk for user login and hash generation
export const loginUser = createAsyncThunk(
  "auth/loginUser",
  async (userId: string, { rejectWithValue }) => {
    try {
      const encoder = new TextEncoder();
      const data = encoder.encode(userId);
      const hashBuffer = await crypto.subtle.digest("SHA-256", data);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const hashHex = hashArray
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
      const userHash = hashHex.substring(0, 32);

      // Set localStorage BEFORE returning so credentials are available
      // when Redux state change triggers component re-renders and API calls
      localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.USER_ID, userId);
      localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.USER_HASH, userHash);

      return { id: userId, hash: userHash };
    } catch {
      return rejectWithValue("Failed to generate user hash");
    }
  },
);

const authSlice = createSlice({
  name: "auth",
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
    updateSystemKey: (
      state,
      action: PayloadAction<{ systemPublicKey: string; systemKeyId: string }>,
    ) => {
      state.systemPublicKey = action.payload.systemPublicKey;
      state.systemKeyId = action.payload.systemKeyId;
      state.error = null;
    },
    logoutUser: (state) => {
      state.isAuthenticated = false;
      state.user = undefined;
      state.error = null;
    },
    // Restore session from local storage
    restoreSession: (
      state,
      action: PayloadAction<{ id: string; hash: string }>,
    ) => {
      state.isAuthenticated = true;
      state.user = action.payload;
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
        // Don't auto-authenticate user session based on system key
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
        // Don't overwrite isAuthenticated if we have a valid user session
        if (!state.user) {
          state.isAuthenticated = false;
        }
        state.error = null;
      })
      .addCase(refreshSystemKey.rejected, (state, action) => {
        state.isLoading = false;
        state.systemPublicKey = null;
        state.systemKeyId = null;
        state.error = action.payload as string;
      })
      // loginUser cases
      .addCase(loginUser.fulfilled, (state, action) => {
        state.isAuthenticated = true;
        state.user = action.payload;
        state.error = null;
      })
      .addCase(loginUser.rejected, (state, action) => {
        state.isAuthenticated = false;
        state.error = (action.payload as string) || "Login failed";
      });
  },
});

export const {
  clearAuthentication,
  setError,
  clearError,
  updateSystemKey,
  logoutUser,
  restoreSession,
} = authSlice.actions;

export default authSlice.reducer;
