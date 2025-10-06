# Transform Functions Reference

This document provides a comprehensive guide to all available transform functions in the DataFold transformation engine.

## Overview

Transform functions are divided into two categories:
- **Iterator Functions (Mappers)**: Expand one item into multiple items
- **Reducer Functions**: Aggregate multiple items into a single result

## Iterator Functions (Mappers)

Iterator functions take a single input item and produce multiple output items, effectively expanding the data.

### `split_by_word()`

**Purpose**: Splits text content into individual words.

**Syntax**: `field.split_by_word()`

**Input**: Text string
**Output**: Array of individual words

**Example**:
```json
// Input
{
  "content": "hello world test"
}

// Transform: content.split_by_word()
// Output
[
  { "content": "hello" },
  { "content": "world" },
  { "content": "test" }
]
```

**Use Cases**:
- Word frequency analysis
- Text indexing
- Keyword extraction

### `split_array()`

**Purpose**: Splits an array into individual elements.

**Syntax**: `field.split_array()`

**Input**: Array value
**Output**: Individual array elements

**Example**:
```json
// Input
{
  "tags": ["rust", "database", "transforms"]
}

// Transform: tags.split_array()
// Output
[
  { "tags": "rust" },
  { "tags": "database" },
  { "tags": "transforms" }
]
```

**Use Cases**:
- Tag processing
- Array element analysis
- Multi-value field handling

## Reducer Functions

Reducer functions take multiple input items and produce a single aggregated result.

### `count()`

**Purpose**: Counts the number of items.

**Syntax**: `field.count()` or `field.split_by_word().count()`

**Input**: Multiple items
**Output**: String representation of count

**Example**:
```json
// Input
{
  "content": "hello world test"
}

// Transform: content.split_by_word().count()
// Output
{
  "content": "3"
}
```

**Use Cases**:
- Word counting
- Item counting
- Statistics

### `sum()`

**Purpose**: Sums numeric values across items.

**Syntax**: `field.sum()` or `field.split_array().sum()`

**Input**: Multiple items with numeric values
**Output**: String representation of sum

**Example**:
```json
// Input
{
  "scores": [10, 20, 30]
}

// Transform: scores.split_array().sum()
// Output
{
  "scores": "60"
}
```

**Use Cases**:
- Score aggregation
- Financial calculations
- Numeric statistics

### `join()`

**Purpose**: Joins multiple string values with comma separation.

**Syntax**: `field.join()` or `field.split_by_word().join()`

**Input**: Multiple items with string values
**Output**: Comma-separated string

**Example**:
```json
// Input
{
  "tags": ["rust", "database", "transforms"]
}

// Transform: tags.split_array().join()
// Output
{
  "tags": "rust, database, transforms"
}
```

**Use Cases**:
- Tag aggregation
- Text reconstruction
- List formatting

### `first()`

**Purpose**: Returns the first item from a collection.

**Syntax**: `field.first()` or `field.split_by_word().first()`

**Input**: Multiple items
**Output**: String representation of first item

**Example**:
```json
// Input
{
  "content": "hello world test"
}

// Transform: content.split_by_word().first()
// Output
{
  "content": "\"hello\""
}
```

**Use Cases**:
- Primary value selection
- Default value extraction
- Priority handling

### `last()`

**Purpose**: Returns the last item from a collection.

**Syntax**: `field.last()` or `field.split_by_word().last()`

**Input**: Multiple items
**Output**: String representation of last item

**Example**:
```json
// Input
{
  "content": "hello world test"
}

// Transform: content.split_by_word().last()
// Output
{
  "content": "\"test\""
}
```

**Use Cases**:
- Final value selection
- Latest item extraction
- Completion status

### `max()`

**Purpose**: Finds the maximum numeric value from a collection.

**Syntax**: `field.max()` or `field.split_array().max()`

**Input**: Multiple items with numeric values
**Output**: String representation of maximum value

**Example**:
```json
// Input
{
  "scores": [10, 25, 15]
}

// Transform: scores.split_array().max()
// Output
{
  "scores": "25"
}
```

**Use Cases**:
- Peak value detection
- Performance metrics
- Range analysis

### `min()`

**Purpose**: Finds the minimum numeric value from a collection.

**Syntax**: `field.min()` or `field.split_array().min()`

**Input**: Multiple items with numeric values
**Output**: String representation of minimum value

**Example**:
```json
// Input
{
  "scores": [10, 25, 15]
}

// Transform: scores.split_array().min()
// Output
{
  "scores": "10"
}
```

**Use Cases**:
- Baseline value detection
- Performance metrics
- Range analysis

## Function Combinations

You can chain iterator and reducer functions together for complex transformations.

### Iterator → Reducer Chains

**Pattern**: `field.iterator().reducer()`

**Examples**:
```javascript
// Count words in content
content.split_by_word().count()

// Sum array elements
scores.split_array().sum()

// Join array elements
tags.split_array().join()

// Find maximum score
scores.split_array().max()
```

### Multiple Iterators

**Pattern**: `field.iterator1().map().iterator2()`

**Examples**:
```javascript
// Split words, then split each word into characters
content.split_by_word().map().split_by_char()

// Split array, then process each element
data.split_array().map().process()
```

## Validation Rules

The transformation engine enforces these validation rules:

### Valid Patterns
- `field.iterator()` - Iterator after field access
- `field.reducer()` - Reducer after field access  
- `field.iterator().map()` - Map after iterator
- `field.iterator().reducer()` - Reducer after iterator
- `field.map().iterator()` - Iterator after map
- `field.map().reducer()` - Reducer after map

### Invalid Patterns
- `field.reducer().iterator()` - Iterator after reducer ❌
- `field.reducer().reducer()` - Reducer after reducer ❌
- `field.iterator().iterator()` - Iterator after iterator ❌

## Error Handling

Functions handle various error conditions gracefully:

### Empty Collections
- `count()` on empty collection returns `"0"`
- `sum()` on empty collection returns `"0"`
- `max()`/`min()` on empty collection returns `""`
- `first()`/`last()` on empty collection returns `""`

### Type Mismatches
- `sum()`/`max()`/`min()` ignore non-numeric values
- `join()` converts all values to strings
- `split_by_word()` handles any string-like input

### Invalid Input
- Functions return empty results for invalid input
- No exceptions are thrown - graceful degradation

## Performance Considerations

### Iterator Functions
- `split_by_word()`: O(n) where n is text length
- `split_array()`: O(n) where n is array size

### Reducer Functions
- `count()`: O(1) - constant time
- `sum()`/`max()`/`min()`: O(n) where n is collection size
- `join()`: O(n*m) where n is items, m is average string length
- `first()`/`last()`: O(1) - constant time

## Best Practices

### Use Cases
1. **Text Processing**: Use `split_by_word()` for word-level analysis
2. **Array Processing**: Use `split_array()` for element-level processing
3. **Aggregation**: Use reducers for statistical operations
4. **Selection**: Use `first()`/`last()` for priority-based selection

### Performance Tips
1. **Chain Order**: Place selective operations early in the chain
2. **Reduce Early**: Use reducers to minimize data volume
3. **Avoid Deep Nesting**: Keep chains simple and readable
4. **Test Edge Cases**: Validate with empty and malformed data

### Common Patterns
```javascript
// Word counting
content.split_by_word().count()

// Tag aggregation  
tags.split_array().join()

// Score analysis
scores.split_array().max()
scores.split_array().min()
scores.split_array().sum()

// Text reconstruction
content.split_by_word().join()
```

## Examples in Transform Schemas

### Blog Post Word Index
```json
{
  "name": "BlogPostWordIndex",
  "transform_fields": {
    "word": "BlogPost.map().content.split_by_word().map()",
    "word_count": "BlogPost.map().content.split_by_word().count()",
    "title": "BlogPost.map().title"
  }
}
```

### Score Analytics
```json
{
  "name": "ScoreAnalytics", 
  "transform_fields": {
    "total_score": "scores.split_array().sum()",
    "max_score": "scores.split_array().max()",
    "min_score": "scores.split_array().min()",
    "score_count": "scores.split_array().count()"
  }
}
```

### Tag Processing
```json
{
  "name": "TagProcessor",
  "transform_fields": {
    "tag": "tags.split_array().map()",
    "tag_string": "tags.split_array().join()",
    "tag_count": "tags.split_array().count()"
  }
}
```

This reference covers all available transform functions and their usage patterns. For more advanced examples and integration patterns, see the integration tests and example schemas.
