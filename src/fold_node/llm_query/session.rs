//! Session management for LLM query workflow.

use super::types::SessionContext;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Manages user sessions with TTL-based expiration
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionContext>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session or return existing one
    pub fn create_or_get_session(
        &self,
        session_id: Option<String>,
        original_query: String,
    ) -> Result<String, String> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // Clean up expired sessions
        sessions.retain(|_, ctx| !ctx.is_expired());

        // If session_id provided and exists, update and return it.
        // If provided but expired/missing, recreate with the same ID for continuity.
        if let Some(id) = session_id {
            if let Some(ctx) = sessions.get_mut(&id) {
                ctx.update_activity();
                return Ok(id);
            }
            let context = SessionContext::new(id.clone(), original_query);
            sessions.insert(id.clone(), context);
            return Ok(id);
        }

        // No session_id provided — create new session with fresh ID
        let new_id = Uuid::new_v4().to_string();
        let context = SessionContext::new(new_id.clone(), original_query);
        sessions.insert(new_id.clone(), context);
        Ok(new_id)
    }

    /// Get a session context
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionContext>, String> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

        Ok(sessions.get(session_id).cloned())
    }

    /// Add results to a session
    pub fn add_results(
        &self,
        session_id: &str,
        results: Vec<serde_json::Value>,
    ) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        if let Some(ctx) = sessions.get_mut(session_id) {
            ctx.query_results = Some(results);
            ctx.update_activity();
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id))
        }
    }

    /// Add a message to session conversation history
    pub fn add_message(
        &self,
        session_id: &str,
        role: String,
        content: String,
    ) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        if let Some(ctx) = sessions.get_mut(session_id) {
            ctx.add_message(role, content);
            ctx.update_activity();
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id))
        }
    }

    /// Set the schema created for a session
    pub fn set_schema_created(&self, session_id: &str, schema_name: String) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        if let Some(ctx) = sessions.get_mut(session_id) {
            ctx.schema_created = Some(schema_name);
            ctx.update_activity();
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id))
        }
    }

}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let manager = SessionManager::new();
        let session_id = manager
            .create_or_get_session(None, "test query".to_string())
            .unwrap();
        assert!(!session_id.is_empty());
    }

    #[test]
    fn test_get_existing_session() {
        let manager = SessionManager::new();
        let session_id = manager
            .create_or_get_session(None, "test query".to_string())
            .unwrap();
        let session = manager.get_session(&session_id).unwrap();
        assert!(session.is_some());
    }

    #[test]
    fn test_add_results() {
        let manager = SessionManager::new();
        let session_id = manager
            .create_or_get_session(None, "test query".to_string())
            .unwrap();
        manager
            .add_results(&session_id, vec![serde_json::json!({"test": "data"})])
            .unwrap();
        let session = manager.get_session(&session_id).unwrap().unwrap();
        assert!(session.query_results.is_some());
    }
}
