/**
 * ConversationList Component - Shows previous AI conversations
 *
 * Fetches all ai_conversations records, groups by session_id,
 * and displays clickable cards sorted by most recent.
 */

import { useState, useEffect, useCallback } from 'react';
import { mutationClient } from '../../api/clients/mutationClient';

function ConversationList({ onSelectConversation, onNewConversation }) {
  const [sessions, setSessions] = useState([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);

  const fetchConversations = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await mutationClient.executeQuery({
        schema_name: 'ai_conversations',
        fields: ['session_id', 'timestamp', 'query'],
      });

      if (!response.success || !response.data) {
        // Schema doesn't exist yet or no data — show empty state
        setSessions([]);
        return;
      }

      const records = response.data?.results || response.data?.data || [];
      if (!Array.isArray(records) || records.length === 0) {
        setSessions([]);
        return;
      }

      // Group by session_id
      const grouped = {};
      for (const record of records) {
        const fields = record.fields || record;
        const sessionId = fields.session_id;
        const timestamp = fields.timestamp;
        const query = fields.query;

        if (!sessionId) continue;

        if (!grouped[sessionId]) {
          grouped[sessionId] = [];
        }
        grouped[sessionId].push({ timestamp, query });
      }

      // Build session summaries
      const summaries = Object.entries(grouped).map(([sessionId, turns]) => {
        // Sort turns by timestamp ascending
        turns.sort((a, b) => (a.timestamp || '').localeCompare(b.timestamp || ''));

        const firstQuery = turns[0]?.query || 'Untitled conversation';
        const lastTimestamp = turns[turns.length - 1]?.timestamp || '';
        const turnCount = turns.length;

        return { sessionId, firstQuery, lastTimestamp, turnCount };
      });

      // Sort sessions by lastTimestamp descending (most recent first)
      summaries.sort((a, b) => b.lastTimestamp.localeCompare(a.lastTimestamp));

      setSessions(summaries);
    } catch (err) {
      // Schema not found or network error — show empty state for schema errors
      const message = err?.message || String(err);
      if (message.includes('not found') || message.includes('schema')) {
        setSessions([]);
      } else {
        setError(message);
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchConversations();
  }, [fetchConversations]);

  return (
    <div className="flex flex-col h-[600px]">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border">
        <h2 className="text-base font-medium text-primary">Previous Conversations</h2>
        <button onClick={onNewConversation} className="btn-primary btn-sm">
          + New
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-3">
        {isLoading && (
          <div className="flex items-center justify-center h-full text-secondary">
            <p className="text-sm">Loading conversations...</p>
          </div>
        )}

        {!isLoading && error && (
          <div className="flex items-center justify-center h-full text-secondary">
            <p className="text-sm text-red-400">Failed to load conversations: {error}</p>
          </div>
        )}

        {!isLoading && !error && sessions.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-secondary">
            <div className="text-4xl mb-4">&rarr;</div>
            <p className="text-base mb-2">No previous conversations</p>
            <p className="text-sm text-tertiary">
              Start a new conversation to begin
            </p>
          </div>
        )}

        {!isLoading && !error && sessions.map((session) => (
          <button
            key={session.sessionId}
            onClick={() => onSelectConversation(session.sessionId)}
            className="w-full text-left px-4 py-3 bg-surface-secondary border border-border rounded-lg hover:bg-gruvbox-hover cursor-pointer transition-colors"
          >
            <p className="text-sm text-primary truncate">{session.firstQuery}</p>
            <p className="text-xs text-tertiary mt-1">
              {session.lastTimestamp} &middot; {session.turnCount} turn{session.turnCount !== 1 ? 's' : ''}
            </p>
          </button>
        ))}
      </div>
    </div>
  );
}

export default ConversationList;
