//! Prompt templates and instruction blocks for LLM-powered query analysis.
//!
//! The dynamic prompt builders live in `fold_db_node` (they depend on runtime schema
//! data), but the static instruction text is centralized here.

/// Filter type documentation shared across query analysis and followup prompts.
pub const FILTER_TYPES_INSTRUCTION: &str = r#"FILTER TYPES AVAILABLE:

Filters for HashRange schemas (have both Hash Key and Range Key):
- HashRangeKey: {"HashRangeKey": {"hash": "value", "range": "value"}} - exact match on BOTH hash key field AND range key field
- HashKey: {"HashKey": "value"} - filter on hash key field only, returns all records with this hash
- HashRangePrefix: {"HashRangePrefix": {"hash": "value", "prefix": "prefix"}} - filter on hash key field + range key field prefix
- HashPattern: {"HashPattern": "*pattern*"} - glob pattern matching on hash key field

Filters for Hash schemas (have Hash Key only, no Range Key):
- HashKey: {"HashKey": "value"} - exact match on hash key field
- HashPattern: {"HashPattern": "*pattern*"} - glob pattern matching on hash key field

Filters for Range schemas (have Range Key only):
- RangePrefix: {"RangePrefix": "prefix"} - filter on range key field, returns records with range starting with prefix
- RangePattern: {"RangePattern": "*pattern*"} - glob pattern matching on range key field
- RangeRange: {"RangeRange": {"start": "2025-01-01", "end": "2025-12-31"}} - filter on range key field for values within range

Universal filters (work on any schema type):
- SampleN: {"SampleN": 100} - return N RANDOM records (NOT sorted)
- null - no filter (return all records)"#;

/// Critical rules for selecting the right filter type.
pub const FILTER_SELECTION_RULES: &str = r#"IMPORTANT JSON FORMATTING:
- All string values in filters MUST be properly JSON-escaped
- Special characters like @ # $ etc. do NOT need escaping in JSON strings
- Example: {"HashKey": "user@domain.com"} is valid JSON

CRITICAL FILTER SELECTION RULES:
1. ALWAYS check the schema's Hash Key and Range Key fields to determine the correct filter
2. If the search term matches a Hash Key field value, use HashKey or HashPattern filter
3. If the search term matches a Range Key field value on a Range-only schema, use RangePrefix, RangePattern, or RangeRange filter
4. For HashRange schemas: Queries targeting the range key *must* also specify a Hash Key value using `HashRangeKey` or `HashRangePrefix`. If only the range key is specified, the `filter` MUST be `null`.

5. Examples of when to use each:
   - Searching for author "Jennifer Liu" on a schema with hash_field=author -> use {"HashKey": "Jennifer Liu"}
   - Searching for date "2025-09" on a HashRange schema with range_field=publish_date without a hash key -> use null filter

IMPORTANT NOTES:
- For HashRange schemas, HashKey filters operate on the hash_field, Range filters operate on the range_field
- For Hash schemas, HashKey and HashPattern filters operate on the hash_field (no range filters available)
- For Range schemas, Range filters operate on the range_field
- SampleN returns RANDOM records, NOT sorted or ordered
- For "most recent" or "latest" queries, use null filter with sort_order "desc" to get results sorted newest-first by range key
- Range keys are stored as strings and compared lexicographically"#;

/// JSON response format expected from query analysis.
pub const QUERY_RESPONSE_FORMAT: &str = r#"Respond in JSON format with:
{
  "query": {
    "schema_name": "string",
    "fields": ["field1", "field2"],
    "filter": null or one of the filter types above,
    "sort_order": "asc" or "desc" or null
  },
  "reasoning": "your analysis"
}

IMPORTANT:
- **Return ONLY the JSON object. Do NOT include any conversational text, explanations, or markdown code block delimiters (e.g., ```json).**
- For `sort_order`, if the user does not explicitly ask for a specific order (e.g., 'most recent', 'oldest first'), set it to `null`. Do NOT default to 'asc'.
- Use the EXACT filter format shown above
- For "most recent", "latest", or "newest" queries, use null filter with sort_order "desc" (NOT SampleN)
- Prefer existing approved schemas for queries"#;

/// JSON response format for followup analysis.
pub const FOLLOWUP_RESPONSE_FORMAT: &str = r#"Respond in JSON format:
{
  "needs_query": true/false,
  "query": null or {"schema_name": "...", "fields": [...], "filter": ..., "sort_order": "asc" or "desc" or null},
  "reasoning": "explanation"
}

IMPORTANT: Return ONLY the JSON object, no additional text."#;

/// System preamble for query analysis prompts.
pub const QUERY_ANALYSIS_PREAMBLE: &str =
    "You are a database query optimizer. Analyze the following natural language query and available schemas to create an execution plan.\n\n";

/// System preamble for result summarization.
pub const SUMMARIZATION_PREAMBLE: &str = "Summarize the following query results for the user.\n\n";

/// System preamble for chat follow-up answers.
pub const CHAT_PREAMBLE: &str =
    "You are helping a user explore query results. Answer their question based on \
    the context provided.\n\n";

/// System preamble for followup analysis.
pub const FOLLOWUP_ANALYSIS_PREAMBLE: &str =
    "You are analyzing whether a follow-up question can be answered from existing query results or needs a new query.\n\n";

/// System preamble for native index search term generation.
pub const NATIVE_INDEX_QUERY_TERMS_PREAMBLE: &str =
    "You are generating search terms for a native word index. Based on the user's natural language query, \
    generate relevant search terms that would help find matching records.\n\n";

/// System preamble for native index result interpretation.
pub const NATIVE_INDEX_INTERPRETATION_PREAMBLE: &str =
    "You are interpreting native index search results for a user. Analyze the search results and provide a helpful response.\n\n";

/// Guidelines for generating native index search terms.
pub const NATIVE_INDEX_SEARCH_GUIDELINES: &str = r#"Guidelines:
- Extract the most important keywords from the query
- Include specific names, places, or entities mentioned
- Generate terms that would be found in indexed text fields
- Avoid very common words (stopwords)
- Keep terms concise but meaningful
- Focus on terms that are likely to appear in the data

Examples:
- Query: "Find posts about artificial intelligence"
  Terms: ["artificial", "intelligence", "AI", "machine learning"]
- Query: "Show me articles by Jennifer Liu"
  Terms: ["Jennifer", "Liu", "Jennifer Liu"]
- Query: "Products with electronics tag"
  Terms: ["electronics", "electronic", "tech"]

Respond with a JSON array of strings:
["term1", "term2", "term3", ...]

IMPORTANT: Return ONLY the JSON array, no additional text."#;

/// System preamble for alternative query suggestion.
pub const ALTERNATIVE_QUERY_PREAMBLE: &str =
    "A query returned no results. Suggest an alternative approach to find the data the user wants.\n\n";

/// Agent system prompt preamble (the tool definitions are built dynamically).
pub const AGENT_SYSTEM_PREAMBLE: &str =
    "You are a helpful database assistant with access to tools. Use the tools to query and manipulate data to answer the user's question.\n\n";

/// Agent response format instruction.
pub const AGENT_RESPONSE_FORMAT: &str = "IMPORTANT: Always respond with valid JSON. Either:\n\
    - {\"tool\": \"tool_name\", \"params\": {...}} to call a tool\n\
    - {\"answer\": \"your response\"} to provide the final answer\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_types_lists_all_filters() {
        for filter in &[
            "HashRangeKey",
            "HashKey",
            "HashRangePrefix",
            "HashPattern",
            "RangePrefix",
            "RangePattern",
            "RangeRange",
            "SampleN",
        ] {
            assert!(
                FILTER_TYPES_INSTRUCTION.contains(filter),
                "Missing filter: {}",
                filter
            );
        }
    }

    #[test]
    fn query_response_format_has_required_fields() {
        assert!(QUERY_RESPONSE_FORMAT.contains("schema_name"));
        assert!(QUERY_RESPONSE_FORMAT.contains("fields"));
        assert!(QUERY_RESPONSE_FORMAT.contains("filter"));
        assert!(QUERY_RESPONSE_FORMAT.contains("sort_order"));
    }
}
