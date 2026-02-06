//! Shared AI prompt constants for schema generation.
//!
//! Used by both OpenRouter and Ollama services.

/// Prompt header describing the response format, schema structure, and topology rules.
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
- For storing MULTIPLE entities/records, use "key": {"range_field": "field_name"}
- For storing ONE global value per field, omit the "key" field
- If the user is providing an ARRAY of objects, you MUST use a Range schema with a "key"
- The range_field should be a unique identifier field (like "name", "id", "email")

IMPORTANT - Schema Name and Descriptive Name:
- You MUST include "name": use any simple name like "Schema" (it will be replaced automatically)
- ALWAYS include "descriptive_name": a clear, human-readable description of what this schema stores
- Example: "descriptive_name": "User Profile Information" or "Customer Order Records"

IMPORTANT - Field Topologies with Classifications:
- EVERY Primitive leaf MUST include "classifications" array
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
- Topology structure:
  * Primitives: {"type": "Primitive", "value": "String", "classifications": ["name:person", "word"]}
  * Objects: {"type": "Object", "value": {"field_name": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}
  * Arrays of Primitives: {"type": "Array", "value": {"type": "Primitive", "value": "String", "classifications": ["hashtag", "word"]}}
  * Arrays of Objects: {"type": "Array", "value": {"type": "Object", "value": {"field_name": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}}

CRITICAL - Using Flattened Path Structure:
- The superset structure uses flattened dot-separated paths with actual data types
- Primitive fields show their type: "name": "string", "age": "number", "active": "boolean"
- Primitive arrays show element type in brackets: "tags[]": "[string]", "scores[]": "[number]"
- Object arrays expand their fields: "items[].id": "string", "items[].price": "number"
- IMPORTANT: Use the EXACT types shown. If a field says "string", use Primitive String. If "number", use Primitive Number.
- Convert these flattened paths into proper nested topology structures
- For arrays of objects, paths like "user_mentions[].field" mean:
  * user_mentions is an Array
  * Each array element is an Object
  * Each object has the field "field"
  * Create topology: {"type": "Array", "value": {"type": "Object", "value": {"field": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}}
- Group paths by their base path and create proper nested structures
- IMPORTANT: When you see paths like "user_mentions[].id", "user_mentions[].name", etc., this means:
  * user_mentions is an Array (not an Object)
  * Each array element is an Object with fields: id, name, etc.
  * The topology should be: {"type": "Array", "value": {"type": "Object", "value": {"id": {...}, "name": {...}}}}
- NEVER create an object with field names like "[0].id" - this is wrong!
- NEVER use generic "Object" types without specifying the exact fields inside
- ALWAYS specify the complete structure with all nested fields and their types
- For example, instead of {"type": "Object"}, use {"type": "Object", "value": {"field1": {"type": "Primitive", "value": "String"}, "field2": {"type": "Array", "value": {...}}}}

Example Range schema (for multiple records):
{
  "name": "Schema",
  "descriptive_name": "User Profile Information",
  "key": {"range_field": "id"},
  "fields": ["id", "name", "age"],
  "field_topologies": {
    "id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "name": {"root": {"type": "Primitive", "value": "String", "classifications": ["name:person", "word"]}},
    "age": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}}
  }
}

Example Single schema (for one global value):
{
  "name": "Schema",
  "descriptive_name": "Global Counter Statistics",
  "fields": ["count", "total"],
  "field_topologies": {
    "count": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}},
    "total": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}}
  }
}

Example with Arrays and Objects:
{
  "name": "Schema",
  "descriptive_name": "Social Media Post",
  "key": {"range_field": "post_id"},
  "fields": ["post_id", "content", "hashtags", "media"],
  "field_topologies": {
    "post_id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "content": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "hashtags": {"root": {"type": "Array", "value": {"type": "Primitive", "value": "String", "classifications": ["hashtag", "word"]}}},
    "media": {"root": {"type": "Array", "value": {"type": "Object", "value": {"url": {"type": "Primitive", "value": "String", "classifications": ["url", "word"]}, "type": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}}}
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
- If the original input was a JSON array (multiple objects), you MUST create a Range schema with "key": {"range_field": "unique_field"}
- NEVER create a Single-type schema for array inputs - they will overwrite data
- NEVER use generic "Object" types - always specify the complete field structure with exact types and classifications
- ALWAYS provide complete topology definitions with all nested fields explicitly defined

The response must be valid JSON."#;
