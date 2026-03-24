//! Prompt templates for AI-powered schema analysis during data ingestion.
//!
//! These prompts instruct the LLM to analyze sample JSON data and propose
//! schema definitions with field descriptions and classifications.

/// Prompt header describing the response format, schema structure, and classification rules.
///
/// Appended before the sample data in every ingestion prompt.
///
/// Optimized via autoresearch (55 experiments, gemma3:27b): reduced from 9KB to ~1.3KB
/// while maintaining perfect schema quality scores across 8 diverse test cases.
/// See autoresearch-ingestion repo for experiment log.
pub const PROMPT_HEADER: &str = r#"Create a schema for this sample json data. Return JSON with "new_schemas" (single schema) and "mutation_mappers" (top-level JSON keys only, e.g., {"id": "id"}). Keep nested objects as single fields — do NOT flatten.

- HashRange: hash_field for grouping (e.g., "author", "category", "source_file"), range_field for ordering (prefer date/timestamp, else "id"). Use dot-notation for nested values (e.g., "departure.date") but parent must be in "fields" and "mutation_mappers".
- Hash (hash_field only, NO range_field): photos/images MUST use Hash with hash_field="source_file_name" — do NOT add date_taken as range_field.
- Single (omit "key" entirely): for singleton config/settings (URLs, timeouts, feature flags).

"name": short snake_case CONTENT TOPIC (e.g., "recipes", "journal_entries", "medical_records"). Include "descriptive_name", "field_descriptions" (EVERY field).

REJECTED descriptive_name values — names made entirely of structural words are rejected: "Document Collection", "Data Records", "Text Content", "File Metadata", "Record List", "General Information". Read the actual content and name the topic specifically: "Family Vacation Photos", "Technical Architecture Notes", "Weekly Meeting Minutes".

Example:
{"name": "social_media_posts", "descriptive_name": "Social Media Posts", "key": {"hash_field": "author", "range_field": "created_at"}, "fields": ["created_at", "author", "content"], "field_descriptions": {"created_at": "...", "author": "...", "content": "..."}}"#;

/// Instructions appended to every ingestion prompt after the sample data.
pub const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and create a new schema definition in new_schemas with mutation_mappers.

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
    fn prompt_actions_requires_json() {
        assert!(PROMPT_ACTIONS.contains("valid JSON"));
    }

    #[test]
    fn field_descriptions_prompt_has_placeholders() {
        assert!(FIELD_DESCRIPTIONS_PROMPT.contains("{sample}"));
        assert!(FIELD_DESCRIPTIONS_PROMPT.contains("{fields}"));
    }
}
