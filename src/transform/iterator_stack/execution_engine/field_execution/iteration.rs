//! Iteration helpers for field execution
//!
//! Contains iteration logic and recursive traversal methods for
//! processing field expressions across different depths.

use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::execution_engine::iterator_management::IteratorManager;
use crate::transform::iterator_stack::types::IteratorStack;
use log::debug;

/// Helper methods for iteration logic
pub struct IterationHelper;

impl IterationHelper {
    /// Iterates to a specific depth and calls a callback for each combination
    pub fn iterate_to_depth<F>(
        stack: &mut IteratorStack,
        target_depth: usize,
        mut callback: F,
    ) -> IteratorStackResult<()>
    where
        F: FnMut(&mut IteratorStack, &[usize]) -> IteratorStackResult<()>,
    {
        debug!(
            "iterate_to_depth called with target_depth: {}, stack len: {}",
            target_depth,
            stack.len()
        );
        Self::iterate_recursive(stack, target_depth, &mut callback, &mut Vec::new())
    }

    /// Recursive iteration helper
    #[allow(clippy::only_used_in_recursion)]
    fn iterate_recursive<F>(
        stack: &mut IteratorStack,
        target_depth: usize,
        callback: &mut F,
        current_path: &mut Vec<usize>,
    ) -> IteratorStackResult<()>
    where
        F: FnMut(&mut IteratorStack, &[usize]) -> IteratorStackResult<()>,
    {
        debug!(
            "iterate_recursive: current_path.len()={}, target_depth={}",
            current_path.len(),
            target_depth
        );

        if current_path.len() > target_depth {
            return Ok(());
        }

        if current_path.len() == target_depth {
            // We've reached the target depth, iterate over all items at this depth
            debug!("Reached target depth, iterating over items");
            let current_depth = current_path.len();
            if let Some(context) = stack.context_at_depth(current_depth) {
                let items = context.iterator_state.items.clone();
                debug!("Found {} items at target depth", items.len());

                for (index, _item) in items.iter().enumerate() {
                    debug!("Processing item {} at target depth", index);
                    // Set the current item for this depth
                    if let Some(context) = stack.context_at_depth_mut(current_depth) {
                        context.iterator_state.current_item = Some(items[index].clone());
                    }

                    current_path.push(index);

                    // Call the callback for this item
                    callback(stack, current_path)?;

                    current_path.pop();
                }
            }
            return Ok(());
        }

        // Get the current depth
        let current_depth = current_path.len();

        if let Some(context) = stack.context_at_depth(current_depth) {
            let items = context.iterator_state.items.clone();
            debug!("At depth {}, found {} items", current_depth, items.len());

            for (index, _item) in items.iter().enumerate() {
                debug!("Processing item {} at depth {}", index, current_depth);
                // Set the current item for this depth
                if let Some(context) = stack.context_at_depth_mut(current_depth) {
                    context.iterator_state.current_item = Some(items[index].clone());
                }

                current_path.push(index);

                // Update child scopes so nested iterators reflect the current parent item
                Self::prepare_child_scope(stack, current_depth)?;

                // Recursively iterate to the next depth
                Self::iterate_recursive(stack, target_depth, callback, current_path)?;

                current_path.pop();
            }
        }

        Ok(())
    }

    fn prepare_child_scope(
        stack: &mut IteratorStack,
        parent_depth: usize,
    ) -> IteratorStackResult<()> {
        let next_depth = parent_depth + 1;
        if next_depth >= stack.len() {
            return Ok(());
        }

        let parent_item = stack
            .context_at_depth(parent_depth)
            .and_then(|context| context.iterator_state.current_item.clone());

        let Some(parent_value) = parent_item else {
            return Ok(());
        };

        let Some(child_scope) = stack.scope_at_depth(next_depth) else {
            return Ok(());
        };

        let manager = IteratorManager::new();
        let items =
            manager.extract_items_for_iterator(&child_scope.iterator_type, &parent_value)?;

        if let Some(child_context) = stack.context_at_depth_mut(next_depth) {
            child_context.iterator_state.items = items.clone();
            child_context.iterator_state.current_item = items.first().cloned();
            child_context.iterator_state.completed = items.is_empty();
            child_context
                .values
                .insert(format!("depth_{}", next_depth), parent_value);
        }

        Ok(())
    }
}
