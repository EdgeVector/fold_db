/**
 * LlmQueryTab Component - Conversational AI Query Interface
 *
 * A simplified chat-style interface where the AI automatically loops through
 * queries until it finds data or determines it doesn't exist.
 * Uses minimal design system consistent with the rest of FoldDB.
 */

import { useCallback, useRef, useEffect } from 'react';
import { llmQueryClient } from '../../api/clients/llmQueryClient';
import { useAppSelector, useAppDispatch } from '../../store/hooks';
import {
  setInputText,
  setSessionId,
  setIsProcessing,
  addMessage,
  setShowResults,
  startNewConversation,
  selectInputText,
  selectSessionId,
  selectIsProcessing,
  selectConversationLog,
  selectShowResults,
  selectCanAskFollowup,
} from '../../store/aiQuerySlice';

function LlmQueryTab({ onResult }) {
  // Redux state and dispatch
  const dispatch = useAppDispatch();
  const inputText = useAppSelector(selectInputText);
  const sessionId = useAppSelector(selectSessionId);
  const isProcessing = useAppSelector(selectIsProcessing);
  const conversationLog = useAppSelector(selectConversationLog);
  const showResults = useAppSelector(selectShowResults);
  const canAskFollowup = useAppSelector(selectCanAskFollowup);

  const conversationEndRef = useRef(null);

  // Auto-scroll to bottom when conversation updates
  useEffect(() => {
    conversationEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [conversationLog]);

  /**
   * Add a message to the conversation log
   */
  const addToLog = useCallback((type, content, data = null) => {
    dispatch(addMessage({ type, content, data }));
  }, [dispatch]);

  /**
   * Process an AI agent response — shared by follow-up and new-query paths
   */
  const processAgentResponse = useCallback((agentResponse) => {
    if (!agentResponse.success) {
      addToLog('system', `❌ Error: ${agentResponse.error || 'Failed to run AI agent query'}`);
      return;
    }

    const result = agentResponse.data;

    // Update session ID if returned from the server
    if (result.session_id) {
      dispatch(setSessionId(result.session_id));
    }

    // Show tool calls if any were made
    if (result.tool_calls && result.tool_calls.length > 0) {
      addToLog('system', `🔧 Made ${result.tool_calls.length} tool call(s)`);
      addToLog('results', 'Tool execution trace', result.tool_calls);
    }

    // Display the AI's final answer
    addToLog('system', result.answer);

    // Show results section if user has expanded details
    if (showResults && result.tool_calls) {
      onResult({ success: true, data: result.tool_calls });
    }
  }, [addToLog, dispatch, showResults, onResult]);

  /**
   * Handle user input - run query or ask follow-up
   */
  const handleSubmit = useCallback(async (e) => {
    e?.preventDefault();

    if (!inputText.trim() || isProcessing) {
      return;
    }

    const userInput = inputText.trim();
    dispatch(setInputText(''));
    dispatch(setIsProcessing(true));

    // Add user message to log
    addToLog('user', userInput);

    try {
      // If this is a follow-up question (session exists and we have results)
      if (canAskFollowup) {
        addToLog('system', '🤔 Analyzing if question can be answered from existing context...');

        // First analyze if the question can be answered from existing context
        const analysisResponse = await llmQueryClient.analyzeFollowup({
          session_id: sessionId,
          question: userInput
        });

        if (!analysisResponse.success) {
          addToLog('system', `❌ Error: ${analysisResponse.error || 'Failed to analyze question'}`);
          return;
        }

        const analysis = analysisResponse.data;

        if (!analysis.needs_query) {
          // Can answer from existing context
          addToLog('system', `✅ Answering from existing context: ${analysis.reasoning}`);

          const chatResponse = await llmQueryClient.chat({
            session_id: sessionId,
            question: userInput
          });

          if (!chatResponse.success) {
            addToLog('system', `❌ Error: ${chatResponse.error || 'Failed to process question'}`);
            return;
          }

          addToLog('system', chatResponse.data.answer);
        } else {
          // Needs new query - use AI agent with tool calling
          addToLog('system', `🔍 Need new data: ${analysis.reasoning}`);
          addToLog('system', '🤖 Starting AI agent...');

          const agentResponse = await llmQueryClient.agentQuery({
            query: userInput,
            session_id: sessionId,
            max_iterations: 10
          });

          processAgentResponse(agentResponse);
        }
      } else {
        // New query - use AI agent with tool calling
        addToLog('system', '🤖 Starting AI agent...');

        const agentResponse = await llmQueryClient.agentQuery({
          query: userInput,
          session_id: sessionId,
          max_iterations: 10
        });

        processAgentResponse(agentResponse);
      }
    } catch (error) {
      console.error('Error processing input:', error);
      addToLog('system', `❌ Error: ${error.message}`);
      onResult({ error: error.message });
    } finally {
      dispatch(setIsProcessing(false));
    }
  }, [inputText, sessionId, canAskFollowup, isProcessing, processAgentResponse, addToLog, onResult, dispatch]);

  /**
   * Start a new conversation
   */
  const handleNewConversation = useCallback(() => {
    dispatch(startNewConversation());
  }, [dispatch]);

  return (
    <div className="flex flex-col h-[600px]">
      {/* Conversation Log */}
      <div className="flex-1 overflow-y-auto p-6 space-y-3">
        {conversationLog.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-secondary">
            <div className="text-4xl mb-4">→</div>
            <p className="text-base mb-2">Start a conversation</p>
            <p className="text-sm text-tertiary">
              Try: "What schemas are available?" or "Find tweets mentioning rust" or "Search for blog posts about AI"
            </p>
          </div>
        ) : (
          conversationLog.map((entry, idx) => (
            <div key={idx} className="mb-3">
              {entry.type === 'user' && (
                <div className="flex justify-end">
                  <div className="max-w-[80%] px-4 py-3 bg-gruvbox-elevated border border-gruvbox-orange text-primary rounded-lg">
                    <p className="text-xs opacity-70 mb-1">You</p>
                    <p className="text-sm">{entry.content}</p>
                  </div>
                </div>
              )}

              {entry.type === 'system' && (
                <div className="flex justify-start">
                  <div className="max-w-[80%] px-4 py-3 bg-surface-secondary border border-border rounded-lg">
                    <p className="text-xs text-tertiary mb-1">AI Assistant</p>
                    <p className="text-sm text-primary whitespace-pre-wrap">{entry.content}</p>
                  </div>
                </div>
              )}

              {entry.type === 'results' && entry.data && (
                <div className="border border-border bg-surface p-4 rounded-lg">
                  <div className="flex justify-between items-center mb-2">
                    <p className="text-sm font-medium text-primary">
                      Results ({entry.data.length})
                    </p>
                    <button
                      onClick={() => {
                        const newShowResults = !showResults;
                        dispatch(setShowResults(newShowResults));
                        if (newShowResults) {
                          const resultsEntry = conversationLog.find(log => log.type === 'results');
                          if (resultsEntry && resultsEntry.data) {
                            onResult({ success: true, data: resultsEntry.data });
                          }
                        } else {
                          onResult(null);
                        }
                      }}
                      className="text-xs text-gruvbox-blue hover:underline bg-transparent border-none cursor-pointer"
                    >
                      {showResults ? 'Hide Details' : 'Show Details'}
                    </button>
                  </div>
                  {showResults && (
                    <>
                      <div className="bg-surface-secondary p-3 mb-2">
                        <p className="text-primary whitespace-pre-wrap mb-3 text-sm">{entry.content}</p>
                      </div>
                      <details className="mt-2">
                        <summary className="cursor-pointer text-sm text-secondary">
                          View raw data ({entry.data.length} records)
                        </summary>
                        <div className="mt-2 max-h-64 overflow-y-auto">
                          <pre className="text-xs font-mono bg-surface-secondary p-3 border border-border overflow-x-auto">
                            {JSON.stringify(entry.data, null, 2)}
                          </pre>
                        </div>
                      </details>
                    </>
                  )}
                </div>
              )}
            </div>
          ))
        )}
        <div ref={conversationEndRef} />
      </div>

      {/* Input Box */}
      <form onSubmit={handleSubmit} className="px-6 py-4 border-t border-border bg-surface">
        <div className="flex gap-2">
          <input
            type="text"
            value={inputText}
            onChange={(e) => dispatch(setInputText(e.target.value))}
            placeholder={
              conversationLog.some(log => log.type === 'results')
                ? "Ask a follow-up question or start a new query..."
                : "Ask anything (e.g., 'Find tweets about rust', 'What schemas exist?')..."
            }
            disabled={isProcessing}
            className="input flex-1"
            autoFocus
          />
          <button type="submit" disabled={!inputText.trim() || isProcessing} className="btn-primary btn-lg">
            {isProcessing ? 'Processing…' : 'Send'}
          </button>
        </div>
        {isProcessing && (
          <p className="text-center text-sm text-tertiary mt-2">
            AI is analyzing and searching…
          </p>
        )}
      </form>
    </div>
  );
}

export default LlmQueryTab;
