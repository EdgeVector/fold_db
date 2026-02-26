# Topology Redesign: Remove It

## Current State

`JsonTopology` is a recursive enum attached to every schema field. It carries three responsibilities:

1. **Structure validation** — Rejects type mismatches (number where string expected) during mutation execution.
2. **Schema identity** — SHA256 of the topology tree (classifications stripped) becomes the schema name for deduplication.
3. **Classifications** — `["word", "date", "name:person"]` arrays embedded in Primitive nodes drive keyword indexing.

## Why Remove It

### Validation doesn't earn its keep

The data comes from AI-parsed JSON. The AI determines types from actual values, then topology validates the AI's output against the AI's own declaration. When it catches a mismatch, it's the AI being inconsistent with itself — not real data corruption. Meanwhile:

- `Reference` nodes accept any value (`Ok(())`), so the most complex case has zero validation.
- `Any` nodes (the null-inference fallback) accept everything, creating permanent holes.
- Legitimate values like `"42"` in a Number field are hard-rejected with no coercion.

The storage layer (Sled/DynamoDB) stores serialized atoms as `Vec<u8>` — it doesn't care about types. Queries resolve atom content as `serde_json::Value` — they handle mixed types fine.

### Schema identity already uses field names

The schema service's similarity check (`server.rs:464-531`) computes Jaccard index on **field names**, not topology hashes. The topology hash is only used at ingestion time to generate the schema name. These are two disconnected identity mechanisms. Field names are sufficient — and already the authoritative one.

### Classifications are independent of structure

Classifications are semantic annotations for indexing. They're explicitly stripped before topology hashing because they don't affect structure. They're patched in after the fact with manual tree walks. They have no relationship to the recursive type system — they just need to be a flat map from field name to tags.

### The AI prompt is the bottleneck

The AI must return a deeply nested `field_topologies` JSON that mirrors the data shape. This is the most error-prone part of ingestion. When it fails, the fallback inference produces `Any` nodes for nulls — permanent validation gaps. Eliminating topology from the AI response removes the single largest source of ingestion errors.

## What Replaces It

Two flat maps on the schema, plus a refs map for composition:

### 1. Classifications — `HashMap<String, Vec<String>>`

Flat map from field name to semantic tags. Drives keyword indexing.

```rust
// On the schema
pub field_classifications: HashMap<String, Vec<String>>,
```

Example:
```json
{
  "created_at": ["date"],
  "author": ["name:person", "word"],
  "content": ["word"],
  "amount": ["number"]
}
```

- AI returns this as a simple map — no nesting, no recursion.
- Defaults applied with a one-liner: if field not in map, assign `["word"]` for strings, `["number"]` for numbers (inferred from the actual JSON value at ingestion time).
- Indexing reads directly from this map instead of extracting classifications from recursive topology nodes.

### 2. Schema Identity — sorted field names hash

```rust
// On the schema
pub identity_hash: String,  // SHA256 of sorted field names
```

Computed as:
```
SHA256(sorted(field_names).join(","))
```

This replaces `topology_hash` as the schema name. Two schemas with the same fields get the same hash and deduplicate. The schema service's Jaccard similarity check continues to work as-is for fuzzy matching.

Schema names become this hash (same as today's behavior, just computed from field names instead of recursive topology). If we later want human-readable names, the hash stays as dedup metadata and a separate `display_name` field is added.

### 3. Refs — `HashMap<String, String>`

Flat map from field name to child schema name. Replaces `TopologyNode::Reference`.

```rust
// On the schema
pub ref_fields: HashMap<String, String>,
```

Example:
```json
{
  "passenger": "a1b2c3d4...",
  "outbound_flight": "e5f6g7h8..."
}
```

- The decomposer populates this when extracting child groups (currently creates `TopologyNode::Reference` nodes).
- Query rehydration reads this map directly instead of scanning topology nodes at runtime.
- Ref fields accept any JSON value during mutation — same as current `Reference` behavior, but without pretending there's a type system validating it.

### 4. No Validation (or advisory-only)

Drop `validate_field_value()` from the mutation execution path. The data is AI-generated and already structurally correct from the source JSON.

If a safety net is wanted later, add advisory type tracking:

```rust
// Optional, not blocking
pub field_types: HashMap<String, String>,  // "String", "Number", "Boolean" — inferred from data
```

This would log warnings on type mismatches but never reject mutations. Inferred at ingestion time from the actual JSON values, not declared by the AI.

## What Gets Removed

| Current | Replacement |
|---------|-------------|
| `JsonTopology` recursive enum | Removed entirely |
| `TopologyNode` (Primitive, Object, Array, Reference, Any) | Removed |
| `field_topologies: HashMap<String, JsonTopology>` on schema | Removed |
| `field_topology_hashes: HashMap<String, String>` on schema | Removed |
| `topology_hash` as schema name | `identity_hash` from sorted field names |
| `compute_schema_topology_hash()` | `compute_identity_hash()` — one-liner |
| `validate_field_value()` during mutation execution | Removed (or advisory-only) |
| `strip_classifications()` in hash computation | Unnecessary |
| `infer_topologies_from_data()` recursive inference | Unnecessary |
| `TopologyNode::Reference { schema_name }` | `ref_fields: HashMap<String, String>` |
| Complex AI prompt for `field_topologies` | Simple flat map for `field_classifications` |
| Default classification tree-walk (`mod.rs:677-722`) | Flat map iteration |

## What Gets Preserved

| Function | How |
|----------|-----|
| Schema deduplication | `identity_hash` from sorted field names (same Jaccard check at schema service) |
| Keyword indexing | `field_classifications` flat map (same classification tags) |
| Decomposition / composition | `ref_fields` map (same decomposer logic, simpler storage) |
| Query rehydration | Direct lookup in `ref_fields` (replaces topology node scanning) |
| Default classifications | Flat map iteration instead of tree walk |

## AI Prompt Change

Current — AI must return recursive topology:
```json
{
  "field_topologies": {
    "author": {
      "root": {
        "type": "Primitive",
        "value": "String",
        "classifications": ["name:person", "word"]
      }
    },
    "tags": {
      "root": {
        "type": "Array",
        "value": {
          "type": "Primitive",
          "value": "String",
          "classifications": ["hashtag", "word"]
        }
      }
    }
  }
}
```

Proposed — AI returns a flat classification map:
```json
{
  "field_classifications": {
    "author": ["name:person", "word"],
    "tags": ["hashtag", "word"],
    "created_at": ["date"],
    "content": ["word"]
  }
}
```

The AI no longer needs to describe data structure at all. It just tags semantic meaning.

## Schema Example

Before (with topology):
```
Schema: 7eefdad0e595905754d28590829e38903ed1af4179ddc8c72e343ed21025c475
  fields: [author, content, created_at, tags]
  field_topologies:
    author:     Primitive(String, ["name:person", "word"])
    content:    Primitive(String, ["word"])
    created_at: Primitive(String, ["date"])
    tags:       Array(Primitive(String, ["hashtag", "word"]))
  topology_hash: 7eefdad0...
```

After (no topology):
```
Schema: a3b1c4d2...  (SHA256 of "author,content,created_at,tags")
  fields: [author, content, created_at, tags]
  field_classifications:
    author:     ["name:person", "word"]
    content:    ["word"]
    created_at: ["date"]
    tags:       ["hashtag", "word"]
  ref_fields: {}
```

Same dedup behavior. Same indexing. No recursive type system.

## Composition Example

```
Schema: FlightBooking (identity_hash from sorted field names)
  fields: [booking_id, passenger, outbound, return_flight]
  field_classifications:
    booking_id: ["word"]
  ref_fields:
    passenger:     "Passenger_schema_hash"
    outbound:      "FlightLeg_schema_hash"
    return_flight: "FlightLeg_schema_hash"

Schema: Passenger
  fields: [name, email]
  field_classifications:
    name:  ["name:person", "word"]
    email: ["email"]
  ref_fields: {}

Schema: FlightLeg
  fields: [departure, arrival, date]
  field_classifications:
    departure: ["name:place", "word"]
    arrival:   ["name:place", "word"]
    date:      ["date"]
  ref_fields: {}
```

Query rehydration: read `ref_fields`, fetch child records by key, recurse. Same as today but without scanning topology nodes.

## Migration Path

1. **Add `field_classifications` and `ref_fields`** as new fields on schema. Populate from existing topology data during schema load (extract classifications from Primitive nodes, extract schema_name from Reference nodes).
2. **Switch indexing** to read from `field_classifications`.
3. **Switch query rehydration** to read from `ref_fields`.
4. **Switch identity** to `identity_hash` computed from sorted field names.
5. **Remove `validate_field_value()`** call from mutation_manager (or make it advisory).
6. **Update AI prompt** to return `field_classifications` instead of `field_topologies`.
7. **Update decomposer** to populate `ref_fields` instead of creating `TopologyNode::Reference`.
8. **Remove** `field_topologies`, `field_topology_hashes`, `topology_hash`, and all of `topology.rs`.

Steps 1-5 are backward-compatible — old schemas with topology data keep working. Steps 6-8 are the breaking change.

## Open Questions

- **Backward compatibility of stored schemas.** Existing schemas in Sled/DynamoDB have `field_topologies`. Step 1 handles loading old schemas by extracting classifications and refs from topology. But should old schemas be migrated in-place, or converted on read?
- **Advisory type tracking.** Is a `field_types` map worth adding for debugging/logging, or is it pure overhead? The data is available (infer from the JSON value at ingestion time) but may never be read.
- **Identity stability.** If field names change (field added/removed), the identity hash changes. This is the same behavior as today with topology hash. Is this acceptable, or should identity be versioned?
