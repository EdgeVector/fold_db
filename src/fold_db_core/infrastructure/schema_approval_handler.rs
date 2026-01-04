//! Schema Approval Handler
//!
//! Handles schema approval events and orchestrates backfill operations for transforms.

use std::sync::Arc;

use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::transform::manager::types::TransformRunner;
use crate::transform::manager::TransformManager;

use super::backfill_tracker::BackfillTracker;
use super::message_bus::schema_events::SchemaApproved;

pub async fn handle_schema_approved(
    event: SchemaApproved,
    backfill_tracker: &Arc<BackfillTracker>,
    transform_manager: &Arc<TransformManager>,
) -> Result<(), crate::schema::SchemaError> {
    match transform_manager.transform_exists(&event.schema_name) {
        Ok(true) => {
            let transforms = transform_manager.list_transforms().map_err(|e| {
                crate::schema::SchemaError::InvalidTransform(format!(
                    "Failed to list transforms: {}",
                    e
                ))
            })?;

            if let Some(transform) = transforms.get(&event.schema_name) {
                handle_transform_schema_approval(
                    &event,
                    transform,
                    backfill_tracker,
                    transform_manager,
                )
                .await?;
            }
            Ok(())
        }
        Ok(false) => Ok(()),
        Err(e) => Err(crate::schema::SchemaError::InvalidTransform(format!(
            "Failed to check if transform exists for '{}': {}",
            event.schema_name, e
        ))),
    }
}

async fn handle_transform_schema_approval(
    event: &SchemaApproved,
    transform: &crate::schema::types::Transform,
    backfill_tracker: &Arc<BackfillTracker>,
    transform_manager: &Arc<TransformManager>,
) -> Result<(), crate::schema::SchemaError> {
    // Look up the transform's schema from the database
    let schema = transform_manager
        .db_ops
        .get_schema(transform.get_schema_name())
        .await?
        .ok_or_else(|| {
            crate::schema::SchemaError::InvalidTransform(format!(
                "Transform schema '{}' not found in database",
                transform.get_schema_name()
            ))
        })?;

    let source_schemas = schema.get_source_schemas();
    if source_schemas.is_empty() {
        return Err(crate::schema::SchemaError::InvalidTransform(format!(
            "Transform '{}' has no source schemas, cannot perform backfill",
            event.schema_name
        )));
    }

    // Ensure all source schemas are in the "approved" state
    for source_schema in &source_schemas {
        let state = transform_manager
            .db_ops
            .get_schema_state(source_schema)
            .await?;
        match state {
            Some(crate::schema::SchemaState::Approved) => {}
            Some(other) => {
                return Err(crate::schema::SchemaError::InvalidTransform(format!(
                    "Source schema '{}' is not approved (state: {:?})",
                    source_schema, other
                )));
            }
            None => {
                return Err(crate::schema::SchemaError::InvalidTransform(format!(
                    "Source schema '{}' does not exist",
                    source_schema
                )));
            }
        }
    }

    // Use the schema name for backfill
    let schema_name = schema.name.clone();

    let backfill_hash = event.backfill_hash.as_ref().ok_or_else(|| {
        crate::schema::SchemaError::InvalidTransform(format!(
            "SchemaApproved event for transform '{}' missing required backfill_hash",
            event.schema_name
        ))
    })?;

    backfill_tracker.start_backfill_with_hash(
        backfill_hash.clone(),
        event.schema_name.clone(),
        schema_name.clone(),
    );

    // Execute the transform backfill
    let result = handle_transform_backfill(
        &schema_name,
        transform_manager,
        backfill_tracker,
        backfill_hash,
    )
    .await;

    // If transform execution succeeded and produced 0 records, ensure backfill is marked complete
    // This handles the race condition where the event monitor thread might not have processed
    // the BackfillExpectedMutations event yet
    if result.is_ok() {
        // Give a tiny moment for any async operations to complete, then force completion if needed
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        if let Some(info) = backfill_tracker.get_backfill_by_hash(backfill_hash) {
            if info.status
                == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::InProgress
                && info.mutations_expected == 0
            {
                backfill_tracker.force_complete(backfill_hash);
            }
        }
    }

    result.inspect_err(|e| {
        backfill_tracker.fail_backfill(&event.schema_name, e.to_string());
    })
}

async fn handle_transform_backfill(
    transform_id: &str,
    transform_manager: &Arc<TransformManager>,
    backfill_tracker: &Arc<BackfillTracker>,
    backfill_hash: &str,
) -> Result<(), crate::schema::SchemaError> {
    let mutation_context = Some(MutationContext {
        key_value: None,
        mutation_hash: None,
        incremental: false,
        backfill_hash: Some(backfill_hash.to_string()),
    });

    match transform_manager
        .execute_transform_with_context(transform_id, &mutation_context)
        .await
    {
        Ok(result) => {
            // If the transform produced 0 records, immediately mark the backfill as completed
            // This handles the case where there's no source data to process
            // The event monitor will also process BackfillExpectedMutations, but this ensures
            // immediate completion for the zero-count case
            if result.records.is_empty() {
                // Directly mark as completed since there are no records to process
                // This avoids waiting for the async event monitor thread
                // We do both set_mutations_expected and force_complete to handle race conditions
                backfill_tracker.set_mutations_expected(backfill_hash, 0);
                backfill_tracker.force_complete(backfill_hash);

                // Verify it was set (for debugging)
                if let Some(info) = backfill_tracker.get_backfill_by_hash(backfill_hash) {
                    if info.status != crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed {
                        log::error!("Failed to mark backfill {} as completed, status is still: {:?}", backfill_hash, info.status);
                    }
                } else {
                    log::error!(
                        "Backfill {} not found after attempting to mark as completed",
                        backfill_hash
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            backfill_tracker.fail_backfill(transform_id, e.to_string());
            Err(e)
        }
    }
}
