//! Prompt templates for AI-powered schema analysis during data ingestion.
//!
//! These prompts instruct the LLM to analyze sample JSON data and propose
//! schema definitions with field descriptions and classifications.

/// Prompt header describing the response format, schema structure, and classification rules.
///
/// Appended before the sample data in every ingestion prompt.
pub const PROMPT_HEADER: &str = r#"Create a schema for this sample json data. Return the value in this format:
{
  "new_schemas": <single_schema_definition>,
  "mutation_mappers": {json_field_name: schema_field_name}
}

Where:
- new_schemas is a single schema definition for the input data
- mutation_mappers maps ONLY TOP-LEVEL JSON field names to schema field names (e.g., {"id": "id", "user": "user"})

CRITICAL - Mutation Mappers:
- ONLY use top-level field names in mutation_mappers (e.g., "user", "comments", "id")
- DO NOT use nested paths (e.g., "user.name", "comments[*].content") - they will not work
- Nested objects and arrays will be stored as-is in their top-level field
- Example: if JSON has {"user": {"id": 1, "name": "Tom"}}, mapper should be {"user": "user"}, NOT {"user.id": "id"}

IMPORTANT - Schema Types (4 types: Single, Hash, Range, HashRange):
- ALWAYS assume the data belongs to a COLLECTION/SET of similar items, even if only one item is provided now.
- STRONGLY PREFER HashRange schemas. Most data benefits from both a hash key (for grouping) and a range key (for ordering).
- Choose a meaningful hash_field that groups related records (e.g., "author", "category", "user_id", "type", "source").
  If the data has a natural grouping dimension, use it. If not, pick the most useful field for filtering.
- Choose a range_field based on a date/timestamp if one exists (e.g., "created_at", "date", "timestamp", "posted_at").
  If no date field exists, use a unique identifier or sequential field (e.g., "id", "name").
- hash_field and range_field can use dot-notation to reference nested values (e.g., "departure.date", "departure.airport").
  The parent field (e.g., "departure") MUST still be included in "fields" and "mutation_mappers" as a top-level entry.
- NEVER use "file_type" as a hash_field or range_field — it is metadata, not a semantic grouping dimension.
  For text/document data, use "source_file" or "category" as hash_field if available, and derive the schema name from the content's topic (e.g., "recipes", "meeting_notes", "journal_entries"), not the file extension.
- Use Hash (hash_field only, no range_field) when items are uniquely keyed but have no meaningful ordering dimension.
  Good for: images (keyed by filename), user profiles (keyed by user_id), config entries (keyed by name).
- Use Range (range_field only, no hash_field) ONLY when there is genuinely no meaningful grouping dimension.
- Use Single (no "key" field) ONLY for truly singleton global config/settings with no possibility of multiple records.
- If the user provides an ARRAY of objects, you MUST use HashRange, Hash, or Range with a "key".

IMPORTANT - Schema Name and Descriptive Name:
- "name" MUST be a short, semantic, snake_case name describing the CONTENT TOPIC (e.g., "recipes", "journal_entries", "medical_records", "meeting_notes", "blog_posts").
  Think of it as a database table name — concise, plural, descriptive of the data set.
- READ THE ACTUAL CONTENT to determine the topic. A file containing a cookie recipe should produce "recipes", not "document_content".
  A file containing a journal entry should produce "journal_entries". A file about doctor visits should produce "medical_records".
- NEVER use generic names like "document_content", "text_content", "file_content", or "text_records". These are useless.
  If the data has a "content" field with text, read that text to determine the domain/topic.
- If a "category" field is present (e.g., "recipes", "journal", "health"), use it as a strong hint for the schema name.
- ALWAYS include "descriptive_name": a clear, human-readable description of what this schema stores
- Example: "name": "recipes", "descriptive_name": "Recipe Collection"

IMPORTANT - Field Descriptions:
- EVERY field MUST have a "field_descriptions" entry
- Each entry is a short natural language description of what the field represents
- Descriptions should be specific enough to distinguish semantically similar fields across different domains
- Example: "field_descriptions": {"artist": "the person who created the artwork", "title": "the name of the artwork"}

IMPORTANT - Field Classifications:
- EVERY field MUST have a "field_classifications" entry
- Analyze field semantic meaning and assign appropriate classification types
- Multiple classifications per field are encouraged (e.g., ["name:person", "word"])
- ALWAYS include "word" classification for any string field that contains searchable text
- Available classification types:
  * "word" - general text, split into words for search (MANDATORY for searchable text)
  * "name:person" - person names (kept whole: "Jennifer Liu")
  * "name:company" - company/organization names
  * "name:place" - location names (cities, countries, places)
  * "email" - email addresses
  * "phone" - phone numbers
  * "url" - URLs or domains
  * "date" - dates and timestamps
  * "hashtag" - hashtags (from social media)
  * "username" - usernames/handles
  * "number" - numeric values (amounts, counts, scores, percentages, quantities)
- "field_classifications" is a flat map from field name to list of classification strings

Example HashRange schema (PREFERRED — grouping + time ordering):
{
  "name": "social_media_posts",
  "descriptive_name": "Social Media Posts",
  "key": {"hash_field": "author", "range_field": "created_at"},
  "fields": ["created_at", "author", "content"],
  "field_descriptions": {
    "created_at": "when the post was published",
    "author": "the person who wrote the post",
    "content": "the text body of the post"
  },
  "field_classifications": {
    "created_at": ["date"],
    "author": ["name:person", "word"],
    "content": ["word"]
  }
}

Example HashRange schema with non-date range (when no timestamp exists):
{
  "name": "user_profiles",
  "descriptive_name": "User Profile Information",
  "key": {"hash_field": "department", "range_field": "id"},
  "fields": ["id", "department", "name", "age"],
  "field_descriptions": {
    "id": "unique identifier for the user",
    "department": "the department the user belongs to",
    "name": "the user's full name",
    "age": "the user's age in years"
  },
  "field_classifications": {
    "id": ["word"],
    "department": ["word"],
    "name": ["name:person", "word"],
    "age": ["number"]
  }
}

Example Hash schema (unique key, no ordering needed):
{
  "name": "image_collection",
  "descriptive_name": "Image Collection",
  "key": {"hash_field": "source_file_name"},
  "fields": ["source_file_name", "image_type", "subjects", "description"],
  "field_descriptions": {
    "source_file_name": "the filename of the image",
    "image_type": "the type or category of the image",
    "subjects": "the subjects depicted in the image",
    "description": "a description of what the image shows"
  },
  "field_classifications": {
    "source_file_name": ["word"],
    "image_type": ["word"],
    "subjects": ["word"],
    "description": ["word"]
  }
}

Example Range schema (only when NO meaningful grouping dimension exists):
{
  "name": "global_metrics",
  "descriptive_name": "Global System Metrics",
  "key": {"range_field": "recorded_at"},
  "fields": ["recorded_at", "cpu_usage", "memory_usage"],
  "field_descriptions": {
    "recorded_at": "when the metric was recorded",
    "cpu_usage": "CPU utilization percentage",
    "memory_usage": "memory utilization percentage"
  },
  "field_classifications": {
    "recorded_at": ["date"],
    "cpu_usage": ["number"],
    "memory_usage": ["number"]
  }
}

Example with Arrays and Objects (HashRange with date range):
{
  "name": "blog_posts",
  "descriptive_name": "Blog Posts with Media",
  "key": {"hash_field": "author", "range_field": "posted_at"},
  "fields": ["posted_at", "author", "content", "hashtags", "media"],
  "field_descriptions": {
    "posted_at": "when the post was published",
    "author": "the person who wrote the post",
    "content": "the text body of the post",
    "hashtags": "tags or topics associated with the post",
    "media": "URLs to attached images or videos"
  },
  "field_classifications": {
    "posted_at": ["date"],
    "author": ["name:person", "word"],
    "content": ["word"],
    "hashtags": ["hashtag", "word"],
    "media": ["url", "word"]
  }
}

IMPORTANT - Transform Fields (DSL):
- You can add a "transform_fields" map to the schema to derive new fields from existing ones.
- SYNTAX: "SourceField.function().function()"
- IMPLICIT CARDINALITY:
  * The system automatically iterates over every record in the schema (1:N). You do NOT need a .map() token.
  * Iterator Functions (like split_by_word, split_array) INCREASE depth/cardinality (one row -> many rows).
  * Reducer Functions (like count, join, sum) DECREASE depth/cardinality (many rows -> one row).
- DEPRECATION: The ".map()" token is DEPRECATED. Do not use it.
- Examples:
  * Word Count: "content.split_by_word().count()" (Iterates content -> splits into words -> counts words per row)
  * Character Count: "content.slugify().len()"
  * Array Join: "hashtags.join(', ')" (Joins array elements into a string)
"#;

/// Instructions appended to every ingestion prompt after the sample data.
pub const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and create a new schema definition in new_schemas with mutation_mappers.

CRITICAL RULES:
- ALWAYS assume data belongs to a collection. Use HashRange with a meaningful hash_field for grouping and range_field for ordering.
- PREFER a date/timestamp field as range_field (e.g., "created_at", "date", "timestamp") — this enables time-based queries. Only use an ID field if no date/timestamp exists.
- Pick a hash_field that provides a useful grouping dimension (e.g., author, category, type, source, department).
- hash_field and range_field can use dot-notation for nested values (e.g., "departure.date"). The parent must be in mutation_mappers.
- Only fall back to Range (no hash_field) if there is genuinely no meaningful grouping. Never use Single for array inputs.
- The schema "name" MUST describe the content topic, NOT the format. Read the actual text/data to determine the topic.
  Good: "recipes", "journal_entries", "medical_records", "meeting_notes". Bad: "document_content", "text_records", "file_data".
- If there is a "category" field, use it as a strong signal for the schema name.
- ALWAYS provide field_descriptions and field_classifications for every field
- For document/note/journal schemas where "title" is not guaranteed unique (e.g., dated entries, journal notes),
  use "content_hash" as the range_field if it is present in the data. This guarantees uniqueness.
  Use "title" or a category field as hash_field for grouping.
  Example for notes: {"hash_field": "title", "range_field": "content_hash"}
  This prevents collision when multiple notes share the same title.

The response must be valid JSON."#;

/// Prompt for a second AI pass that generates field_descriptions when the
/// initial schema proposal omitted them.
pub const FIELD_DESCRIPTIONS_PROMPT: &str = r#"Given the following JSON data structure and a list of field names, provide a short natural language description for each field.

Return ONLY a JSON object mapping field names to descriptions. Example:
{
  "artist": "the person who created the artwork",
  "title": "the name of the artwork",
  "year": "the year the artwork was created"
}

Descriptions should be:
- Specific enough to distinguish semantically similar fields across different domains
- Short (one sentence max)
- Focused on what the field represents, not its data type

JSON data sample:
{sample}

Fields that need descriptions:
{fields}

Return ONLY the JSON object with field descriptions. No other text."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_header_mentions_all_schema_types() {
        assert!(PROMPT_HEADER.contains("Single"));
        assert!(PROMPT_HEADER.contains("Hash"));
        assert!(PROMPT_HEADER.contains("Range"));
        assert!(PROMPT_HEADER.contains("HashRange"));
    }

    #[test]
    fn prompt_header_mentions_all_classification_types() {
        for cls in &["word", "name:person", "name:company", "name:place", "email", "phone", "url", "date", "hashtag", "username", "number"] {
            assert!(PROMPT_HEADER.contains(cls), "Missing classification type: {}", cls);
        }
    }

    #[test]
    fn prompt_actions_requires_json() {
        assert!(PROMPT_ACTIONS.contains("valid JSON"));
    }

    #[test]
    fn field_descriptions_prompt_has_placeholders() {
        assert!(FIELD_DESCRIPTIONS_PROMPT.contains("{sample}"));
        assert!(FIELD_DESCRIPTIONS_PROMPT.contains("{fields}"));
    }
}
