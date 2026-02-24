/**
 * Redux AI Query Slice
 * 
 * Manages all AI query-related state including conversation history,
 * session management, and UI state for persistence across tab navigation.
 */

import { createSlice, PayloadAction } from '@reduxjs/toolkit';

// ============================================================================
// TYPES
// ============================================================================

let _msgCounter = 0;

export interface ConversationMessage {
  id: string;
  type: 'user' | 'system' | 'results';
  content: string;
  data?: unknown;
  timestamp: string;
}

export interface AIQueryState {
  // Input state
  inputText: string;

  // Session management
  sessionId: string | null;

  // Processing state
  isProcessing: boolean;

  // Conversation history
  conversationLog: ConversationMessage[];

  // UI state
  showResults: boolean;
  viewMode: 'list' | 'chat';
}

// ============================================================================
// INITIAL STATE
// ============================================================================

const initialState: AIQueryState = {
  inputText: '',
  sessionId: null,
  isProcessing: false,
  conversationLog: [],
  showResults: false,
  viewMode: 'list',
};

// ============================================================================
// SLICE
// ============================================================================

const aiQuerySlice = createSlice({
  name: 'aiQuery',
  initialState,
  reducers: {
    // Input management
    setInputText: (state, action: PayloadAction<string>) => {
      state.inputText = action.payload;
    },
    
    clearInputText: (state) => {
      state.inputText = '';
    },
    
    // Session management
    setSessionId: (state, action: PayloadAction<string | null>) => {
      state.sessionId = action.payload;
    },
    
    // Processing state
    setIsProcessing: (state, action: PayloadAction<boolean>) => {
      state.isProcessing = action.payload;
    },
    
    // Conversation management
    addMessage: (state, action: PayloadAction<Omit<ConversationMessage, 'id' | 'timestamp'>>) => {
      const message: ConversationMessage = {
        ...action.payload,
        id: `msg-${++_msgCounter}`,
        timestamp: new Date().toISOString(),
      };
      state.conversationLog.push(message);
    },
    
    clearConversation: (state) => {
      state.conversationLog = [];
    },
    
    // UI state
    setShowResults: (state, action: PayloadAction<boolean>) => {
      state.showResults = action.payload;
    },
    
    // View mode
    setViewMode: (state, action: PayloadAction<'list' | 'chat'>) => {
      state.viewMode = action.payload;
    },

    loadConversation: (state, action: PayloadAction<{ sessionId: string; messages: Omit<ConversationMessage, 'id'>[] }>) => {
      state.sessionId = action.payload.sessionId;
      state.conversationLog = action.payload.messages.map(m => ({
        ...m,
        id: m.id || `msg-${++_msgCounter}`,
      })) as ConversationMessage[];
      state.viewMode = 'chat';
      state.inputText = '';
      state.isProcessing = false;
      state.showResults = false;
    },

    // Combined actions
    startNewConversation: (state) => {
      state.sessionId = null;
      state.conversationLog = [];
      state.inputText = '';
      state.isProcessing = false;
      state.showResults = false;
      state.viewMode = 'chat';
    },

    // Reset all state
    resetAIQueryState: () => initialState,
  },
});

// ============================================================================
// EXPORTS
// ============================================================================

export const {
  setInputText,
  clearInputText,
  setSessionId,
  setIsProcessing,
  addMessage,
  clearConversation,
  setShowResults,
  setViewMode,
  loadConversation,
  startNewConversation,
  resetAIQueryState,
} = aiQuerySlice.actions;

export default aiQuerySlice.reducer;

// ============================================================================
// SELECTORS
// ============================================================================

export const selectAIQueryState = (state: { aiQuery: AIQueryState }) => state.aiQuery;

export const selectInputText = (state: { aiQuery: AIQueryState }) => state.aiQuery.inputText;

export const selectSessionId = (state: { aiQuery: AIQueryState }) => state.aiQuery.sessionId;

export const selectIsProcessing = (state: { aiQuery: AIQueryState }) => state.aiQuery.isProcessing;

export const selectConversationLog = (state: { aiQuery: AIQueryState }) => state.aiQuery.conversationLog;

export const selectShowResults = (state: { aiQuery: AIQueryState }) => state.aiQuery.showResults;

export const selectViewMode = (state: { aiQuery: AIQueryState }) => state.aiQuery.viewMode;

export const selectHasResults = (state: { aiQuery: AIQueryState }) => 
  state.aiQuery.conversationLog.some(log => log.type === 'results');

export const selectCanAskFollowup = (state: { aiQuery: AIQueryState }) => 
  state.aiQuery.sessionId && state.aiQuery.conversationLog.some(log => log.type === 'results');
