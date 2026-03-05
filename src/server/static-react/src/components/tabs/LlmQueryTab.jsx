/**
 * LlmQueryTab Component - Conversational AI Query Interface
 *
 * A simplified chat-style interface where the AI automatically loops through
 * queries until it finds data or determines it doesn't exist.
 * Uses minimal design system consistent with the rest of FoldDB.
 */

import { useCallback, useRef, useEffect, useState } from 'react';
import { llmQueryClient } from '../../api/clients/llmQueryClient';
import { mutationClient } from '../../api/clients/mutationClient';
import { createHashKeyFilter } from '../../utils/filterUtils';
import { extractImagesFromToolCalls } from '../../utils/imageUtils';
import { STARTER_SUGGESTIONS } from '../../constants/suggestions';
import ImageThumbnail from './llm-query/ImageThumbnail';
import { useAppSelector, useAppDispatch } from '../../store/hooks';
import ConversationList from './ConversationList';
import {
  setInputText,
  setSessionId,
  setIsProcessing,
  addMessage,
  setShowResults,
  setViewMode,
  loadConversation,
  startNewConversation,
  selectInputText,
  selectSessionId,
  selectIsProcessing,
  selectConversationLog,
  selectShowResults,
  selectCanAskFollowup,
  selectViewMode,
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

  const viewMode = useAppSelector(selectViewMode);
  const [isLoadingConversation, setIsLoadingConversation] = useState(false);

  const conversationEndRef = useRef(null);

  // Auto-scroll to bottom when conversation updates
  useEffect(() => {
    conversationEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [conversationLog]);

  const addToLog = useCallback((type, content, data = null) => {
    dispatch(addMessage({ type, content, data }));
  }, [dispatch]);

  /** Process an AI agent response — shared by follow-up and new-query paths */
  const processAgentResponse = useCallback((agentResponse) => {
    if (!agentResponse.success) {
      addToLog('system', `❌ Error: ${agentResponse.error || 'Failed to run AI agent query'}`);
      return;
    }

    const result = agentResponse.data;

    if (result.session_id) {
      dispatch(setSessionId(result.session_id));
    }

    if (result.tool_calls && result.tool_calls.length > 0) {
      addToLog('system', `🔧 Made ${result.tool_calls.length} tool call(s)`);
      addToLog('results', 'Tool execution trace', result.tool_calls);
    }

    addToLog('system', result.answer);

    if (result.tool_calls) {
      const images = extractImagesFromToolCalls(result.tool_calls);
      if (images.length > 0) {
        addToLog('images', `${images.length} image(s)`, images);
      }
    }

    if (showResults && result.tool_calls) {
      onResult({ success: true, data: result.tool_calls });
    }
  }, [addToLog, dispatch, showResults, onResult]);

  /** Submit a query — core logic shared by form submit and suggestion clicks */
  const submitQuery = useCallback(async (text) => {
    const userInput = text.trim();
    if (!userInput || isProcessing) {
      return;
    }

    dispatch(setInputText(''));
    dispatch(setIsProcessing(true));

    addToLog('user', userInput);

    const runAgentQuery = async () => {
      const agentResponse = await llmQueryClient.agentQuery({
        query: userInput,
        session_id: sessionId,
        max_iterations: 10
      });
      processAgentResponse(agentResponse);
    };

    try {
      if (canAskFollowup) {
        let analysisResponse;
        try {
          addToLog('system', '🤔 Analyzing if question can be answered from existing context...');
          analysisResponse = await llmQueryClient.analyzeFollowup({
            session_id: sessionId,
            question: userInput
          });
        } catch {
          await runAgentQuery();
          return;
        }

        if (!analysisResponse.success) {
          await runAgentQuery();
          return;
        }

        const analysis = analysisResponse.data;

        if (!analysis.needs_query) {
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
          addToLog('system', `🔍 Need new data: ${analysis.reasoning}`);
          await runAgentQuery();
        }
      } else {
        await runAgentQuery();
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      console.error('Error processing input:', error);
      addToLog('system', `❌ Error: ${msg}`);
      onResult({ error: msg });
    } finally {
      dispatch(setIsProcessing(false));
    }
  }, [sessionId, canAskFollowup, isProcessing, processAgentResponse, addToLog, onResult, dispatch]);

  const handleSubmit = useCallback(async (e) => {
    e?.preventDefault();
    await submitQuery(inputText);
  }, [inputText, submitQuery]);

  /** Load a past conversation by session ID */
  const handleSelectConversation = useCallback(async (selectedSessionId) => {
    setIsLoadingConversation(true);
    try {
      const response = await mutationClient.executeQuery({
        schema_name: 'ai_conversations',
        fields: ['session_id', 'timestamp', 'query', 'answer', 'tool_calls_json'],
        filter: createHashKeyFilter(selectedSessionId),
      });

      if (!response.success || !response.data) {
        addToLog('system', 'Failed to load conversation');
        dispatch(setViewMode('chat'));
        return;
      }

      const records = response.data?.results || response.data?.data || [];
      if (!Array.isArray(records) || records.length === 0) {
        dispatch(startNewConversation());
        return;
      }

      const sorted = [...records].sort((a, b) => {
        const fa = a.fields || a;
        const fb = b.fields || b;
        return (fa.timestamp || '').localeCompare(fb.timestamp || '');
      });

      const messages = [];
      for (const record of sorted) {
        const fields = record.fields || record;
        const timestamp = fields.timestamp || new Date().toISOString();

        if (fields.query) {
          messages.push({ type: 'user', content: fields.query, timestamp });
        }

        if (fields.tool_calls_json) {
          try {
            const toolCalls = JSON.parse(fields.tool_calls_json);
            if (Array.isArray(toolCalls) && toolCalls.length > 0) {
              messages.push({
                type: 'system',
                content: `Made ${toolCalls.length} tool call(s)`,
                timestamp,
              });
              messages.push({
                type: 'results',
                content: 'Tool execution trace',
                data: toolCalls,
                timestamp,
              });
            }
          } catch {
            // Ignore malformed tool_calls_json
          }
        }

        if (fields.answer) {
          messages.push({ type: 'system', content: fields.answer, timestamp });
        }

        if (fields.tool_calls_json) {
          try {
            const toolCalls = JSON.parse(fields.tool_calls_json);
            const images = extractImagesFromToolCalls(toolCalls);
            if (images.length > 0) {
              messages.push({ type: 'images', content: `${images.length} image(s)`, data: images, timestamp });
            }
          } catch { /* already handled above */ }
        }
      }

      dispatch(loadConversation({ sessionId: selectedSessionId, messages }));
    } catch (err) {
      console.error('Error loading conversation:', err);
      dispatch(setViewMode('chat'));
    } finally {
      setIsLoadingConversation(false);
    }
  }, [dispatch, addToLog]);

  const handleBackToList = useCallback(() => {
    dispatch(setViewMode('list'));
  }, [dispatch]);

  const handleNewConversation = useCallback(() => {
    dispatch(startNewConversation());
  }, [dispatch]);

  if (viewMode === 'list') {
    return (
      <ConversationList
        onSelectConversation={handleSelectConversation}
        onNewConversation={handleNewConversation}
      />
    );
  }

  return (
    <div className="flex flex-col h-[600px]">
      {/* Header bar */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-border">
        <button
          onClick={handleBackToList}
          className="text-sm text-gruvbox-blue hover:underline bg-transparent border-none cursor-pointer"
        >
          &larr; Conversations
        </button>
        <button onClick={handleNewConversation} className="btn-primary btn-sm">
          New Conversation
        </button>
      </div>

      {/* Conversation Log */}
      <div className="flex-1 overflow-y-auto p-6 space-y-3">
        {isLoadingConversation ? (
          <div className="flex items-center justify-center h-full text-secondary">
            <p className="text-sm">Loading conversation...</p>
          </div>
        ) : conversationLog.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-secondary">
            <div className="text-4xl mb-4">→</div>
            <p className="text-base mb-4">Start a conversation</p>
            <div className="flex flex-wrap gap-2 justify-center max-w-lg">
              {STARTER_SUGGESTIONS.map((suggestion) => (
                <button
                  key={suggestion}
                  onClick={() => submitQuery(suggestion)}
                  className="border border-border rounded-lg px-4 py-2 text-sm text-secondary hover:bg-surface-hover cursor-pointer bg-transparent"
                >
                  {suggestion}
                </button>
              ))}
            </div>
          </div>
        ) : (
          conversationLog.map((entry, idx) => (
            <div key={entry.id || `msg-${idx}`} className="mb-3">
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

              {entry.type === 'images' && Array.isArray(entry.data) && entry.data.length > 0 && (
                <div className="flex justify-start">
                  <div className="max-w-[80%] px-4 py-3 bg-surface-secondary border border-border rounded-lg">
                    <p className="text-xs text-tertiary mb-2">{entry.data.length} image{entry.data.length !== 1 ? 's' : ''}</p>
                    <div className="flex flex-wrap gap-2">
                      {entry.data.map((img) => (
                        <ImageThumbnail key={img.fileHash} fileHash={img.fileHash} sourceFile={img.sourceFile} />
                      ))}
                    </div>
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
        {isProcessing && (
          <div className="flex justify-start mb-3">
            <div className="px-4 py-3 bg-surface-secondary border border-border rounded-lg flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-tertiary animate-bounce" style={{ animationDelay: '0ms' }} />
              <span className="w-2 h-2 rounded-full bg-tertiary animate-bounce" style={{ animationDelay: '150ms' }} />
              <span className="w-2 h-2 rounded-full bg-tertiary animate-bounce" style={{ animationDelay: '300ms' }} />
            </div>
          </div>
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
