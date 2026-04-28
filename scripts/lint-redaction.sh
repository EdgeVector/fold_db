#!/bin/sh
# lint-redaction.sh
#
# Fail the build if a `tracing` macro emits a sensitive field as a raw value
# instead of through `observability::redact!()` / `observability::redact_id!()`.
#
# Sensitive fields (Phase 5 / T1):
#   password, token, api_key, secret, auth_token, email, phone, ssn
#
# Scope: `crates/*/src/` — the same scope as `lint-tracing-egress.sh`.
# Tests under `crates/*/tests/` and the FMT-layer self-tests are out of scope
# at the directory level; intentional raw-value test fixtures inside
# `crates/*/src/` use the inline override below.
#
# Override: add a comment containing the literal `lint:redaction-ok <reason>`
# on the violating line OR on the line immediately above it. The two-line
# window is so the override survives `rustfmt`, which will lift a long
# trailing comment onto its own line. Example:
#
#     // lint:redaction-ok FMT-layer test must emit raw value to verify deny-list
#     tracing::info!(password = "hunter2", "login");
#
# Use overrides sparingly — typically only for unit tests that need to feed
# the raw value to verify the FMT layer's deny-list.
#
# Usage: sh scripts/lint-redaction.sh
# Exit code: 0 if every match is wrapped or overridden, 1 otherwise.

set -eu

PATTERN='tracing::(info|warn|debug|error|trace)!.*?(password|token|api_key|secret|auth_token|email|phone|ssn)\s*=\s*[^,]'

SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

if ! command -v rg >/dev/null 2>&1; then
    echo "lint-redaction: ripgrep (rg) not found in PATH" >&2
    exit 1
fi

# Confirm at least one crate src dir exists, mirroring lint-tracing-egress.sh.
found_any=0
for d in crates/*/src; do
    [ -d "$d" ] && found_any=1
done
if [ "$found_any" -eq 0 ]; then
    echo "lint-redaction: no crates/*/src directories found at $REPO_ROOT" >&2
    exit 1
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT INT HUP TERM

# rg --pcre2 -n: numbered lines; `|| true` because rg exits 1 when no matches.
rg --pcre2 -n "$PATTERN" crates/*/src > "$tmp" 2>/dev/null || true

failed=0
hits=0

while IFS= read -r match; do
    [ -z "$match" ] && continue

    file=${match%%:*}
    rest=${match#*:}
    lineno=${rest%%:*}
    content=${rest#*:}

    # Override on the violating line itself.
    if printf '%s\n' "$content" | grep -q 'lint:redaction-ok'; then
        continue
    fi

    # Override on the line directly above (so rustfmt is free to lift a
    # trailing comment onto its own line without breaking the override).
    if [ "$lineno" -gt 1 ] && [ -r "$file" ]; then
        prev_lineno=$((lineno - 1))
        prev=$(sed -n "${prev_lineno}p" "$file" 2>/dev/null || true)
        if printf '%s\n' "$prev" | grep -q 'lint:redaction-ok'; then
            continue
        fi
    fi

    # Extract the right-hand side starting at the sensitive field name and
    # check whether it routes through redact!() / redact_id!() before the
    # next field separator. We accept either `field = %redact!(x)` (the
    # `tracing` `%`-display form) or a bare `redact!(...)` / `redact_id!(...)`.
    rhs=$(printf '%s\n' "$content" | grep -oE '(password|token|api_key|secret|auth_token|email|phone|ssn)[[:space:]]*=[^,]*' | head -1)
    if printf '%s\n' "$rhs" | grep -qE 'redact(_id)?!\('; then
        continue
    fi

    hits=$((hits + 1))
    echo "ERROR: $file:$lineno — sensitive field emitted without redact!() / redact_id!()"
    echo "    $content"
    failed=1
done < "$tmp"

if [ "$failed" -ne 0 ]; then
    cat >&2 <<EOF

Found $hits unredacted sensitive-field site(s) in tracing macros.

Wrap the value in observability::redact!(...) (opaque "<redacted>") or
observability::redact_id!(...) (correlatable hash). Example:

    tracing::info!(
        api_key = %observability::redact!(&api_key),
        user.hash = %observability::redact_id!(&user_hash),
        "request received",
    );

For an intentional exception (e.g. a test feeding the FMT layer a raw value
to verify deny-list redaction), add an inline comment containing
\`lint:redaction-ok <reason>\` on the same line.

See docs/observability/redaction-lint.md for guidance.
EOF
    exit 1
fi

echo "lint-redaction: ok — no unredacted sensitive-field tracing call sites in crates/*/src/."
