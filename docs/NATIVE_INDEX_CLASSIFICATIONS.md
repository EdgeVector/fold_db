# Native Index Classification System

## Overview

The native index classification system allows fields to be indexed intelligently based on their semantic type. Classifications are **declarative** - they're defined in the schema as part of the field metadata, not computed at runtime.

## Architecture

### Schema-Based Classifications

Classifications are stored in the schema's `field_classifications` field:

```json
{
  "name": "SocialPost",
  "fields": ["author", "content", "email", "location"],
  "field_classifications": {
    "author": ["name:person", "word"],
    "content": ["word"],
    "email": ["email"],
    "location": ["name:place", "word"]
  }
}
```

### How It Works

1. **Schema Creation**: When a schema is created (manually or via AI), field classifications are determined and stored in the schema JSON
2. **Mutation Processing**: When data is written, the system reads the field's classifications from the schema
3. **Multi-Index Storage**: Each field value is indexed under all its classification types
4. **Efficient Search**: Users can search globally or filter by classification type

## Classification Types

| Type | Prefix | Example | Use Case |
|------|--------|---------|----------|
| General Word | `word` | `word:tokyo` | Split into words for general text search |
| Person Name | `name:person` | `name:person:jennifer liu` | Keep names together as entities |
| Company Name | `name:company` | `name:company:apple inc` | Company/organization names |
| Place Name | `name:place` | `name:place:san francisco` | Location/place names |
| Email | `email` | `email:user@example.com` | Email addresses (kept whole) |
| Phone | `phone` | `phone:+15551234567` | Phone numbers |
| URL | `url` | `url:github.com` | URLs or domains |
| Date | `date` | `date:2025-01-15` | Dates |
| Hashtag | `hashtag` | `hashtag:rust` | Hashtags |
| Username | `username` | `username:tomtang` | Usernames/handles |

## Multi-Classification

Fields can have multiple classifications. For example, `author`:
- `name:person` - Preserves "Jennifer Liu" as a single entity
- `word` - Also indexes individual words "jennifer" and "liu"

This enables both:
- Precise entity search: "Find person named Jennifer Liu"
- Word-level search: "Find anything with 'Jennifer'"

## Index Key Structure

```
{classification}:{normalized_value} → [IndexResult]
```

Examples:
```
name:person:jennifer liu → [{schema: "BlogPost", field: "author", key: "post-123", value: "Jennifer Liu"}]
word:jennifer           → [{...}, {...}]
email:jen@example.com   → [{schema: "User", field: "email", ...}]
```

## Adding Classifications to Schemas

### Manual Definition

Edit your schema JSON:

```json
{
  "name": "User",
  "fields": ["name", "email", "company"],
  "field_classifications": {
    "name": ["name:person", "word"],
    "email": ["email"],
    "company": ["name:company", "word"]
  }
}
```

### AI-Generated (Future)

During ingestion, AI can analyze field names and sample values to suggest classifications:

```rust
// TODO: Implement AI-powered classification suggestion
let classifications = ai_classifier.suggest_classifications(&schema).await?;
schema.field_classifications = Some(classifications);
```

### Heuristic-Based (Current)

The system includes heuristic rules:
- Fields containing "email" → `["email"]`
- Fields containing "name", "author", "user" → `["name:person", "word"]`
- Fields containing "location", "city", "place" → `["name:place", "word"]`
- Fields containing "tag", "hashtag" → `["hashtag", "word"]`

## Search API

### Basic Search (All Classifications)

```bash
GET /api/native-index/search?term=tokyo
```

Returns results from all classification types.

### Filtered Search (Specific Classification)

```bash
GET /api/native-index/search?term=tokyo&classification=name:place
```

Returns only place names matching "tokyo".

## Implementation Details

### Indexing Flow

1. **Mutation arrives** with data: `{"author": "Jennifer Liu"}`
2. **Schema is queried** for field classifications: `["name:person", "word"]`
3. **Multiple indexes are created**:
   - `name:person:jennifer liu` → IndexResult
   - `word:jennifer` → IndexResult
   - `word:liu` → IndexResult
4. **All indexes point to same record** with different metadata

### Storage Efficiency

- Classifications are stored once per field (in schema)
- No runtime AI calls for every mutation
- Index keys are compact strings
- Multiple classifications share the same value data (just different index keys)

## Benefits

1. **Declarative**: Classifications defined in schema, version controlled
2. **Efficient**: No AI calls during mutation processing
3. **Flexible**: Easy to add/modify classifications
4. **Backward Compatible**: Schemas without classifications default to word-only indexing
5. **Multi-dimensional**: Same field indexed multiple ways simultaneously

## Migration Path

### Existing Schemas

Schemas without `field_classifications` use fallback:
- All text fields get `word` classification (split into words)
- Maintains backward compatibility

### Adding Classifications

1. Add `field_classifications` to schema JSON
2. Re-index existing data (backfill)
3. New mutations use new classifications

## Example Use Cases

### Social Media Platform

```json
{
  "field_classifications": {
    "author": ["name:person", "username", "word"],
    "content": ["word"],
    "hashtags": ["hashtag", "word"],
    "mentions": ["username"],
    "location": ["name:place", "word"]
  }
}
```

Search scenarios:
- "Find user '@tom'" → `username:tom`
- "Find posts about #rust" → `hashtag:rust`
- "Find mentions of Tokyo" → `word:tokyo` OR `name:place:tokyo`

### E-commerce

```json
{
  "field_classifications": {
    "product_name": ["word"],
    "brand": ["name:company", "word"],
    "seller_email": ["email"],
    "description": ["word"]
  }
}
```

## Future Enhancements

1. **AI-Powered Classification**: Analyze field content to suggest optimal classifications
2. **Entity Extraction**: Extract entities from free text (e.g., find "Google" in "I work at Google")
3. **Custom Classifications**: User-defined classification types
4. **Classification Confidence**: Store confidence scores for AI-suggested classifications
5. **Classification Evolution**: Track classification changes over time

