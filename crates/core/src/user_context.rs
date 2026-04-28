//! Task-local propagation of the current user id.
//!
//! Background workers spawn `tokio::tasks` that need to know which user a
//! request belongs to (mutation pipelines, progress stores, the messaging
//! constructors). Wrap an async block with [`run_with_user`] and any code in
//! that task tree can read the id back via [`get_current_user_id`].
//!
//! Lives outside `logging` so it survives the LoggingSystem retirement —
//! user context is a request property, not a logging detail.

use std::future::Future;

tokio::task_local! {
    static CURRENT_USER_ID: String;
}

/// Run `f` with `user_id` bound for the duration of the future and any
/// tasks it spawns that inherit the task-local scope.
pub async fn run_with_user<F>(user_id: &str, f: F) -> F::Output
where
    F: Future,
{
    CURRENT_USER_ID.scope(user_id.to_string(), f).await
}

/// Read the current user id from task-local storage, or `None` if no
/// [`run_with_user`] frame is on the stack.
pub fn get_current_user_id() -> Option<String> {
    CURRENT_USER_ID.try_with(|id| id.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn outside_scope_returns_none() {
        assert!(get_current_user_id().is_none());
    }

    #[tokio::test]
    async fn inside_scope_returns_bound_id() {
        run_with_user("alice", async {
            assert_eq!(get_current_user_id().as_deref(), Some("alice"));
        })
        .await;
    }

    #[tokio::test]
    async fn nested_scope_overrides_outer() {
        run_with_user("alice", async {
            run_with_user("bob", async {
                assert_eq!(get_current_user_id().as_deref(), Some("bob"));
            })
            .await;
            assert_eq!(get_current_user_id().as_deref(), Some("alice"));
        })
        .await;
    }
}
