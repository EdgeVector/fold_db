import { describe, it, expect, vi, beforeEach } from "vitest";
import { configureStore } from "@reduxjs/toolkit";
import ingestionReducer, {
  fetchIngestionConfig,
  saveIngestionConfig,
  selectIngestionConfig,
  selectAiProvider,
  selectActiveModel,
  selectIsAiConfigured,
  selectIsAiReady,
} from "../../store/ingestionSlice";
import type { RootState } from "../../store/store";
import type { IngestionConfig } from "../../api/clients/ingestionClient";
import { ingestionClient } from "../../api/clients";

// Spy on the singleton instance methods so the same object the slice uses is mocked
const mockGetConfig = vi.spyOn(ingestionClient, "getConfig");
const mockSaveConfig = vi.spyOn(ingestionClient, "saveConfig");

function createStore(preloadedIngestion = {}) {
  return configureStore({
    reducer: { ingestion: ingestionReducer },
    preloadedState: { ingestion: { config: null, loading: false, error: null, saving: false, saveError: null, ...preloadedIngestion } },
  });
}

// Helper to build a RootState-shaped object for selector tests
function stateWith(config: IngestionConfig | null): Pick<RootState, "ingestion"> {
  return { ingestion: { config, loading: false, error: null, saving: false, saveError: null } } as Pick<RootState, "ingestion">;
}

const openrouterConfig: IngestionConfig = {
  provider: "OpenRouter",
  openrouter: { api_key: "sk-or-test", model: "google/gemini-2.5-flash", base_url: "https://openrouter.ai/api/v1" },
  ollama: { model: "", base_url: "http://localhost:11434" },
};

const ollamaConfig: IngestionConfig = {
  provider: "Ollama",
  openrouter: { api_key: "", model: "", base_url: "https://openrouter.ai/api/v1" },
  ollama: { model: "llama3.3", base_url: "http://localhost:11434" },
};

const envKeyConfig: IngestionConfig = {
  provider: "OpenRouter",
  openrouter: { api_key: "***configured***", model: "anthropic/claude-sonnet-4.6", base_url: "https://openrouter.ai/api/v1" },
  ollama: { model: "", base_url: "http://localhost:11434" },
};

describe("ingestionSlice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("starts with null config and no loading/error", () => {
      const store = createStore();
      const state = store.getState().ingestion;
      expect(state.config).toBeNull();
      expect(state.loading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.saving).toBe(false);
      expect(state.saveError).toBeNull();
    });
  });

  describe("fetchIngestionConfig", () => {
    it("sets loading while pending then stores config on success", async () => {
      mockGetConfig.mockResolvedValue({ success: true, data: openrouterConfig } as any);
      const store = createStore();

      const promise = store.dispatch(fetchIngestionConfig());
      expect(store.getState().ingestion.loading).toBe(true);

      await promise;
      const state = store.getState().ingestion;
      expect(state.loading).toBe(false);
      expect(state.config).toEqual(openrouterConfig);
      expect(state.error).toBeNull();
    });

    it("sets error on API failure", async () => {
      mockGetConfig.mockResolvedValue({ success: false, data: null } as any);
      const store = createStore();

      await store.dispatch(fetchIngestionConfig());
      const state = store.getState().ingestion;
      expect(state.loading).toBe(false);
      expect(state.config).toBeNull();
      expect(state.error).toBe("Failed to fetch ingestion config");
    });

    it("sets error on network exception", async () => {
      mockGetConfig.mockRejectedValue(new Error("Network error"));
      const store = createStore();

      await store.dispatch(fetchIngestionConfig());
      const state = store.getState().ingestion;
      expect(state.loading).toBe(false);
      expect(state.error).toBe("Network error");
    });
  });

  describe("saveIngestionConfig", () => {
    it("saves then re-fetches config on success", async () => {
      mockSaveConfig.mockResolvedValue({ success: true, data: { success: true, message: "ok" } } as any);
      mockGetConfig.mockResolvedValue({ success: true, data: ollamaConfig } as any);
      const store = createStore();

      const promise = store.dispatch(saveIngestionConfig(ollamaConfig));
      expect(store.getState().ingestion.saving).toBe(true);

      await promise;
      const state = store.getState().ingestion;
      expect(state.saving).toBe(false);
      expect(state.saveError).toBeNull();
      // Config should be populated from the re-fetch
      expect(state.config).toEqual(ollamaConfig);
      expect(mockSaveConfig).toHaveBeenCalledWith(ollamaConfig);
      expect(mockGetConfig).toHaveBeenCalledTimes(1);
    });

    it("sets saveError when API returns failure", async () => {
      mockSaveConfig.mockResolvedValue({ success: false } as any);
      const store = createStore();

      await store.dispatch(saveIngestionConfig(openrouterConfig));
      const state = store.getState().ingestion;
      expect(state.saving).toBe(false);
      expect(state.saveError).toBe("Failed to save ingestion config");
    });

    it("sets saveError on network exception", async () => {
      mockSaveConfig.mockRejectedValue(new Error("Save failed"));
      const store = createStore();

      await store.dispatch(saveIngestionConfig(openrouterConfig));
      const state = store.getState().ingestion;
      expect(state.saving).toBe(false);
      expect(state.saveError).toBe("Save failed");
    });
  });

  describe("selectors", () => {
    describe("selectIngestionConfig", () => {
      it("returns null when no config", () => {
        expect(selectIngestionConfig(stateWith(null) as RootState)).toBeNull();
      });

      it("returns the config object", () => {
        expect(selectIngestionConfig(stateWith(openrouterConfig) as RootState)).toEqual(openrouterConfig);
      });
    });

    describe("selectAiProvider", () => {
      it("returns null when no config", () => {
        expect(selectAiProvider(stateWith(null) as RootState)).toBeNull();
      });

      it("returns OpenRouter", () => {
        expect(selectAiProvider(stateWith(openrouterConfig) as RootState)).toBe("OpenRouter");
      });

      it("returns Ollama", () => {
        expect(selectAiProvider(stateWith(ollamaConfig) as RootState)).toBe("Ollama");
      });
    });

    describe("selectActiveModel", () => {
      it("returns null when no config", () => {
        expect(selectActiveModel(stateWith(null) as RootState)).toBeNull();
      });

      it("returns OpenRouter model when provider is OpenRouter", () => {
        expect(selectActiveModel(stateWith(openrouterConfig) as RootState)).toBe("google/gemini-2.5-flash");
      });

      it("returns Ollama model when provider is Ollama", () => {
        expect(selectActiveModel(stateWith(ollamaConfig) as RootState)).toBe("llama3.3");
      });
    });

    describe("selectIsAiConfigured", () => {
      it("returns false when no config", () => {
        expect(selectIsAiConfigured(stateWith(null) as RootState)).toBe(false);
      });

      it("returns true for OpenRouter with api_key", () => {
        expect(selectIsAiConfigured(stateWith(openrouterConfig) as RootState)).toBe(true);
      });

      it("returns true for OpenRouter with redacted env key", () => {
        expect(selectIsAiConfigured(stateWith(envKeyConfig) as RootState)).toBe(true);
      });

      it("returns false for OpenRouter without api_key", () => {
        const noKey: IngestionConfig = {
          ...openrouterConfig,
          openrouter: { ...openrouterConfig.openrouter, api_key: "" },
        };
        expect(selectIsAiConfigured(stateWith(noKey) as RootState)).toBe(false);
      });

      it("returns true for Ollama with model", () => {
        expect(selectIsAiConfigured(stateWith(ollamaConfig) as RootState)).toBe(true);
      });

      it("returns false for Ollama without model", () => {
        const noModel: IngestionConfig = {
          ...ollamaConfig,
          ollama: { ...ollamaConfig.ollama, model: "" },
        };
        expect(selectIsAiConfigured(stateWith(noModel) as RootState)).toBe(false);
      });
    });

    describe("selectIsAiReady", () => {
      it("returns false when no config", () => {
        expect(selectIsAiReady(stateWith(null) as RootState)).toBe(false);
      });

      it("returns true when config exists and is configured", () => {
        expect(selectIsAiReady(stateWith(openrouterConfig) as RootState)).toBe(true);
      });

      it("returns false when config exists but not configured", () => {
        const unconfigured: IngestionConfig = {
          ...openrouterConfig,
          openrouter: { ...openrouterConfig.openrouter, api_key: "" },
        };
        expect(selectIsAiReady(stateWith(unconfigured) as RootState)).toBe(false);
      });
    });
  });
});
