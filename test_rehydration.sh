#!/usr/bin/env bash
set -euo pipefail

# --- Config ---
BASE="http://localhost:9001/api"
PASS=0
FAIL=0
TOTAL=0

green() { printf "\033[32m%s\033[0m\n" "$1"; }
red()   { printf "\033[31m%s\033[0m\n" "$1"; }
bold()  { printf "\033[1m%s\033[0m\n" "$1"; }

assert_contains() {
  TOTAL=$((TOTAL+1))
  local label="$1" needle="$2" haystack="$3"
  if echo "$haystack" | grep -q "$needle"; then
    green "  PASS: $label"
    PASS=$((PASS+1))
  else
    red "  FAIL: $label"
    echo "    expected to contain: $needle"
    echo "    actual: $(echo "$haystack" | head -c 200)"
    FAIL=$((FAIL+1))
  fi
}

api() {
  curl -sf -H "X-User-Hash:test_user" -H "Content-Type:application/json" "$@"
}

# ========================================
# 0. Wait for server
# ========================================
bold "=== Waiting for server on :9001 ==="
for i in $(seq 1 30); do
  if api "$BASE/system/status" >/dev/null 2>&1; then
    green "Server is up."
    break
  fi
  [ "$i" -eq 30 ] && { red "Server not responding. Start with: ./run.sh --local --dev"; exit 1; }
  sleep 1
done

# ========================================
# 1. Reset database
# ========================================
bold ""
bold "=== Step 1: Reset database ==="
RESET=$(api -X POST -d '{"confirm":true}' "$BASE/system/reset-database")
echo "  $RESET"
RESET_JOB=$(echo "$RESET" | python3 -c "import json,sys; print(json.load(sys.stdin).get('job_id',''))" 2>/dev/null)
echo "  Reset job: $RESET_JOB — waiting for completion..."
for _i in $(seq 1 60); do
  PROG=$(api "$BASE/ingestion/progress/$RESET_JOB" 2>/dev/null || echo "{}")
  IS_DONE=$(echo "$PROG" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print('done' if d.get('is_complete') else 'running')
" 2>/dev/null)
  [ "$IS_DONE" = "done" ] && { green "  Reset completed."; break; }
  [ "$_i" -eq 60 ] && { red "  Reset timed out."; exit 1; }
  sleep 1
done
# Give the node manager time to recreate a fresh node
sleep 2

# ========================================
# 2. Ingest nested JSON
# ========================================
bold ""
bold "=== Step 2: Ingest nested data ==="

read -r -d '' PAYLOAD << 'JSONEOF' || true
{
  "data": [
    {
      "name": "Alice Johnson",
      "email": "alice@example.com",
      "posts": [
        {
          "title": "My First Post",
          "body": "Hello world, this is my first blog post!",
          "comments": [
            {"author": "Bob", "text": "Great post Alice!"},
            {"author": "Charlie", "text": "Welcome to the blog!"}
          ]
        },
        {
          "title": "Second Post",
          "body": "Another day, another blog post.",
          "comments": [
            {"author": "Dave", "text": "Keep it up!"}
          ]
        }
      ]
    },
    {
      "name": "Bob Smith",
      "email": "bob@example.com",
      "posts": [
        {
          "title": "Bobs Blog",
          "body": "Hi everyone, Bob here.",
          "comments": []
        }
      ]
    }
  ],
  "auto_execute": true,
  "trust_distance": 0,
  "pub_key": "test_user"
}
JSONEOF

echo "  Ingesting nested user/posts/comments data..."
INGEST_RESULT=$(api -X POST -d "$PAYLOAD" "$BASE/ingestion/process" 2>&1) || INGEST_RESULT="FAILED: $?"
echo "  Response: $(echo "$INGEST_RESULT" | python3 -m json.tool 2>/dev/null | head -20 || echo "$INGEST_RESULT" | head -10)"

PROGRESS_ID=$(echo "$INGEST_RESULT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('progress_id',''))" 2>/dev/null)
echo "  Progress ID: $PROGRESS_ID"
echo "  Waiting for ingestion to complete..."
for _i in $(seq 1 120); do
  PROG=$(api "$BASE/ingestion/progress" 2>/dev/null || echo "{}")
  IS_DONE=$(echo "$PROG" | python3 -c "
import json, sys
d = json.load(sys.stdin)
for p in d.get('progress', []):
    if p.get('id') == '$PROGRESS_ID':
        print('done' if p.get('is_complete') else 'running')
        sys.exit()
print('unknown')
" 2>/dev/null)
  if [ "$IS_DONE" = "done" ]; then
    IS_FAILED=$(echo "$PROG" | python3 -c "
import json, sys
d = json.load(sys.stdin)
for p in d.get('progress', []):
    if p.get('id') == '$PROGRESS_ID':
        print('true' if p.get('is_failed') else 'false')
        sys.exit()
print('false')
" 2>/dev/null)
    if [ "$IS_FAILED" = "true" ]; then
      red "  Ingestion FAILED!"
      echo "$PROG" | python3 -m json.tool 2>/dev/null | head -20
      exit 1
    fi
    green "  Ingestion completed."
    break
  fi
  [ "$_i" -eq 120 ] && { red "  Timed out waiting for ingestion (240s)."; exit 1; }
  sleep 2
done

# ========================================
# 3. List schemas
# ========================================
bold ""
bold "=== Step 3: List schemas ==="
SCHEMAS_RAW=$(api "$BASE/schemas")
echo "$SCHEMAS_RAW" | python3 -m json.tool 2>/dev/null | head -40 || echo "$SCHEMAS_RAW"

# Extract schema names and find parent (with Reference topologies)
IFS=$'\t' read -r SCHEMA_NAMES PARENT_SCHEMA PARENT_FIELDS << PYEOF
$(echo "$SCHEMAS_RAW" | python3 -c "
import json, sys
data = json.load(sys.stdin)
schemas_list = data.get('schemas', [])
names = []
parent = ''
parent_fields = '[]'
for entry in schemas_list:
    s = entry.get('schema', entry) if isinstance(entry, dict) else entry
    name = s.get('name', '')
    if not name:
        continue
    names.append(name)
    topos = s.get('field_topologies', {})
    for fname, topo in topos.items():
        root = topo.get('root', topo)
        if root.get('type') == 'Reference' and not parent:
            parent = name
            parent_fields = json.dumps(s.get('fields', []))
all_names = ' '.join(names) if names else ''
print(f'{all_names}\t{parent}\t{parent_fields}')
" 2>/dev/null)
PYEOF

echo ""
echo "  All schemas: $SCHEMA_NAMES"
echo "  Parent schema (has Reference fields): $PARENT_SCHEMA"
echo "  Parent fields: $PARENT_FIELDS"

if [ -z "$PARENT_SCHEMA" ]; then
  red "No parent schema with Reference topology found. Ingestion may not have decomposed the data."
  echo ""
  bold "Trying to query all schemas to see what data exists..."
  for S in $SCHEMA_NAMES; do
    SDETAIL=$(api "$BASE/schemas/$S" 2>/dev/null || echo "{}")
    SF=$(echo "$SDETAIL" | python3 -c "
import json, sys
d = json.load(sys.stdin)
s = d.get('schema', d)
print(json.dumps(s.get('fields', [])))
" 2>/dev/null || echo "[]")
    echo ""
    bold "  --- $S (fields: $SF) ---"
    QRESULT=$(api -X POST -d "{\"schema_name\": \"$S\", \"fields\": $SF}" "$BASE/query" 2>/dev/null || echo "{}")
    echo "$QRESULT" | python3 -m json.tool 2>/dev/null | head -30 || echo "  $QRESULT" | head -30
  done

  bold ""
  bold "========================================="
  red "CANNOT TEST REHYDRATION: no Reference fields found in schemas."
  red "The AI ingestion may not have decomposed the nested data."
  bold "========================================="
  exit 1
fi

# ========================================
# 4. Query parent WITHOUT rehydration
# ========================================
bold ""
bold "=== Step 4: Query parent WITHOUT rehydrate_depth ==="
RAW_QUERY="{\"schema_name\": \"$PARENT_SCHEMA\", \"fields\": $PARENT_FIELDS}"
RAW_RESULT=$(api -X POST -d "$RAW_QUERY" "$BASE/query" 2>/dev/null || echo "{}")
echo "$RAW_RESULT" | python3 -m json.tool 2>/dev/null | head -60 || echo "$RAW_RESULT" | head -60

# ========================================
# 5. Query parent WITH rehydrate_depth=1
# ========================================
bold ""
bold "=== Step 5: Query parent WITH rehydrate_depth=1 ==="
D1_QUERY="{\"schema_name\": \"$PARENT_SCHEMA\", \"fields\": $PARENT_FIELDS, \"rehydrate_depth\": 1}"
D1_RESULT=$(api -X POST -d "$D1_QUERY" "$BASE/query" 2>/dev/null || echo "{}")
echo "$D1_RESULT" | python3 -m json.tool 2>/dev/null | head -80 || echo "$D1_RESULT" | head -80

# ========================================
# 6. Query parent WITH rehydrate_depth=2
# ========================================
bold ""
bold "=== Step 6: Query parent WITH rehydrate_depth=2 ==="
D2_QUERY="{\"schema_name\": \"$PARENT_SCHEMA\", \"fields\": $PARENT_FIELDS, \"rehydrate_depth\": 2}"
D2_RESULT=$(api -X POST -d "$D2_QUERY" "$BASE/query" 2>/dev/null || echo "{}")
echo "$D2_RESULT" | python3 -m json.tool 2>/dev/null | head -120 || echo "$D2_RESULT" | head -120

# ========================================
# 7. Assertions
# ========================================
bold ""
bold "=== Step 7: Assertions ==="

# Count raw schema references vs hydrated "fields" keys
RAW_SCHEMA_COUNT=$(echo "$RAW_RESULT" | python3 -c "
import json, sys
data = json.load(sys.stdin)
records = data.get('results', data.get('records', []))
if isinstance(data, list): records = data
count = 0
def count_refs(v):
    global count
    if isinstance(v, dict):
        if 'schema' in v and 'key' in v and 'fields' not in v:
            count += 1
        for val in v.values():
            count_refs(val)
    elif isinstance(v, list):
        for item in v:
            count_refs(item)
count_refs(records)
print(count)
" 2>/dev/null || echo "0")

D1_SCHEMA_COUNT=$(echo "$D1_RESULT" | python3 -c "
import json, sys
data = json.load(sys.stdin)
records = data.get('results', data.get('records', []))
if isinstance(data, list): records = data
count = 0
def count_refs(v):
    global count
    if isinstance(v, dict):
        if 'schema' in v and 'key' in v and 'fields' not in v:
            count += 1
        for val in v.values():
            count_refs(val)
    elif isinstance(v, list):
        for item in v:
            count_refs(item)
count_refs(records)
print(count)
" 2>/dev/null || echo "0")

D1_HYDRATED=$(echo "$D1_RESULT" | python3 -c "
import json, sys
data = json.load(sys.stdin)
records = data.get('results', data.get('records', []))
if isinstance(data, list): records = data
count = 0
def count_hydrated(v, depth=0):
    global count
    if isinstance(v, dict):
        if 'fields' in v and 'key' in v and depth > 0:
            count += 1
        for val in v.values():
            count_hydrated(val, depth+1)
    elif isinstance(v, list):
        for item in v:
            count_hydrated(item, depth)
count_hydrated(records)
print(count)
" 2>/dev/null || echo "0")

echo "  Raw query: $RAW_SCHEMA_COUNT unresolved references"
echo "  Depth=1:   $D1_SCHEMA_COUNT unresolved references, $D1_HYDRATED hydrated records"

TOTAL=$((TOTAL+1))
if [ "$RAW_SCHEMA_COUNT" -gt 0 ]; then
  green "  PASS: Raw query contains unresolved references ($RAW_SCHEMA_COUNT)"
  PASS=$((PASS+1))
else
  red "  FAIL: Raw query should contain unresolved references (got $RAW_SCHEMA_COUNT)"
  FAIL=$((FAIL+1))
fi

TOTAL=$((TOTAL+1))
if [ "$D1_HYDRATED" -gt 0 ]; then
  green "  PASS: Depth=1 query resolved references into hydrated records ($D1_HYDRATED)"
  PASS=$((PASS+1))
else
  red "  FAIL: Depth=1 should have hydrated records (got $D1_HYDRATED)"
  FAIL=$((FAIL+1))
fi

TOTAL=$((TOTAL+1))
if [ "$D1_SCHEMA_COUNT" -lt "$RAW_SCHEMA_COUNT" ]; then
  green "  PASS: Depth=1 has fewer unresolved refs than raw ($D1_SCHEMA_COUNT < $RAW_SCHEMA_COUNT)"
  PASS=$((PASS+1))
else
  red "  FAIL: Depth=1 should have fewer unresolved refs (raw=$RAW_SCHEMA_COUNT, d1=$D1_SCHEMA_COUNT)"
  FAIL=$((FAIL+1))
fi

# Depth=2 should resolve even deeper nested references
D2_HYDRATED=$(echo "$D2_RESULT" | python3 -c "
import json, sys
data = json.load(sys.stdin)
records = data.get('results', data.get('records', []))
if isinstance(data, list): records = data
count = 0
def count_hydrated(v, depth=0):
    global count
    if isinstance(v, dict):
        if 'fields' in v and 'key' in v and depth > 0:
            count += 1
        for val in v.values():
            count_hydrated(val, depth+1)
    elif isinstance(v, list):
        for item in v:
            count_hydrated(item, depth)
count_hydrated(records)
print(count)
" 2>/dev/null || echo "0")

TOTAL=$((TOTAL+1))
if [ "$D2_HYDRATED" -ge "$D1_HYDRATED" ]; then
  green "  PASS: Depth=2 hydrated >= depth=1 hydrated ($D2_HYDRATED >= $D1_HYDRATED)"
  PASS=$((PASS+1))
else
  red "  FAIL: Depth=2 should resolve at least as much as depth=1 (d2=$D2_HYDRATED, d1=$D1_HYDRATED)"
  FAIL=$((FAIL+1))
fi

# ========================================
# Summary
# ========================================
echo ""
bold "========================================="
if [ "$FAIL" -eq 0 ]; then
  green "ALL $TOTAL TESTS PASSED ($PASS/$TOTAL)"
else
  red "$FAIL FAILED, $PASS passed out of $TOTAL"
fi
bold "========================================="
exit "$FAIL"
