# Schema Indexing Iterator Stack and Fan-out Model

## Overview

The schema indexing system handles fan-out using a **stack of iterators (scopes)**. Each field expression is evaluated within this stacked scope, with the field containing the deepest active iterator determining the number of output rows. Other fields are either broadcast, aligned 1:1, or reduced relative to that deepest scope.

## Mental Model: Iterator Stack

When you write chains like:
- `blogpost.map()` → **I0**
- `blogpost.map().tags.split_array().map()` → **I1** 
- `blogpost.map().tags.split_array().map().split_by_word().map()` → **I2**

You create an iterator stack `[I0, I1, I2]`. At emission time, the system operates at a particular depth:
- **Deepest depth in any field = D**. That depth sets the number of emitted rows.
- For each row at depth D, every field is evaluated using specific alignment rules.

## Alignment Rules

For a given field F:

### 1. 1:1 Aligned (uses D)
If F reaches the deepest iterator depth D, it contributes a distinct value per emitted row.

### 2. Broadcast (invariant w.r.t D)
If F references only outer iterators (shallower depth), its value is duplicated across all rows at depth D in the same outer context.

### 3. Reduced (optional)
If F would fan out deeper than D or intentionally collapses inner collections, you must apply a reducer (e.g., `first()`, `join(',')`, `count()`) to bring it back to ≤ D.
- If reducers aren't supported, it's an error.

### 4. Illegal / Ambiguous
If two fields try to fan out at different, incomparable branches (different iterator stacks that don't share a prefix), the system rejects with a cartesian-join error. All fan-outs must be along the same iterator chain (shared prefix).

## What Sets D (Emission Depth)?

The `hash_field`'s deepest `.map()` usually sets D, because the index must produce one key per hash value. Other fields can be:
- **Aligned** at the same depth (1:1)
- **Broadcast** from shallower depths
- **Reduced** to a shallower value

## Concrete Examples (Chain Style)

### A) Word Index with Date Range (Broadcast from I0 to I2)

```json
{
  "name": "blogs_by_word",
  "schema_type": "Range",
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().content.split_by_word().map()",    // I1 ← deepest
      "range_field": "blogpost.map().publish_date",                   // I0 → broadcast
      "atom_uuid": "blogpost.map().$atom_uuid"                        // I0 → broadcast
    }
  }
}
```

**Analysis:**
- D = I1 (from `hash_field` word iteration)
- `hash_field` uses word iteration → 1:1 aligned at I1
- `range_field` uses publish_date (I0) → duplicated for all words in that post
- `atom_uuid` uses post UUID (I0) → duplicated for all words in that post

### B) Author Index (Single Depth)

```json
{
  "name": "blogs_by_author",
  "schema_type": "Range",
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().author",           // I0 (single depth)
      "range_field": "blogpost.map().publish_date",   // I0 (same depth)
      "atom_uuid": "blogpost.map().$atom_uuid"        // I0 (same depth)
    }
  }
}
```

**Analysis:**
- D = I0 (all fields at same depth)
- All fields operate at I0 → 1:1 aligned
- One index entry per blogpost record

### C) Tag Index with Broadcasting

```json
{
  "name": "blogs_by_tag",
  "schema_type": "Range",
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().tags.split_array().map()",  // I1 (deepest)
      "range_field": "blogpost.map().publish_date",            // I0 → broadcast
      "atom_uuid": "blogpost.map().$atom_uuid"                 // I0 → broadcast
    }
  }
}
```

**Analysis:**
- D = I1 from `hash_field` tag iteration
- `range_field` and `atom_uuid` broadcast from I0 across all tags

### D) Illegal: Incomparable Fan-outs (Cartesian)

```json
{
  "name": "illegal_example",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().tags.split_array().map()",      // fans out I1 via tags
      "range_field": "blogpost.map().comments.map().content"        // fans out J1 via comments (different branch)
    }
  }
}
```

**Error:** Different branches (`tags` vs `comments`) at the same structural level would imply a cartesian product. Rejected with `AmbiguousFanoutDifferentBranches`.

## Chain Style Analysis

The chain style automatically infers iterator depth:
- Deepest `.map()` across all fields sets D
- Fields that don't reach that `.map()` are broadcast
- Fields that try to go deeper than D must be reduced or error

### Complex Multi-Field Example:

```json
{
  "name": "complex_word_index",
  "schema_type": "Range",
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().content.split_by_word().map()",           // I1 (deepest)
      "range_field": "blogpost.map().publish_date.getFullYear().toString()",  // I0 → broadcast
      "atom_uuid": "blogpost.map().$atom_uuid"                               // I0 → broadcast
    }
  }
}
```

**Analysis:**
- D = I1 from `hash_field` (Schema→Map(I0)→split→Map(I1))
- `range_field` operates at I0 with additional processing (Schema→Map(I0))
- `atom_uuid` operates at I0 (Schema→Map(I0))
- Both shallow fields broadcast across all words

## Validation Checklist

To keep the system consistent:

### ✅ Valid Conditions
- All field chains share a common prefix (same `schema.map()` and any nested maps up to min depth)
- The maximum depth among fields is well defined → that's D
- Any field deeper than D must apply a reducer (or system rejects)

### ❌ Invalid Conditions
- Reject incomparable depths (fan-out on different branches at the same level)
- Reject `range_field`/`atom_uuid` ending with `.map()` unless your index format supports multi-range entries explicitly (most don't)

## Field Alignment Behavior Table

| Field Usage vs D | Behavior | Example |
|------------------|----------|----------|
| References iterator at D | 1:1 aligned | `hash_field` at I1 |
| References only outer scopes (< D) | Broadcast | `range_field` at I0, D=I1 |
| Tries to go deeper than D | Reduce or error | `.map()` beyond D → reduce or bump D |
| Fan-out on different branch | Error (cartesian) | `tags.map()` vs `comments.map()` |

## Execution Examples

### Simple Word Index

**Input Data:**
```javascript
blogpost = [
  { id: "post1", content: "hello world", publish_date: "2024-01-01" },
  { id: "post2", content: "foo bar", publish_date: "2024-01-02" }
]
```

**Expression:**
```json
{
  "hash_field": "blogpost.map().content.split_by_word().map()",  // I1
  "range_field": "blogpost.map().publish_date",                // I0 → broadcast
  "atom_uuid": "blogpost.map().$atom_uuid"                     // I0 → broadcast
}
```

**Output (D=I1):**
```javascript
[
  { hash: "hello", range: "2024-01-01", uuid: "post1" },
  { hash: "world", range: "2024-01-01", uuid: "post1" },
  { hash: "foo",   range: "2024-01-02", uuid: "post2" },
  { hash: "bar",   range: "2024-01-02", uuid: "post2" }
]
```
```

### Multi-Level Chain Example

**Input Data:**
```javascript
blogpost = [
  { id: "post1", tags: ["tech ai", "machine learning"], publish_date: "2024-01-01" }
]
```

**Expression:**
```json
{
  "name": "nested_word_tag_index",
  "schema_type": "Range", 
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().tags.split_array().map().split_by_word().map()",  // I2 (deepest)
      "range_field": "blogpost.map().publish_date",                                  // I0 → broadcast
      "atom_uuid": "blogpost.map().$atom_uuid"                                       // I0 → broadcast
    }
  }
}
```

**Output (D=I2):**
```javascript
[
  { hash: "tech", range: "2024-01-01", uuid: "post1" },
  { hash: "ai", range: "2024-01-01", uuid: "post1" },
  { hash: "machine", range: "2024-01-01", uuid: "post1" },
  { hash: "learning", range: "2024-01-01", uuid: "post1" }
]
```

### Tag-Word Index with Error

**Input Data:**
```javascript
blogpost = [
  { id: "post1", tags: ["tech", "ai"], content: "hello world" }
]
```

**Expression:**
```json
{
  "hash_field": "blogpost.map().content.split_by_word().map()",  // I1
  "range_field": "blogpost.map().tags.split_array().map()",     // I1 → ERROR: different depth
}
```

**Error:** `hash_field` fans out to I1 (words), `range_field` fans out to I1 (tags). Different fan-out depths without proper alignment.

**Corrected Version:**
```json
{
  "hash_field": "blogpost.map().tags.split_array().map()",       // I1
  "range_field": "blogpost.map().publish_date",                // I0 → broadcast
  "atom_uuid": "blogpost.map().$atom_uuid"                     // I0 → broadcast
}
```

## Iterator Depth Analysis

### Standard Chain Syntax
```json
{
  "name": "standard_word_index",
  "schema_type": "Range",
  "range_key": "composite_key", 
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().content.split_by_word().map()",  // I1 (sets D)
      "range_field": "blogpost.map().publish_date",                 // I0 (broadcast)
      "atom_uuid": "blogpost.map().$atom_uuid"                      // I0 (broadcast)
    }
  }
}
```

**Iterator Stack Analysis:**
- `blogpost.map()` → I0
- `content.split_by_word()` → processing step (no iterator)
- `.map()` → I1 (iterate over words)
- Final depth D = I1

### Complex Nested Chain
```json
{
  "hash_field": "blogpost.map().tags.split_array().map().normalize_text().split_by_word().map()"
}
```

**Iterator Stack Analysis:**
- `blogpost.map()` → I0
- `tags.split_array()` → processing step
- `.map()` → I1 (iterate over tags)
- `normalize_text().split_by_word()` → processing steps
- `.map()` → I2 (iterate over words)
- Final depth D = I2

## Reducer Functions (Future Extension)

To handle cases where fields would exceed D, reducers can collapse collections:

### Available Reducers
- `first()` - Take first element
- `last()` - Take last element  
- `count()` - Count elements
- `join(separator)` - Join array into string
- `sum()` - Sum numeric values
- `max()` / `min()` - Extrema

### Example with Reducer
```json
{
  "name": "word_index_with_tag_summary",
  "schema_type": "Range",
  "range_key": "composite_key",
  "fields": {
    "composite_key": {
      "field_type": "HashRange",
      "hash_field": "blogpost.map().content.split_by_word().map()",     // I1 (sets D)
      "range_field": "blogpost.map().tags.split_array().join(',')",    // I1 → I0 (reduced)
      "atom_uuid": "blogpost.map().$atom_uuid"                        // I0
    }
  }
}
```

The reducer `join(',')` collapses the tag array back to I0, making it compatible with D=I1.

## Implementation Notes

### Parser Requirements
1. **Stack Tracking**: Parser must track iterator depth for each field expression
2. **Depth Analysis**: Determine maximum depth D across all fields
3. **Alignment Validation**: Ensure all fields are properly aligned relative to D
4. **Branch Detection**: Identify and reject incomparable fan-out branches

### Runtime Execution
1. **Iterator Instantiation**: Create nested iterators based on expression depth
2. **Field Evaluation**: Evaluate each field at appropriate scope depth
3. **Broadcasting**: Duplicate values from shallow scopes across deeper iterations
4. **Emission**: Generate index entries at depth D

### Error Handling
- `IncompatibleFanoutDepths`: Fields fan out to different, unaligned depths
- `CartesianFanoutError`: Fields fan out on incomparable branches
- `ReducerRequired`: Field exceeds D without appropriate reducer
- `InvalidIteratorChain`: Malformed iterator stack or scope references

### Chain Parsing Algorithm

1. **Tokenize**: Split chain on `.map()` boundaries
2. **Depth Calculation**: Count `.map()` occurrences = iterator depth
3. **Branch Analysis**: Ensure all chains share common prefix up to min depth
4. **Depth Alignment**: Verify max depth compatibility across fields
5. **Validation**: Reject incompatible depth combinations

### Performance Optimization

- **Lazy Evaluation**: Don't materialize intermediate collections
- **Streaming Processing**: Process records one at a time through iterator stack
- **Early Termination**: Stop processing on validation errors
- **Memory Management**: Efficient iterator state management for deep stacks

This iterator stack model provides precise control over how multi-level iterations interact, ensuring predictable and efficient index generation while preventing ambiguous cartesian products.