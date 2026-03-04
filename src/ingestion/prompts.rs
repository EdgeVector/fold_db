//! Prompt templates for AI-powered schema analysis.
//!
//! Separated from parsing/validation logic to make prompt updates easy
//! without touching the extraction code.

/// Prompt header describing the response format, schema structure, and classification rules.
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

IMPORTANT - Schema Types:
- STRONGLY PREFER Range schemas over Single schemas. Most data benefits from a range key.
- For storing MULTIPLE entities/records, use "key": {"range_field": "field_name"}
- Only use Single (no "key" field) when the data is truly a single global config/settings object with no records
- If the user is providing an ARRAY of objects, you MUST use a Range schema with a "key"
- Even for single objects, if the data has a date/timestamp field, use a Range schema so future records can be added
- PREFER a date or timestamp field as the range_field (like "created_at", "date", "timestamp", "posted_at") so that data can be queried by time range
- If NO date/timestamp field exists, fall back to a unique identifier field (like "id", "name", "email")

IMPORTANT - Schema Name and Descriptive Name:
- You MUST include "name": use any simple name like "Schema" (it will be replaced automatically)
- ALWAYS include "descriptive_name": a clear, human-readable description of what this schema stores
- Example: "descriptive_name": "User Profile Information" or "Customer Order Records"

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

Example Range schema with date range_field (PREFERRED when data has timestamps):
{
  "name": "Schema",
  "descriptive_name": "Social Media Posts",
  "key": {"range_field": "created_at"},
  "fields": ["created_at", "author", "content"],
  "field_classifications": {
    "created_at": ["date"],
    "author": ["name:person", "word"],
    "content": ["word"]
  }
}

Example Range schema with ID range_field (only when NO date/timestamp field exists):
{
  "name": "Schema",
  "descriptive_name": "User Profile Information",
  "key": {"range_field": "id"},
  "fields": ["id", "name", "age"],
  "field_classifications": {
    "id": ["word"],
    "name": ["name:person", "word"],
    "age": ["number"]
  }
}

Example Single schema (for one global value):
{
  "name": "Schema",
  "descriptive_name": "Global Counter Statistics",
  "fields": ["count", "total"],
  "field_classifications": {
    "count": ["number"],
    "total": ["number"]
  }
}

Example with Arrays and Objects (note: date field used as range_field):
{
  "name": "Schema",
  "descriptive_name": "Social Media Post",
  "key": {"range_field": "posted_at"},
  "fields": ["posted_at", "content", "hashtags", "media"],
  "field_classifications": {
    "posted_at": ["date"],
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

/// Instructions appended to every prompt.
pub const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and create a new schema definition in new_schemas with mutation_mappers.

CRITICAL RULES:
- If the original input was a JSON array (multiple objects), you MUST create a Range schema with "key": {"range_field": "field_name"}
- PREFER a date/timestamp field as range_field (e.g., "created_at", "date", "timestamp") — this enables time-based queries. Only use an ID field if no date/timestamp exists.
- NEVER create a Single-type schema for array inputs - they will overwrite data
- AVOID Single schemas unless the data is truly a one-off global config. If any field looks like a date, timestamp, or unique ID, use Range instead.
- ALWAYS provide field_classifications for every field

The response must be valid JSON."#;
