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

          if (!agentResponse.ok) {
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
            // Add tool calls as results for detailed view
            addToLog('results', 'Tool execution trace', result.tool_calls);
          }

          // Display the AI's final answer
          addToLog('system', result.answer);

          // Show results section if user has expanded details
          if (showResults && result.tool_calls) {
            onResult({ success: true, data: result.tool_calls });
          }
        }
      } else {
        // New query - use AI agent with tool calling
        addToLog('system', '🤖 Starting AI agent...');

        const agentResponse = await llmQueryClient.agentQuery({
          query: userInput,
          session_id: sessionId,
          max_iterations: 10
        });

        if (!agentResponse.ok) {
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
          // Add tool calls as results for detailed view
          addToLog('results', 'Tool execution trace', result.tool_calls);
        }

        // Display the AI's final answer
        addToLog('system', result.answer);

        // Show results section if user has expanded details
        if (showResults && result.tool_calls) {
          onResult({ success: true, data: result.tool_calls });
        }
      }
    } catch (error) {
      console.error('Error processing input:', error);
      addToLog('system', `❌ Error: ${error.message}`);
      onResult({ error: error.message });
    } finally {
      dispatch(setIsProcessing(false));
    }
  }, [inputText, sessionId, canAskFollowup, isProcessing, showResults, addToLog, onResult, dispatch]);

  /**
   * Start a new conversation
   */
  const handleNewConversation = useCallback(() => {
    dispatch(startNewConversation());
  }, [dispatch]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div style={{
        padding: '12px 16px',
        borderBottom: '1px solid #e5e5e5',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <div>
          <h2 style={{ fontSize: '16px', fontWeight: 500, color: '#111', margin: 0 }}>
            AI Query
          </h2>
          <p style={{ color: '#999', fontSize: '13px', margin: '4px 0 0' }}>
            Ask questions in plain English — the AI will find your data
          </p>
        </div>
        {conversationLog.length > 0 && (
          <button
            onClick={handleNewConversation}
            disabled={isProcessing}
            style={{
              padding: '8px 16px',
              background: isProcessing ? '#e5e5e5' : '#fff',
              color: isProcessing ? '#999' : '#111',
              border: '1px solid #e5e5e5',
              fontSize: '13px',
              cursor: isProcessing ? 'not-allowed' : 'pointer',
              transition: 'border-color 0.2s'
            }}
            onMouseOver={(e) => !isProcessing && (e.target.style.borderColor = '#111')}
            onMouseOut={(e) => !isProcessing && (e.target.style.borderColor = '#e5e5e5')}
          >
            New Conversation
          </button>
        )}
      </div>

      {/* Conversation Log */}
      <div style={{
        overflowY: 'auto',
        background: '#fafafa',
        padding: '16px',
        maxHeight: '60vh',
        minHeight: '300px'
      }}>
        {conversationLog.length === 0 ? (
          <div style={{ textAlign: 'center', color: '#999', marginTop: '60px' }}>
            <div style={{ fontSize: '48px', marginBottom: '16px', opacity: 0.5 }}>→</div>
            <p style={{ fontSize: '15px', marginBottom: '8px' }}>Start a conversation</p>
            <p style={{ fontSize: '13px', color: '#bbb' }}>
              Try: "What schemas are available?" or "Find tweets mentioning rust" or "Search for blog posts about AI"
            </p>
          </div>
        ) : (
          conversationLog.map((entry, idx) => (
            <div key={idx} style={{ marginBottom: '12px' }}>
              {entry.type === 'user' && (
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <div style={{
                    background: '#111',
                    color: '#fff',
                    padding: '10px 16px',
                    maxWidth: '600px'
                  }}>
                    <p style={{ fontSize: '11px', fontWeight: 500, marginBottom: '4px', opacity: 0.7 }}>You</p>
                    <p style={{ whiteSpace: 'pre-wrap', margin: 0, fontSize: '14px' }}>{entry.content}</p>
                  </div>
                </div>
              )}
              
              {entry.type === 'system' && (
                <div style={{ display: 'flex', justifyContent: 'flex-start' }}>
                  <div style={{
                    background: '#fff',
                    border: '1px solid #e5e5e5',
                    padding: '10px 16px',
                    maxWidth: '600px'
                  }}>
                    <p style={{ fontSize: '11px', fontWeight: 500, color: '#999', marginBottom: '4px' }}>AI Assistant</p>
                    <p style={{ color: '#111', whiteSpace: 'pre-wrap', margin: 0, fontSize: '14px' }}>{entry.content}</p>
                  </div>
                </div>
              )}
              
              {entry.type === 'results' && entry.data && (
                <div style={{
                  background: '#fff',
                  border: '1px solid #e5e5e5',
                  borderLeftWidth: '3px',
                  borderLeftColor: '#22c55e',
                  padding: '16px',
                  maxWidth: '100%'
                }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                    <p style={{ fontSize: '13px', fontWeight: 500, color: '#111', margin: 0 }}>
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
                      style={{
                        fontSize: '13px',
                        color: '#666',
                        background: 'transparent',
                        border: 'none',
                        cursor: 'pointer',
                        textDecoration: 'underline'
                      }}
                    >
                      {showResults ? 'Hide Details' : 'Show Details'}
                    </button>
                  </div>
                  {showResults && (
                    <>
                      <div style={{ background: '#fafafa', padding: '12px', marginBottom: '8px' }}>
                        <p style={{ color: '#111', whiteSpace: 'pre-wrap', marginBottom: '12px', fontSize: '14px' }}>{entry.content}</p>
                      </div>
                      <details style={{ marginTop: '8px' }}>
                        <summary style={{
                          cursor: 'pointer',
                          fontSize: '13px',
                          color: '#666'
                        }}>
                          View raw data ({entry.data.length} records)
                        </summary>
                        <div style={{ marginTop: '8px', maxHeight: '256px', overflowY: 'auto' }}>
                          <pre style={{
                            fontSize: '12px',
                            background: '#111',
                            color: '#22c55e',
                            padding: '12px',
                            fontFamily: "'SF Mono', Monaco, monospace",
                            whiteSpace: 'pre-wrap',
                            margin: 0
                          }}>
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
      <form onSubmit={handleSubmit} style={{
        borderTop: '1px solid #e5e5e5',
        padding: '12px 16px',
        background: '#fff'
      }}>
        <div style={{ display: 'flex', gap: '8px' }}>
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
            style={{
              flex: 1,
              padding: '12px 16px',
              border: '1px solid #e5e5e5',
              background: isProcessing ? '#fafafa' : '#fff',
              fontSize: '14px',
              color: '#111',
              outline: 'none',
              transition: 'border-color 0.2s'
            }}
            onFocus={(e) => e.target.style.borderColor = '#111'}
            onBlur={(e) => e.target.style.borderColor = '#e5e5e5'}
            autoFocus
          />
          <button
            type="submit"
            disabled={!inputText.trim() || isProcessing}
            style={{
              padding: '12px 24px',
              background: (!inputText.trim() || isProcessing) ? '#e5e5e5' : '#111',
              color: (!inputText.trim() || isProcessing) ? '#999' : '#fff',
              border: 'none',
              fontSize: '14px',
              fontWeight: 500,
              cursor: (!inputText.trim() || isProcessing) ? 'not-allowed' : 'pointer',
              transition: 'background 0.2s'
            }}
            onMouseOver={(e) => {
              if (inputText.trim() && !isProcessing) e.target.style.background = '#333';
            }}
            onMouseOut={(e) => {
              if (inputText.trim() && !isProcessing) e.target.style.background = '#111';
            }}
          >
            {isProcessing ? 'Processing…' : 'Send'}
          </button>
        </div>
        {isProcessing && (
          <p style={{ textAlign: 'center', fontSize: '13px', color: '#999', marginTop: '8px' }}>
            AI is analyzing and searching…
          </p>
        )}
      </form>
    </div>
  );
}

export default LlmQueryTab;
