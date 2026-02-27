#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK_SRC="$SCRIPT_DIR/pre-commit"
HOOK_DST="$REPO_DIR/.git/hooks/pre-commit"

if [ ! -f "$HOOK_SRC" ]; then
    echo "Error: pre-commit script not found at $HOOK_SRC"
    exit 1
fi

cp "$HOOK_SRC" "$HOOK_DST"
chmod +x "$HOOK_DST"
echo "Pre-commit hook installed at $HOOK_DST"
