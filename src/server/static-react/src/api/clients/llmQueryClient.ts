/**
 * LLM Query API Client
 * Provides natural language query capabilities with LLM analysis
 */

import { createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES } from '../../constants/api';

// LLM client needs custom timeout for AI processing
const client = createApiClient({
  timeout: API_TIMEOUTS.AI_PROCESSING,
  retries: API_RETRIES.LIMITED
});

export interface ChatRequest {
  session_id: string;
  question: string;
}

export interface ChatResponse {
  answer: string;
  context_used: boolean;
}

export interface BackfillStatusResponse {
  status: string;
  progress: number;
  total_records: number;
  processed_records: number;
  estimated_completion?: string;
}

export interface FollowupAnalysis {
  needs_query: boolean;
  query?: Record<string, unknown>;
  reasoning: string;
}

export interface AgentQueryRequest {
  query: string;
  session_id?: string;
  max_iterations?: number;
}

export interface ToolCallRecord {
  tool: string;
  params: Record<string, unknown>;
  result: unknown;
}

export interface AgentQueryResponse {
  answer: string;
  tool_calls: ToolCallRecord[];
  session_id: string;
}

export const llmQueryClient = {
  /**
   * Ask a follow-up question about results
   */
  async chat(request: ChatRequest) {
    return client.post<ChatResponse>(API_ENDPOINTS.CHAT, request);
  },

  /**
   * Analyze if a follow-up question can be answered from existing context
   */
  async analyzeFollowup(request: ChatRequest) {
    return client.post<FollowupAnalysis>(API_ENDPOINTS.ANALYZE_FOLLOWUP, request);
  },

  /**
   * Get backfill status by hash
   */
  async getBackfillStatus(hash: string) {
    return client.get<BackfillStatusResponse>(API_ENDPOINTS.GET_BACKFILL_STATUS(hash));
  },

  /**
   * Run an autonomous agent query with tool calling
   */
  async agentQuery(request: AgentQueryRequest) {
    return client.post<AgentQueryResponse>(API_ENDPOINTS.AGENT_QUERY, request);
  }
};

