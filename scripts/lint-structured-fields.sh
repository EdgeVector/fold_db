#!/bin/sh
# lint-structured-fields.sh
#
# Educational, warn-only lint that highlights `tracing` call sites whose
# message string carries positional `{}` / `{:?}` / `{name}` placeholders
# instead of emitting structured fields.
#
# Why: a positional arg becomes part of the flat message string. Honeycomb,
# Sentry, and our local query tools index *fields*, not message bodies, so
# `tracing::warn!("upload to '{}' failed: {}", target, err)` produces a log
# line you cannot pivot on by `target` or `err`. The structured form
#
#     tracing::warn!(target = %target, error = %err, "upload failed");
#
# emits the same human-readable text plus two queryable fields.
#
# This is intentionally non-blocking. The script always exits 0 — CI prints
# the suggestions to the job summary so reviewers see them, but no PR is
# blocked. Migrating existing sites is out of scope here; the goal is to
# stop the bleed at the call-site level.
#
# Scope: `crates/*/src/` only — same scope as `lint-redaction.sh` and
# `lint-spawn-instrument.sh`.
#
# Usage:    sh scripts/lint-structured-fields.sh
# Exit:     0, always.
#
# See docs/observability/structured-fields-lint.md for guidance.

set -eu

# Match `tracing::{info,warn,debug,error,trace}!(...,"...{...}...", ...)` —
# the message string contains a `{...}` placeholder and is followed by a
# comma, which means at least one positional/inline arg trails it. We keep
# the leading-arg slop (`[^"]*`) so the match also fires when a structured
# field precedes the message (mixed-style call) — that is still a candidate
# for full conversion.
PATTERN='tracing::(info|warn|debug|error|trace)!\([^"]*"[^"]*\{[^"]*\}[^"]*",'

SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

if ! command -v rg >/dev/null 2>&1; then
    # Mirror lint-redaction.sh: ripgrep is a hard requirement on dev
    # machines and CI installs it explicitly. Stay quiet otherwise.
    echo "lint-structured-fields: ripgrep (rg) not found in PATH — skipping" >&2
    exit 0
fi

found_any=0
for d in crates/*/src; do
    [ -d "$d" ] && found_any=1
done
if [ "$found_any" -eq 0 ]; then
    echo "lint-structured-fields: no crates/*/src directories found at $REPO_ROOT" >&2
    exit 0
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT INT HUP TERM

rg --pcre2 -n "$PATTERN" crates/*/src > "$tmp" 2>/dev/null || true

hits=0
while IFS= read -r match; do
    [ -z "$match" ] && continue
    hits=$((hits + 1))
done < "$tmp"

if [ "$hits" -eq 0 ]; then
    echo "lint-structured-fields: ok — no positional-arg tracing call sites in crates/*/src/."
    exit 0
fi

cat <<EOF
lint-structured-fields: $hits site(s) use positional/inline message args
instead of structured fields. This is warn-only — no PR is blocked.

Why prefer structured fields: positional args are baked into the flat
message string and cannot be queried as fields in Honeycomb / Sentry /
local log tools. Structured fields can.

Before:
    tracing::warn!("upload to '{}' failed: {}", target, err);

After:
    tracing::warn!(target = %target, error = %err, "upload failed");

Sites flagged:
EOF

while IFS= read -r match; do
    [ -z "$match" ] && continue
    file=${match%%:*}
    rest=${match#*:}
    lineno=${rest%%:*}
    content=${rest#*:}
    # Trim leading whitespace from the source line for legible output.
    trimmed=$(printf '%s' "$content" | sed 's/^[[:space:]]*//')
    printf '  %s:%s\n      %s\n' "$file" "$lineno" "$trimmed"
done < "$tmp"

cat <<'EOF'

See docs/observability/structured-fields-lint.md for the full rationale.
EOF

exit 0
