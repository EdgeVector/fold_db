import { createSlice, createAsyncThunk } from "@reduxjs/toolkit";
import { ingestionClient } from "../api/clients";
import type { IngestionConfig } from "../api/clients/ingestionClient";
import type { RootState } from "./store";

interface IngestionState {
  config: IngestionConfig | null;
  loading: boolean;
  error: string | null;
  saving: boolean;
  saveError: string | null;
}

const initialState: IngestionState = {
  config: null,
  loading: false,
  error: null,
  saving: false,
  saveError: null,
};

export const fetchIngestionConfig = createAsyncThunk(
  "ingestion/fetchConfig",
  async (_, { rejectWithValue }) => {
    try {
      const response = await ingestionClient.getConfig();
      if (response.success && response.data) {
        return response.data;
      }
      return rejectWithValue("Failed to fetch ingestion config");
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : String(error),
      );
    }
  },
);

export const saveIngestionConfig = createAsyncThunk(
  "ingestion/saveConfig",
  async (config: IngestionConfig, { dispatch, rejectWithValue }) => {
    try {
      const response = await ingestionClient.saveConfig(config);
      if (response.success) {
        // Re-fetch to get the canonical config from the server
        await dispatch(fetchIngestionConfig()).unwrap();
        return true;
      }
      return rejectWithValue("Failed to save ingestion config");
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : String(error),
      );
    }
  },
);

const ingestionSlice = createSlice({
  name: "ingestion",
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    builder
      .addCase(fetchIngestionConfig.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchIngestionConfig.fulfilled, (state, action) => {
        state.config = action.payload;
        state.loading = false;
        state.error = null;
      })
      .addCase(fetchIngestionConfig.rejected, (state, action) => {
        state.loading = false;
        state.error = (action.payload as string) ?? "Unknown error";
      })
      .addCase(saveIngestionConfig.pending, (state) => {
        state.saving = true;
        state.saveError = null;
      })
      .addCase(saveIngestionConfig.fulfilled, (state) => {
        state.saving = false;
        state.saveError = null;
      })
      .addCase(saveIngestionConfig.rejected, (state, action) => {
        state.saving = false;
        state.saveError = (action.payload as string) ?? "Unknown error";
      });
  },
});

// Selectors
export const selectIngestionConfig = (state: RootState) =>
  state.ingestion.config;

export const selectAiProvider = (state: RootState) =>
  state.ingestion.config?.provider ?? null;

export const selectActiveModel = (state: RootState) => {
  const config = state.ingestion.config;
  if (!config) return null;
  return config.provider === "OpenRouter"
    ? config.openrouter?.model
    : config.ollama?.model;
};

export const selectIsAiConfigured = (state: RootState) => {
  const config = state.ingestion.config;
  if (!config) return false;
  if (config.provider === "OpenRouter") {
    return !!config.openrouter?.api_key;
  }
  return !!config.ollama?.model;
};

export const selectIsAiReady = (state: RootState) =>
  state.ingestion.config !== null && selectIsAiConfigured(state);

export default ingestionSlice.reducer;
