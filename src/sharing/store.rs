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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharing::types::{ShareRule, ShareScope, ShareSubscription};
    use tempfile::tempdir;

    fn make_pool() -> Arc<SledPool> {
        let dir = tempdir().unwrap();
        Arc::new(SledPool::new(dir.keep()))
    }

    fn make_rule(rule_id: &str, recipient: &str) -> ShareRule {
        ShareRule {
            rule_id: rule_id.to_string(),
            recipient_pubkey: recipient.to_string(),
            recipient_display_name: "Test".to_string(),
            scope: ShareScope::AllSchemas,
            share_prefix: format!("share:me:{}", recipient),
            share_e2e_secret: vec![1, 2, 3, 4],
            active: true,
            created_at: 1000,
            writer_pubkey: "me".to_string(),
            signature: String::new(),
        }
    }

    #[test]
    fn test_create_and_list_share_rule() {
        let pool = make_pool();
        let rule = make_rule("r1", "alice");
        create_share_rule(&pool, rule.clone()).unwrap();
        let listed = list_share_rules(&pool).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].rule_id, "r1");
        assert_eq!(listed[0].recipient_pubkey, "alice");
        assert!(listed[0].active);
    }

    #[test]
    fn test_deactivate_share_rule() {
        let pool = make_pool();
        let rule = make_rule("r1", "alice");
        create_share_rule(&pool, rule).unwrap();
        deactivate_share_rule(&pool, "r1").unwrap();
        let listed = list_share_rules(&pool).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].active);
    }

    #[test]
    fn test_deactivate_nonexistent_rule_errors() {
        let pool = make_pool();
        let result = deactivate_share_rule(&pool, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_and_get_share_subscription() {
        let pool = make_pool();
        let sub = ShareSubscription {
            sender_pubkey: "alice".to_string(),
            share_prefix: "share:alice:me".to_string(),
            share_e2e_secret: vec![5, 6, 7, 8],
            accepted_at: 2000,
            active: true,
        };
        create_share_subscription(&pool, sub.clone()).unwrap();
        let got = get_share_subscription(&pool, "alice").unwrap().unwrap();
        assert_eq!(got.sender_pubkey, "alice");
        assert_eq!(got.share_e2e_secret, vec![5, 6, 7, 8]);
        assert_eq!(got.accepted_at, 2000);
    }

    #[test]
    fn test_get_nonexistent_subscription_returns_none() {
        let pool = make_pool();
        let got = get_share_subscription(&pool, "nobody").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn test_list_share_subscriptions_multiple() {
        let pool = make_pool();
        for (sender, secret) in [("alice", 1u8), ("bob", 2u8)] {
            let sub = ShareSubscription {
                sender_pubkey: sender.to_string(),
                share_prefix: format!("share:{}:me", sender),
                share_e2e_secret: vec![secret; 4],
                accepted_at: 1000,
                active: true,
            };
            create_share_subscription(&pool, sub).unwrap();
        }
        let subs = list_share_subscriptions(&pool).unwrap();
        assert_eq!(subs.len(), 2);
    }
}
