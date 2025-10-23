/**
 * LlmQueryTab Component - Conversational AI Query Interface
 * 
 * A simplified chat-style interface where the AI automatically loops through
 * queries until it finds data or determines it doesn't exist.
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { llmQueryClient } from '../../api/clients/llmQueryClient';

function LlmQueryTab({ onResult }) {
  // State management
  const [inputText, setInputText] = useState('');
  const [sessionId, setSessionId] = useState(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [conversationLog, setConversationLog] = useState([]);
  const [showResults, setShowResults] = useState(false);
  const conversationEndRef = useRef(null);

  // Auto-scroll to bottom when conversation updates
  useEffect(() => {
    conversationEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [conversationLog]);

  /**
   * Add a message to the conversation log
   */
  const addToLog = useCallback((type, content, data = null) => {
    setConversationLog(prev => [...prev, {
      type, // 'user', 'system', 'results'
      content,
      data,
      timestamp: new Date().toISOString()
    }]);
  }, []);

  /**
   * Handle user input - run query or ask follow-up
   */
  const handleSubmit = useCallback(async (e) => {
    e?.preventDefault();
    
    if (!inputText.trim() || isProcessing) {
      return;
    }

    const userInput = inputText.trim();
    setInputText('');
    setIsProcessing(true);

    // Add user message to log
    addToLog('user', userInput);

    try {
      // If this is a follow-up question (session exists and we have results)
      if (sessionId && conversationLog.some(log => log.type === 'results')) {
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
          // Needs new query - use AI-native index
          addToLog('system', `🔍 Need new data: ${analysis.reasoning}`);
          addToLog('system', '🔍 Using AI-native index search...');
          
          const response = await fetch('/api/llm-query/native-index', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({
              query: userInput,
              session_id: sessionId
            }),
          });

          if (!response.ok) {
            const errorData = await response.json();
            addToLog('system', `❌ Error: ${errorData.error || 'Failed to run AI-native index query'}`);
            return;
          }

          const result = await response.json();
          addToLog('system', '✅ AI-native index search completed');
          
          // Update session ID if returned from the server
          if (result.session_id) {
            setSessionId(result.session_id);
          }
          
          // Display the AI's interpretation of the results in the conversation
          addToLog('system', result.ai_interpretation);
          
          // Also add the raw results for detailed view
          addToLog('results', 'Raw search results', result.raw_results);
          // Only show results section if user has expanded details
          if (showResults) {
            onResult({ success: true, data: result.raw_results });
          }
        }
      } else {
        // New query - use AI-native index
        addToLog('system', '🔍 Using AI-native index search...');
        
        const response = await fetch('/api/llm-query/native-index', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            query: userInput,
            session_id: sessionId
          }),
        });

        if (!response.ok) {
          const errorData = await response.json();
          addToLog('system', `❌ Error: ${errorData.error || 'Failed to run AI-native index query'}`);
          return;
        }

        const result = await response.json();
        addToLog('system', '✅ AI-native index search completed');
        
        // Update session ID if returned from the server
        if (result.session_id) {
          setSessionId(result.session_id);
        }
        
        // Display the AI's interpretation of the results in the conversation
        addToLog('system', result.ai_interpretation);
        
        // Also add the raw results for detailed view
        addToLog('results', 'Raw search results', result.raw_results);
        // Only show results section if user has expanded details
        if (showResults) {
          onResult({ success: true, data: result.raw_results });
        }
      }
    } catch (error) {
      console.error('Error processing input:', error);
      addToLog('system', `❌ Error: ${error.message}`);
      onResult({ error: error.message });
    } finally {
      setIsProcessing(false);
    }
  }, [inputText, sessionId, conversationLog, isProcessing, addToLog, onResult]);

  /**
   * Start a new conversation
   */
  const handleNewConversation = useCallback(() => {
    setSessionId(null);
    setConversationLog([]);
    setInputText('');
    setIsProcessing(false);
  }, []);

  return (
    <div className="flex flex-col bg-white rounded-lg shadow">
      {/* Header */}
      <div className="p-4 border-b border-gray-200 flex justify-between items-center">
        <div>
          <h2 className="text-xl font-bold text-gray-900">
            🤖 AI Data Assistant
          </h2>
          <p className="text-sm text-gray-600">
            Ask questions in plain English - I'll find your data
          </p>
        </div>
        {conversationLog.length > 0 && (
          <button
            onClick={handleNewConversation}
            disabled={isProcessing}
            className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors text-sm"
          >
            New Conversation
          </button>
        )}
      </div>

      {/* Conversation Log */}
      <div className="overflow-y-auto bg-gray-50 p-4 space-y-3" style={{ maxHeight: '60vh', minHeight: '400px' }}>
        {conversationLog.length === 0 ? (
          <div className="text-center text-gray-500 mt-20">
            <div className="text-6xl mb-4">💬</div>
            <p className="text-lg mb-2">Start a conversation</p>
            <p className="text-sm">
              Try: "Find all blog posts from last month" or "Show me products over $100"
            </p>
          </div>
        ) : (
          conversationLog.map((entry, idx) => (
            <div key={idx}>
              {entry.type === 'user' && (
                <div className="flex justify-end">
                  <div className="bg-blue-600 text-white rounded-lg px-4 py-2 max-w-3xl">
                    <p className="text-sm font-semibold mb-1">You</p>
                    <p className="whitespace-pre-wrap">{entry.content}</p>
                  </div>
                </div>
              )}
              
              {entry.type === 'system' && (
                <div className="flex justify-start">
                  <div className="bg-white border border-gray-200 rounded-lg px-4 py-2 max-w-3xl">
                    <p className="text-sm font-semibold text-gray-700 mb-1">AI Assistant</p>
                    <p className="text-gray-900 whitespace-pre-wrap">{entry.content}</p>
                  </div>
                </div>
              )}
              
              {entry.type === 'results' && entry.data && (
                <div className="bg-green-50 border border-green-200 rounded-lg p-4 max-w-full">
                  <div className="flex justify-between items-center mb-2">
                    <p className="text-sm font-semibold text-green-800">📊 Results ({entry.data.length})</p>
                    <button
                      onClick={() => {
                        const newShowResults = !showResults;
                        setShowResults(newShowResults);
                        // Show/hide the results section based on toggle
                        if (newShowResults) {
                          // Find the results data from conversation log
                          const resultsEntry = conversationLog.find(log => log.type === 'results');
                          if (resultsEntry && resultsEntry.data) {
                            onResult({ success: true, data: resultsEntry.data });
                          }
                        } else {
                          // Hide the results section
                          onResult(null);
                        }
                      }}
                      className="text-sm text-green-700 hover:text-green-900 underline"
                    >
                      {showResults ? 'Hide Details' : 'Show Details'}
                    </button>
                  </div>
                  {showResults && (
                    <>
                      <div className="bg-white rounded p-3 mb-2">
                        <p className="text-gray-900 whitespace-pre-wrap mb-3">{entry.content}</p>
                      </div>
                      <details className="mt-2">
                        <summary className="cursor-pointer text-sm text-green-700 hover:text-green-900">
                          View raw data ({entry.data.length} records)
                        </summary>
                        <div className="mt-2 max-h-64 overflow-auto">
                          <pre className="text-xs bg-gray-900 text-green-400 p-3 rounded">
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
      <form onSubmit={handleSubmit} className="border-t border-gray-200 p-4 bg-white">
        <div className="flex gap-2">
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            placeholder={
              conversationLog.some(log => log.type === 'results')
                ? "Ask a follow-up question or start a new query..."
                : "Search the native index (e.g., 'Find posts about AI')..."
            }
            disabled={isProcessing}
            className="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100"
            autoFocus
          />
          <button
            type="submit"
            disabled={!inputText.trim() || isProcessing}
            className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold"
          >
            {isProcessing ? '⏳ Processing...' : 'Send'}
          </button>
        </div>
        {isProcessing && (
          <p className="text-center text-sm text-gray-500 mt-2">
            AI is analyzing and searching...
          </p>
        )}
      </form>
    </div>
  );
}

export default LlmQueryTab;

