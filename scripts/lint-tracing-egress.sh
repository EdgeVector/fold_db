#!/usr/bin/env bash
# lint-tracing-egress.sh
#
# Enforce that every `reqwest::Client` / `reqwest::ClientBuilder` construction
# inside `crates/*/src/` carries a `// trace-egress: <class>` classifier comment
# within the 3 lines immediately preceding it.
#
# Classes (Phase 2 / observability propagation):
#   propagate — call goes to one of our own services; .send() should be wrapped with
#               `observability::propagation::inject_w3c`.
#   loopback  — same as propagate but for internal localhost loopback / test fakes.
#   skip-s3   — presigned-URL S3 calls; injecting headers would corrupt the signature.
#   skip-3p   — third-party (Stripe, OpenRouter, etc.) that does not honour traceparent.
#
# Tests under `crates/*/tests/` (top-level integration tests) are out of scope —
# classification matters at runtime, not in test scaffolding outside `src/`.
#
# Usage:
#   bash scripts/lint-tracing-egress.sh            # warn-only: print errors, exit 0
#   bash scripts/lint-tracing-egress.sh --strict   # fail on any unclassified site (CI)
#
# Default mode is warn-only so a half-finished local edit doesn't block iteration.
# CI runs with `--strict` so unclassified constructions cannot land.

set -euo pipefail

strict=0
for arg in "$@"; do
    case "$arg" in
        --strict)
            strict=1
            ;;
        -h|--help)
            sed -n '2,22p' "$0"
            exit 0
            ;;
        *)
            echo "lint-tracing-egress: unknown argument: $arg" >&2
            echo "usage: bash scripts/lint-tracing-egress.sh [--strict]" >&2
            exit 2
            ;;
    esac
done

PATTERN='reqwest::(Client|ClientBuilder)::(new|default|builder)\(\)'

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
cd "$REPO_ROOT"

if ! compgen -G "crates/*/src" > /dev/null; then
    echo "lint-tracing-egress: no crates/*/src directories found at $REPO_ROOT" >&2
    exit 1
fi

violations=0
total=0

while IFS= read -r match; do
    [[ -z "$match" ]] && continue
    total=$((total + 1))

    file="${match%%:*}"
    rest="${match#*:}"
    lineno="${rest%%:*}"

    start=$((lineno - 3))
    [[ $start -lt 1 ]] && start=1
    end=$((lineno - 1))

    preceding=""
    if [[ $end -ge 1 ]]; then
        preceding=$(sed -n "${start},${end}p" "$file")
    fi

    if ! printf '%s\n' "$preceding" | grep -q '// trace-egress:'; then
        if [[ $strict -eq 1 ]]; then
            echo "ERROR: $file:$lineno — reqwest::Client construction without // trace-egress: classifier in preceding 3 lines"
        else
            echo "WARN: $file:$lineno — reqwest::Client construction without // trace-egress: classifier in preceding 3 lines"
        fi
        violations=$((violations + 1))
    fi
done < <(grep -rnE "$PATTERN" crates/*/src 2>/dev/null || true)

if [[ $violations -ne 0 ]]; then
    cat >&2 <<'EOF'

Add a comment like '// trace-egress: <propagate|loopback|skip-s3|skip-3p>' on
one of the 3 lines immediately preceding each reqwest::Client construction.
See docs/observability/tracing-egress-lint.md for guidance.
EOF
    if [[ $strict -eq 1 ]]; then
        exit 1
    fi
    echo "lint-tracing-egress: warn — $violations of $total reqwest construction sites in crates/*/src/ are unclassified (run with --strict to fail)."
    exit 0
fi

echo "lint-tracing-egress: ok — all $total reqwest construction sites in crates/*/src/ are classified."
