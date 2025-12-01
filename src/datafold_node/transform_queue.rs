use super::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TransformQueueInfo {
    pub queue: Vec<String>,
    pub length: usize,
    pub is_empty: bool,
}

impl DataFoldNode {
    /// Add a transform to the queue
    pub fn add_transform_to_queue(&self, transform_id: &str) -> FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        let orchestrator = db.transform_orchestrator()
            .ok_or_else(|| FoldDbError::Config("Transform orchestrator not available (requires Sled backend)".to_string()))?;
        orchestrator
            .add_transform(transform_id, "manual")?;
        Ok(())
    }

    /// Get information about the transform queue
    pub fn get_transform_queue_info(&self) -> FoldDbResult<TransformQueueInfo> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        let orchestrator = db.transform_orchestrator()
            .ok_or_else(|| FoldDbError::Config("Transform orchestrator not available (requires Sled backend)".to_string()))?;
        let queue = orchestrator.list_queued_transforms()?;
        let queue_length = queue.len();
        let is_empty = orchestrator.is_empty()?;
        Ok(TransformQueueInfo {
            queue,
            length: queue_length,
            is_empty,
        })
    }

    /// Get all backfill information
    pub fn get_all_backfills(&self) -> FoldDbResult<Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.get_all_backfills())
    }

    /// Get active backfills
    pub fn get_active_backfills(&self) -> FoldDbResult<Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.get_active_backfills())
    }

    /// Get specific backfill info by transform ID
    pub fn get_backfill(&self, transform_id: &str) -> FoldDbResult<Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.get_backfill(transform_id))
    }

    /// Get specific backfill info by backfill hash
    pub fn get_backfill_by_hash(&self, hash: &str) -> FoldDbResult<Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.get_backfill_tracker().get_backfill_by_hash(hash))
    }

    /// Get event statistics
    pub fn get_event_statistics(&self) -> FoldDbResult<crate::fold_db_core::infrastructure::event_monitor::EventStatistics> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.get_event_statistics())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datafold_node::config::NodeConfig;

    use tempfile::tempdir;

    #[tokio::test]
    async fn queue_info_works() {
        let dir = tempdir().unwrap();
        let config = NodeConfig {
            database: crate::datafold_node::config::DatabaseConfig::default(),
            storage_path: dir.path().to_path_buf(),
            default_trust_distance: 1,
            network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
            security_config: crate::security::SecurityConfig::default(),
            schema_service_url: Some("test://mock".to_string()),
        };
        let node = DataFoldNode::new(config).await.unwrap();
        let info = node.get_transform_queue_info().unwrap();
        assert!(info.is_empty);
        assert_eq!(info.length, 0);
    }
}
