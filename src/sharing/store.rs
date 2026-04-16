use super::types::{ShareRule, ShareSubscription};
use crate::error::FoldDbError;
use crate::storage::SledPool;
use std::sync::Arc;

const SHARE_RULE_TREE: &str = "share_rules";
const SHARE_SUB_TREE: &str = "share_subscriptions";

fn rule_tree(pool: &Arc<SledPool>) -> Result<sled::Tree, FoldDbError> {
    let guard = pool
        .acquire_arc()
        .map_err(|e| FoldDbError::Database(format!("Failed to acquire SledPool: {}", e)))?;
    guard
        .db()
        .open_tree(SHARE_RULE_TREE)
        .map_err(|e| FoldDbError::Database(format!("Failed to open share_rules tree: {}", e)))
}

fn sub_tree(pool: &Arc<SledPool>) -> Result<sled::Tree, FoldDbError> {
    let guard = pool
        .acquire_arc()
        .map_err(|e| FoldDbError::Database(format!("Failed to acquire SledPool: {}", e)))?;
    guard.db().open_tree(SHARE_SUB_TREE).map_err(|e| {
        FoldDbError::Database(format!("Failed to open share_subscriptions tree: {}", e))
    })
}

pub fn create_share_rule(pool: &Arc<SledPool>, rule: ShareRule) -> Result<(), FoldDbError> {
    let tree = rule_tree(pool)?;
    let key = format!("share_rule:{}", rule.rule_id);
    let value = serde_json::to_vec(&rule)?;
    tree.insert(key.as_bytes(), value)
        .map_err(|e| FoldDbError::Database(format!("Failed to store share rule: {}", e)))?;
    Ok(())
}

pub fn list_share_rules(pool: &Arc<SledPool>) -> Result<Vec<ShareRule>, FoldDbError> {
    let tree = rule_tree(pool)?;
    let mut rules = Vec::new();
    for entry in tree.iter() {
        let (_, value) = entry.map_err(|e| {
            FoldDbError::Database(format!("Failed to iterate share rules tree: {}", e))
        })?;
        let rule: ShareRule = serde_json::from_slice(&value)?;
        rules.push(rule);
    }
    Ok(rules)
}

pub fn deactivate_share_rule(pool: &Arc<SledPool>, rule_id: &str) -> Result<(), FoldDbError> {
    let tree = rule_tree(pool)?;
    let key = format!("share_rule:{}", rule_id);

    if let Some(value) = tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read share rules: {}", e)))?
    {
        let mut rule: ShareRule = serde_json::from_slice(&value)?;
        rule.active = false;
        let value = serde_json::to_vec(&rule)?;
        tree.insert(key.as_bytes(), value)
            .map_err(|e| FoldDbError::Database(format!("Failed to update share rule: {}", e)))?;
        return Ok(());
    }

    Err(FoldDbError::Database(format!(
        "Share rule {} not found",
        rule_id
    )))
}

pub fn create_share_subscription(
    pool: &Arc<SledPool>,
    sub: ShareSubscription,
) -> Result<(), FoldDbError> {
    let tree = sub_tree(pool)?;
    let key = format!("share_sub:{}", sub.sender_pubkey);
    let value = serde_json::to_vec(&sub)?;
    tree.insert(key.as_bytes(), value)
        .map_err(|e| FoldDbError::Database(format!("Failed to store share subscription: {}", e)))?;
    Ok(())
}

pub fn list_share_subscriptions(
    pool: &Arc<SledPool>,
) -> Result<Vec<ShareSubscription>, FoldDbError> {
    let tree = sub_tree(pool)?;
    let mut subs = Vec::new();
    for entry in tree.iter() {
        let (_, value) = entry.map_err(|e| {
            FoldDbError::Database(format!("Failed to iterate share subscriptions tree: {}", e))
        })?;
        let sub: ShareSubscription = serde_json::from_slice(&value)?;
        subs.push(sub);
    }
    Ok(subs)
}

pub fn get_share_subscription(
    pool: &Arc<SledPool>,
    sender_pubkey: &str,
) -> Result<Option<ShareSubscription>, FoldDbError> {
    let tree = sub_tree(pool)?;
    let key = format!("share_sub:{}", sender_pubkey);

    if let Some(value) = tree
        .get(key.as_bytes())
        .map_err(|e| FoldDbError::Database(format!("Failed to read share subscriptions: {}", e)))?
    {
        let sub: ShareSubscription = serde_json::from_slice(&value)?;
        return Ok(Some(sub));
    }
    Ok(None)
}
