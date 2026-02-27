use super::FoldNode;
use crate::error::FoldDbResult;

impl FoldNode {
    /// Get all backfill information
    pub async fn get_all_backfills(
        &self,
    ) -> FoldDbResult<Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>>
    {
        let db = self.db.lock().await;
        Ok(db.get_all_backfills())
    }

    /// Get active backfills
    pub async fn get_active_backfills(
        &self,
    ) -> FoldDbResult<Vec<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>>
    {
        let db = self.db.lock().await;
        Ok(db.get_active_backfills())
    }

    /// Get specific backfill info by transform ID
    pub async fn get_backfill(
        &self,
        transform_id: &str,
    ) -> FoldDbResult<Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>>
    {
        let db = self.db.lock().await;
        Ok(db.get_backfill(transform_id))
    }

    /// Get specific backfill info by backfill hash
    pub async fn get_backfill_by_hash(
        &self,
        hash: &str,
    ) -> FoldDbResult<Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>>
    {
        let db = self.db.lock().await;
        Ok(db.get_backfill_tracker().get_backfill_by_hash(hash))
    }

    /// Get event statistics
    pub async fn get_event_statistics(
        &self,
    ) -> FoldDbResult<crate::fold_db_core::infrastructure::event_monitor::EventStatistics> {
        let db = self.db.lock().await;
        Ok(db.get_event_statistics())
    }
}

