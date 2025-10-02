# Agent Guidelines

All contributors must ensure that every new feature is accompanied by appropriate tests. Write comprehensive tests for each feature you build before submitting your changes.

read .cursorrules and follow the instructions.

1. Keep code DRY.
2. Limit file size for code.  Break out into helpers and utilities
3. Limit line length for function.  Break out into separate functions where possible.

For refactoring try to delete more lines than you add.

Find simplifications if you can.

No silent failures.

validation:
1. run cargo test
2. run cargo clippy   // fix any linting issues
3. run npm tests in fold_node/src/datafold_node/static-react