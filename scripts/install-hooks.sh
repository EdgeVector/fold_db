#!/usr/bin/env bash
# Install the committed git hooks into this clone's .git/hooks/.
# Idempotent — safe to re-run.
#
# Why committed hooks: keeps every checkout gated on `cargo fmt --check`
# (and anything else we add to `hooks/`) without a separate tool like
# husky or pre-commit. Each developer runs this once after clone.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_SRC="$REPO_ROOT/hooks"

# Resolve real .git dir (works for both regular repos and submodule worktrees).
GITDIR=$(git -C "$REPO_ROOT" rev-parse --git-dir)
case "$GITDIR" in
    /*) HOOKS_DST="$GITDIR/hooks" ;;
    *)  HOOKS_DST="$REPO_ROOT/$GITDIR/hooks" ;;
esac

mkdir -p "$HOOKS_DST"
for hook in "$HOOKS_SRC"/*; do
    name=$(basename "$hook")
    cp "$hook" "$HOOKS_DST/$name"
    chmod +x "$HOOKS_DST/$name"
    echo "installed: $HOOKS_DST/$name"
done
