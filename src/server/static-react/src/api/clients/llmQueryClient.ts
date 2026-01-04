/**
 * LLM Query API Client
 * Provides natural language query capabilities with LLM analysis
 */

import { createApiClient } from '../core/client';
import { API_TIMEOUTS, API_RETRIES } from '../../constants/api';

const client = createApiClient({
  timeout: API_TIMEOUTS.AI_PROCESSING,
  retries: API_RETRIES.LIMITED
});

export interface AnalyzeQueryRequest {
  query: string;
  session_id?: string;
}

export interface QueryPlan {
  query: {
    schema_name: string;
    fields: string[];
    filter?: any;
  };
  index_schema?: any;
  reasoning: string;
}

export interface AnalyzeQueryResponse {
  session_id: string;
  query_plan: QueryPlan;
}

export interface ExecuteQueryPlanRequest {
  session_id: string;
  query_plan: QueryPlan;
}

export interface ExecuteQueryPlanResponse {
  status: 'pending' | 'running' | 'complete';
  backfill_progress?: number;
  results?: any[];
  summary?: string;
}

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

export interface RunQueryRequest {
  query: string;
  session_id?: string;
}

export interface RunQueryResponse {
  session_id: string;
  query_plan: QueryPlan;
  results: any[];
  summary?: string;
}

export interface FollowupAnalysis {
  needs_query: boolean;
  query?: QueryPlan;
  reasoning: string;
}

export const llmQueryClient = {
  /**
   * Run a query in a single step (analyze + execute with internal polling loop)
   */
  async runQuery(request: RunQueryRequest) {
    return client.post<RunQueryResponse>('/llm-query/run', request);
  },

  /**
   * Analyze a natural language query
   */
  async analyzeQuery(request: AnalyzeQueryRequest) {
    return client.post<AnalyzeQueryResponse>('/llm-query/analyze', request);
  },

  /**
   * Execute a query plan
   */
  async executeQueryPlan(request: ExecuteQueryPlanRequest) {
    return client.post<ExecuteQueryPlanResponse>('/llm-query/execute', request);
  },

  /**
   * Ask a follow-up question about results
   */
  async chat(request: ChatRequest) {
    return client.post<ChatResponse>('/llm-query/chat', request);
  },

  /**
   * Analyze if a follow-up question can be answered from existing context
   */
  async analyzeFollowup(request: ChatRequest) {
    return client.post<FollowupAnalysis>('/llm-query/analyze-followup', request);
  },

  /**
   * Get backfill status by hash
   */
  async getBackfillStatus(hash: string) {
    return client.get<BackfillStatusResponse>(`/llm-query/backfill/${hash}`);
  }
};

