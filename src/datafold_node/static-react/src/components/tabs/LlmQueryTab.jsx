/**
 * LlmQueryTab Component - Natural Language Query Interface
 * 
 * Provides an interactive natural language query interface that uses LLM
 * to analyze queries, create indexes if needed, and provide interactive
 * results exploration.
 */

import { useState, useCallback, useEffect } from 'react';
import { llmQueryClient } from '../../api/clients/llmQueryClient';

function LlmQueryTab({ onResult }) {
  // State management
  const [query, setQuery] = useState('');
  const [sessionId, setSessionId] = useState(null);
  const [queryPlan, setQueryPlan] = useState(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [executionStatus, setExecutionStatus] = useState(null);
  const [results, setResults] = useState(null);
  const [summary, setSummary] = useState(null);
  const [followUpQuestion, setFollowUpQuestion] = useState('');
  const [chatHistory, setChatHistory] = useState([]);
  const [isAsking, setIsAsking] = useState(false);
  const [backfillProgress, setBackfillProgress] = useState(null);
  const [pollingInterval, setPollingInterval] = useState(null);

  // Clear polling interval on unmount
  useEffect(() => {
    return () => {
      if (pollingInterval) {
        clearInterval(pollingInterval);
      }
    };
  }, [pollingInterval]);

  /**
   * Analyze the natural language query
   */
  const handleAnalyze = useCallback(async () => {
    if (!query.trim()) {
      onResult({ error: 'Please enter a query' });
      return;
    }

    setIsAnalyzing(true);
    setQueryPlan(null);
    setExecutionStatus(null);
    setResults(null);
    setSummary(null);
    
    try {
      const response = await llmQueryClient.analyzeQuery({
        query: query.trim(),
        session_id: sessionId
      });

      if (!response.success) {
        onResult({ error: response.error || 'Failed to analyze query' });
        return;
      }

      setSessionId(response.data.session_id);
      setQueryPlan(response.data.query_plan);
      onResult({
        success: true,
        message: 'Query analyzed successfully'
      });
    } catch (error) {
      console.error('Failed to analyze query:', error);
      onResult({
        error: `Analysis failed: ${error.message}`
      });
    } finally {
      setIsAnalyzing(false);
    }
  }, [query, sessionId, onResult]);

  /**
   * Execute the query plan
   */
  const handleExecute = useCallback(async () => {
    if (!queryPlan || !sessionId) {
      onResult({ error: 'No query plan to execute' });
      return;
    }

    setIsExecuting(true);
    setExecutionStatus('pending');
    
    try {
      const response = await llmQueryClient.executeQueryPlan({
        session_id: sessionId,
        query_plan: queryPlan
      });

      if (!response.success) {
        onResult({ error: response.error || 'Failed to execute query' });
        setIsExecuting(false);
        return;
      }

      const data = response.data;
      setExecutionStatus(data.status);

      if (data.status === 'complete') {
        // Query is complete
        setResults(data.results);
        setSummary(data.summary);
        setBackfillProgress(null);
        onResult({
          success: true,
          data: data.results
        });
        setIsExecuting(false);
      } else if (data.status === 'running' || data.status === 'pending') {
        // Backfill in progress, start polling
        setBackfillProgress(data.backfill_progress || 0);
        
        // Poll every 2 seconds
        const interval = setInterval(async () => {
          try {
            const pollResponse = await llmQueryClient.executeQueryPlan({
              session_id: sessionId,
              query_plan: queryPlan
            });

            if (pollResponse.success) {
              const pollData = pollResponse.data;
              setBackfillProgress(pollData.backfill_progress || 0);
              
              if (pollData.status === 'complete') {
                setResults(pollData.results);
                setSummary(pollData.summary);
                setExecutionStatus('complete');
                setBackfillProgress(null);
                setIsExecuting(false);
                onResult({
                  success: true,
                  data: pollData.results
                });
                clearInterval(interval);
              }
            }
          } catch (error) {
            console.error('Polling error:', error);
          }
        }, 2000);

        setPollingInterval(interval);
      }
    } catch (error) {
      console.error('Failed to execute query:', error);
      onResult({
        error: `Execution failed: ${error.message}`
      });
      setIsExecuting(false);
    }
  }, [queryPlan, sessionId, onResult]);

  /**
   * Ask a follow-up question
   */
  const handleAskFollowUp = useCallback(async () => {
    if (!followUpQuestion.trim() || !sessionId) {
      return;
    }

    setIsAsking(true);
    
    try {
      const response = await llmQueryClient.chat({
        session_id: sessionId,
        question: followUpQuestion.trim()
      });

      if (!response.success) {
        onResult({ error: response.error || 'Failed to get answer' });
        return;
      }

      // Add to chat history
      setChatHistory(prev => [
        ...prev,
        { role: 'user', content: followUpQuestion.trim() },
        { role: 'assistant', content: response.data.answer }
      ]);
      
      setFollowUpQuestion('');
    } catch (error) {
      console.error('Failed to ask follow-up question:', error);
      onResult({
        error: `Failed to get answer: ${error.message}`
      });
    } finally {
      setIsAsking(false);
    }
  }, [followUpQuestion, sessionId, onResult]);

  /**
   * Start a new query session
   */
  const handleNewQuery = useCallback(() => {
    if (pollingInterval) {
      clearInterval(pollingInterval);
      setPollingInterval(null);
    }
    
    setQuery('');
    setSessionId(null);
    setQueryPlan(null);
    setExecutionStatus(null);
    setResults(null);
    setSummary(null);
    setChatHistory([]);
    setFollowUpQuestion('');
    setBackfillProgress(null);
    setIsExecuting(false);
  }, [pollingInterval]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="bg-white p-6 rounded-lg shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          🤖 Natural Language Query
        </h2>
        <p className="text-gray-600">
          Ask questions in plain English. The AI will analyze your query, create indexes if needed, and provide interactive results exploration.
        </p>
      </div>

      {/* Query Input Section */}
      <div className="bg-white p-6 rounded-lg shadow">
        <label className="block text-sm font-medium text-gray-700 mb-2">
          Your Question
        </label>
        <textarea
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Example: Find all blog posts about AI from last month"
          className="w-full h-32 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
          disabled={isAnalyzing || isExecuting}
        />
        
        <div className="mt-4 flex gap-3">
          <button
            onClick={handleAnalyze}
            disabled={isAnalyzing || isExecuting || !query.trim()}
            className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
          >
            {isAnalyzing ? 'Analyzing...' : 'Analyze Query'}
          </button>
          
          {queryPlan && !isExecuting && (
            <button
              onClick={handleNewQuery}
              className="px-6 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors"
            >
              New Query
            </button>
          )}
        </div>
      </div>

      {/* Query Plan Section */}
      {queryPlan && (
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">📋 Query Plan</h3>
          
          <div className="space-y-4">
            {/* Reasoning */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Analysis
              </label>
              <p className="text-gray-600 bg-gray-50 p-3 rounded">
                {queryPlan.reasoning}
              </p>
            </div>

            {/* Target Schema */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Target Schema
              </label>
              <p className="text-gray-900 font-mono bg-gray-50 p-3 rounded">
                {queryPlan.query.schema_name}
              </p>
            </div>

            {/* Fields */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Fields to Retrieve
              </label>
              <div className="flex flex-wrap gap-2">
                {queryPlan.query.fields.map((field, idx) => (
                  <span
                    key={idx}
                    className="px-3 py-1 bg-blue-100 text-blue-700 rounded-full text-sm font-mono"
                  >
                    {field}
                  </span>
                ))}
              </div>
            </div>

            {/* Index Schema (if needed) */}
            {queryPlan.index_schema && (
              <div className="border-l-4 border-yellow-400 bg-yellow-50 p-4 rounded">
                <h4 className="text-sm font-semibold text-yellow-800 mb-2">
                  ⚠️ Index Creation Required
                </h4>
                <p className="text-sm text-yellow-700 mb-2">
                  An index schema will be created to optimize this query:
                </p>
                <p className="font-mono text-sm text-yellow-900">
                  {queryPlan.index_schema.name}
                </p>
              </div>
            )}

            {/* Execute Button */}
            <button
              onClick={handleExecute}
              disabled={isExecuting}
              className="w-full px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold"
            >
              {isExecuting ? 'Executing...' : '▶️ Execute Query'}
            </button>
          </div>
        </div>
      )}

      {/* Backfill Progress */}
      {isExecuting && backfillProgress !== null && (
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            ⏳ Building Index...
          </h3>
          <div className="space-y-2">
            <div className="w-full bg-gray-200 rounded-full h-4">
              <div
                className="bg-blue-600 h-4 rounded-full transition-all duration-500"
                style={{ width: `${(backfillProgress * 100).toFixed(1)}%` }}
              />
            </div>
            <p className="text-sm text-gray-600 text-center">
              {(backfillProgress * 100).toFixed(1)}% complete
            </p>
          </div>
        </div>
      )}

      {/* Results Summary */}
      {summary && (
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">📊 Summary</h3>
          <div className="prose max-w-none">
            <p className="text-gray-700 whitespace-pre-wrap">{summary}</p>
          </div>
        </div>
      )}

      {/* Follow-up Questions */}
      {results && sessionId && (
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            💬 Ask Follow-up Questions
          </h3>
          
          {/* Chat History */}
          {chatHistory.length > 0 && (
            <div className="mb-4 space-y-3 max-h-96 overflow-y-auto">
              {chatHistory.map((msg, idx) => (
                <div
                  key={idx}
                  className={`p-3 rounded-lg ${
                    msg.role === 'user'
                      ? 'bg-blue-50 ml-8'
                      : 'bg-gray-50 mr-8'
                  }`}
                >
                  <p className="text-sm font-semibold text-gray-700 mb-1">
                    {msg.role === 'user' ? '👤 You' : '🤖 AI'}
                  </p>
                  <p className="text-gray-900 whitespace-pre-wrap">{msg.content}</p>
                </div>
              ))}
            </div>
          )}

          {/* Question Input */}
          <div className="flex gap-2">
            <input
              type="text"
              value={followUpQuestion}
              onChange={(e) => setFollowUpQuestion(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && !isAsking && handleAskFollowUp()}
              placeholder="Ask a question about these results..."
              className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              disabled={isAsking}
            />
            <button
              onClick={handleAskFollowUp}
              disabled={isAsking || !followUpQuestion.trim()}
              className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
            >
              {isAsking ? '...' : 'Ask'}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default LlmQueryTab;

