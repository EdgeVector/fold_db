//! LLM prompt builders for query analysis, summarization, chat, and native index.

use crate::schema::SchemaWithState;
use super::LlmQueryService;
use serde_json::Value;
use super::super::types::Message;

impl LlmQueryService {
    /// Build the analysis prompt
    pub(super) fn build_analysis_prompt(&self, user_query: &str, schemas: &[SchemaWithState]) -> String {
        let mut prompt = String::from(
            "You are a database query optimizer. Analyze the following natural language query \
            and available schemas to create an execution plan.\n\n",
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Determine:\n\
            1. Which schema(s) to query\n\
            2. What fields to retrieve\n\
            3. What filters to apply (if any)\n\
            4. If an index is needed (consider element count > 10,000 as threshold)\n\n\
            FILTER TYPES AVAILABLE:\n\n\
            Filters for HashRange schemas (have both Hash Key and Range Key):\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match on BOTH hash key field AND range key field\n\
            - HashKey: {\"HashKey\": \"value\"} - filter on hash key field only, returns all records with this hash\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}} - filter on hash key field + range key field prefix\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - glob pattern matching on hash key field\n\n\
            Filters for Range schemas (have Range Key only):\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - filter on range key field, returns records with range starting with prefix\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern matching on range key field\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - filter on range key field for values within range\n\n\
            Universal filters (work on any schema type):\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records (NOT sorted)\n\
            - null - no filter (return all records)\n\n\
            IMPORTANT JSON FORMATTING:\n\
            - All string values in filters MUST be properly JSON-escaped\n\
            - Special characters like @ # $ etc. do NOT need escaping in JSON strings\n\
            - Example: {\"HashKey\": \"user@domain.com\"} is valid JSON\n\n\
            CRITICAL FILTER SELECTION RULES:\n\
            1. ALWAYS check the schema's Hash Key and Range Key fields to determine the correct filter\n\
            2. If the search term matches a Hash Key field value, use HashKey or HashPattern filter\n\
            3. If the search term matches a Range Key field value, use RangePrefix, RangePattern, or RangeRange filter\n\
            4. Examples of when to use each:\n\
               - Searching for author \"Jennifer Liu\" on a schema with hash_field=author → use {\"HashKey\": \"Jennifer Liu\"}\n\
               - Searching for date \"2025-09\" on a schema with range_field=publish_date → use {\"RangePrefix\": \"2025-09\"}\n\n\
            IMPORTANT NOTES:\n\
            - For HashRange schemas, HashKey filters operate on the hash_field, Range filters operate on the range_field\n\
            - For Range schemas, Range filters operate on the range_field\n\
            - SampleN returns RANDOM records, NOT sorted or ordered\n\
            - For \"most recent\" or \"latest\" queries, use null filter with sort_order \"desc\" to get results sorted newest-first by range key\n\
            - Range keys are stored as strings and compared lexicographically\n\n\
            EXAMPLES:\n\
            - Search for word \"ai\" in BlogPostWordIndex (hash_field=word): {\"HashKey\": \"ai\"} ✓ CORRECT\n\
            - Search for author \"Jennifer Liu\" in schema with hash_field=author: {\"HashKey\": \"Jennifer Liu\"} ✓ CORRECT\n\
            - Get blog post by ID in BlogPost (range_field=post_id): {\"RangePrefix\": \"post-123\"} ✓ CORRECT\n\
            - Get most recent posts: null filter + sort_order \"desc\" ✓ CORRECT\n\
            - Get posts in date range (range_field=publish_date): {\"RangeRange\": {\"start\": \"2025-09-01\", \"end\": \"2025-09-30\"}} ✓ CORRECT\n\n\
            Respond in JSON format with:\n\
            {\n\
              \"query\": {\n\
                \"schema_name\": \"string\",\n\
                \"fields\": [\"field1\", \"field2\"],\n\
                \"filter\": null or one of the filter types above,\n\
                \"sort_order\": \"asc\" or \"desc\" or null\n\
              },\n\
              \"reasoning\": \"your analysis\"\n\
            }\n\n\
            IMPORTANT: \n\
            - Return ONLY the JSON object, no additional text\n\
            - Use the EXACT filter format shown above\n\
            - For \"most recent\", \"latest\", or \"newest\" queries, use null filter with sort_order \"desc\" (NOT SampleN)\n\
            - Prefer existing approved schemas for queries"
        );

        prompt
    }

    /// Build the summarization prompt
    pub(super) fn build_summarization_prompt(&self, original_query: &str, results: &[Value]) -> String {
        let results_preview = if results.len() > 1000 {
            &results[..1000]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        format!(
            "Summarize the following query results for the user.\n\n\
            Original Query: {}\n\
            Results ({} total): {}\n\n\
            Provide:\n\
            1. High-level summary\n\
            2. Key insights\n\
            3. Notable patterns or anomalies\n\n\
            Keep the summary concise and informative.",
            original_query,
            results.len(),
            results_str
        )
    }

    /// Build the chat prompt for follow-up questions
    pub(super) fn build_chat_prompt(
        &self,
        original_query: &str,
        results: &[Value],
        conversation_history: &[Message],
        question: &str,
    ) -> String {
        let results_preview = if results.len() > 1000 {
            &results[..1000]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        let mut prompt = String::from(
            "You are helping a user explore query results. Answer their question based on \
            the context provided.\n\n",
        );

        prompt.push_str(&format!("Original Query: {}\n", original_query));
        prompt.push_str(&format!(
            "Results ({} total): {}\n\n",
            results.len(),
            results_str
        ));

        if !conversation_history.is_empty() {
            prompt.push_str("Conversation History:\n");
            for msg in conversation_history {
                prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("User Question: {}\n\n", question));
        prompt.push_str("Provide a clear, concise answer based on the data.");

        prompt
    }

    /// Build prompt to analyze if a followup needs a new query
    pub(super) fn build_followup_analysis_prompt(
        &self,
        original_query: &str,
        results: &[Value],
        question: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let results_preview = if results.len() > 100 {
            &results[..100]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        let mut prompt = String::from(
            "You are analyzing whether a follow-up question can be answered from existing query results or needs a new query.\n\n"
        );

        prompt.push_str(&format!("Original Query: {}\n", original_query));
        prompt.push_str(&format!(
            "Existing Results ({} total): {}\n\n",
            results.len(),
            results_str
        ));
        prompt.push_str(&format!("Follow-up Question: {}\n\n", question));

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?})\n",
                schema.schema.name, schema.schema.schema_type
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nDetermine if:\n");
        prompt.push_str("1. The question can be FULLY answered from the existing results (needs_query: false)\n");
        prompt.push_str(
            "2. The question needs NEW data that requires a query (needs_query: true)\n\n",
        );

        prompt.push_str("If a new query is needed, provide:\n");
        prompt.push_str("- query: The Query object to execute (same format as before)\n");
        prompt.push_str("- reasoning: Why a new query is needed\n\n");

        prompt.push_str(
            "FILTER TYPES AVAILABLE:\n\n\
            Filters for HashRange schemas (have both Hash Key and Range Key):\n\
            - HashRangeKey: {\"HashRangeKey\": {\"hash\": \"value\", \"range\": \"value\"}} - exact match on BOTH hash key field AND range key field\n\
            - HashKey: {\"HashKey\": \"value\"} - filter on hash key field only\n\
            - HashRangePrefix: {\"HashRangePrefix\": {\"hash\": \"value\", \"prefix\": \"prefix\"}} - filter on hash key field + range key field prefix\n\
            - HashPattern: {\"HashPattern\": \"*pattern*\"} - glob pattern on hash key field\n\n\
            Filters for Range schemas (have Range Key only):\n\
            - RangePrefix: {\"RangePrefix\": \"prefix\"} - filter on range key field\n\
            - RangePattern: {\"RangePattern\": \"*pattern*\"} - glob pattern on range key field\n\
            - RangeRange: {\"RangeRange\": {\"start\": \"2025-01-01\", \"end\": \"2025-12-31\"}} - filter on range key field\n\n\
            Universal filters (work on any schema type):\n\
            - SampleN: {\"SampleN\": 100} - return N RANDOM records\n\
            - null - no filter (return all records)\n\n\
            IMPORTANT JSON FORMATTING:\n\
            - All filter string values must use proper JSON format\n\
            - Special characters like @ # $ are valid in JSON strings without escaping\n\
            - Example: {\"HashKey\": \"@techinfluencer\"} is correct\n\n\
            CRITICAL: Always use key-based filters (HashKey, RangePrefix, etc.).\n\
            Check each schema's Hash Key and Range Key fields to determine which filter to use.\n\
            Example: If searching for author \"Jennifer Liu\" and schema has hash_field=author, use {\"HashKey\": \"Jennifer Liu\"}.\n\n"
        );

        prompt.push_str(
            "Respond in JSON format:\n\
            {\n\
              \"needs_query\": true/false,\n\
              \"query\": null or {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ..., \"sort_order\": \"asc\" or \"desc\" or null},\n\
              \"reasoning\": \"explanation\"\n\
            }\n\n\
            IMPORTANT: Return ONLY the JSON object, no additional text.",
        );

        prompt
    }

    /// Build prompt to generate native index query terms
    pub(super) fn build_native_index_query_terms_prompt(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let mut prompt = String::from(
            "You are generating search terms for a native word index. Based on the user's natural language query, \
            generate relevant search terms that would help find matching records.\n\n"
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!(
                        "  Hash Key: {} (indexed for fast lookup)\n",
                        hash_field
                    ));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!(
                        "  Range Key: {} (indexed for fast lookup)\n",
                        range_field
                    ));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Generate 3-8 relevant search terms that would help find records matching this query.\n\n\
            Guidelines:\n\
            - Extract key words and phrases from the query\n\
            - Include synonyms and related terms\n\
            - Consider different ways the same concept might be expressed\n\
            - Include specific names, places, or entities mentioned\n\
            - Generate terms that would be found in indexed fields\n\
            - Avoid very common words (stopwords)\n\
            - Keep terms concise but meaningful\n\n\
            Examples:\n\
            - Query: \"Find posts about artificial intelligence\"\n\
              Terms: [\"artificial\", \"intelligence\", \"AI\", \"machine learning\", \"neural network\"]\n\
            - Query: \"Show me articles by Jennifer Liu\"\n\
              Terms: [\"Jennifer\", \"Liu\", \"Jennifer Liu\", \"author\"]\n\
            - Query: \"Products with electronics tag\"\n\
              Terms: [\"electronics\", \"electronic\", \"tech\", \"gadgets\", \"devices\"]\n\n\
            Respond with a JSON array of strings:\n\
            [\"term1\", \"term2\", \"term3\", ...]\n\n\
            IMPORTANT: Return ONLY the JSON array, no additional text."
        );

        prompt
    }

    /// Build prompt for native index search term generation
    pub(super) fn build_native_index_search_prompt(
        &self,
        user_query: &str,
        schemas: &[crate::schema::SchemaWithState],
    ) -> String {
        let mut prompt = String::from(
            "You are generating search terms for a native word index system. Based on the user's natural language query, \
            generate 3-6 specific search terms that will be used to search the native index.\n\n"
        );

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!(
                        "  Hash Key: {} (indexed for fast lookup)\n",
                        hash_field
                    ));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!(
                        "  Range Key: {} (indexed for fast lookup)\n",
                        range_field
                    ));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str(&format!("\nUser Query: {}\n\n", user_query));

        prompt.push_str(
            "Generate 3-6 specific search terms that will be used to search the native word index.\n\n\
            Guidelines:\n\
            - Extract the most important keywords from the query\n\
            - Include specific names, places, or entities mentioned\n\
            - Generate terms that would be found in indexed text fields\n\
            - Avoid very common words (stopwords)\n\
            - Keep terms concise but meaningful\n\
            - Focus on terms that are likely to appear in the data\n\n\
            Examples:\n\
            - Query: \"Find posts about artificial intelligence\"\n\
              Terms: [\"artificial\", \"intelligence\", \"AI\", \"machine learning\"]\n\
            - Query: \"Show me articles by Jennifer Liu\"\n\
              Terms: [\"Jennifer\", \"Liu\", \"Jennifer Liu\"]\n\
            - Query: \"Products with electronics tag\"\n\
              Terms: [\"electronics\", \"electronic\", \"tech\"]\n\n\
            Respond with a JSON array of strings:\n\
            [\"term1\", \"term2\", \"term3\", ...]\n\n\
            IMPORTANT: Return ONLY the JSON array, no additional text."
        );

        prompt
    }

    /// Build prompt for interpreting native index results
    pub(super) fn build_native_index_interpretation_prompt(
        &self,
        original_query: &str,
        results: &[crate::db_operations::IndexResult],
    ) -> String {
        let results_preview = if results.len() > 50 {
            &results[..50]
        } else {
            results
        };

        let results_str = serde_json::to_string_pretty(results_preview)
            .unwrap_or_else(|_| "Failed to serialize results".to_string());

        format!(
            "You are interpreting native index search results for a user. Analyze the search results and provide a helpful response.\n\n\
            Original User Query: {}\n\
            Search Results ({} total, showing first {}):\n{}\n\n\
            Provide:\n\
            1. A summary of what was found\n\
            2. Key insights from the results\n\
            3. Notable patterns or interesting findings\n\
            4. If no results were found, suggest alternative search terms\n\n\
            Keep the response concise, informative, and helpful to the user.",
            original_query,
            results.len(),
            results_preview.len(),
            results_str
        )
    }

    /// Build prompt to suggest alternative query strategies
    pub(super) fn build_alternative_query_prompt(
        &self,
        original_user_query: &str,
        failed_query: &crate::schema::types::Query,
        schemas: &[crate::schema::SchemaWithState],
        previous_attempts: &[String],
    ) -> String {
        let mut prompt = String::from(
            "A query returned no results. Suggest an alternative approach to find the data the user wants.\n\n"
        );

        prompt.push_str(&format!(
            "User's Original Question: {}\n\n",
            original_user_query
        ));

        prompt.push_str("Failed Query:\n");
        prompt.push_str(&format!("  Schema: {}\n", failed_query.schema_name));
        prompt.push_str(&format!("  Fields: {:?}\n", failed_query.fields));
        prompt.push_str(&format!("  Filter: {:?}\n\n", failed_query.filter));

        if !previous_attempts.is_empty() {
            prompt.push_str("Previous Failed Attempts:\n");
            for (i, attempt) in previous_attempts.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, attempt));
            }
            prompt.push('\n');
        }

        prompt.push_str("Available Schemas:\n");
        for schema in schemas {
            prompt.push_str(&format!(
                "- {} (Type: {:?}, State: {:?})\n",
                schema.schema.name, schema.schema.schema_type, schema.state
            ));

            if let Some(ref key) = schema.schema.key {
                if let Some(ref hash_field) = key.hash_field {
                    prompt.push_str(&format!("  Hash Key: {} (filters: HashKey, HashPattern, HashRangeKey, HashRangePrefix operate on this field)\n", hash_field));
                }
                if let Some(ref range_field) = key.range_field {
                    prompt.push_str(&format!("  Range Key: {} (filters: RangePrefix, RangePattern, RangeRange, HashRangeKey, HashRangePrefix operate on this field)\n", range_field));
                }
            }

            prompt.push_str("  Fields: ");
            let field_names: Vec<String> = schema.schema.runtime_fields.keys().cloned().collect();
            prompt.push_str(&field_names.join(", "));
            prompt.push('\n');
        }

        prompt.push_str("\nSuggest ONE alternative approach:\n");
        prompt.push_str("1. Try a different schema that might have the data\n");
        prompt.push_str(
            "2. Broaden the filter (e.g., remove date constraints, use pattern matching)\n",
        );
        prompt.push_str("3. Try a different filter type (e.g., null filter for all records)\n");
        prompt.push_str("4. Search in related/index schemas\n\n");

        prompt
            .push_str("If you believe there are NO reasonable alternatives left, respond with:\n");
        prompt.push_str(
            "{\"has_alternative\": false, \"query\": null, \"reasoning\": \"explanation\"}\n\n",
        );

        prompt.push_str("Otherwise, respond with:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"has_alternative\": true,\n");
        prompt.push_str(
            "  \"query\": {\"schema_name\": \"...\", \"fields\": [...], \"filter\": ..., \"sort_order\": \"asc\" or \"desc\" or null},\n",
        );
        prompt.push_str("  \"reasoning\": \"why this approach might work\"\n");
        prompt.push_str("}\n\n");

        prompt.push_str(
            "FILTER TYPES:\n\
            For HashRange schemas (check Hash Key field):\n\
            - HashRangeKey, HashKey, HashRangePrefix, HashPattern\n\
            For Range schemas (check Range Key field):\n\
            - RangePrefix, RangePattern, RangeRange\n\
            Universal filters:\n\
            - Value (LAST RESORT ONLY), SampleN, null (all records)\n\n\
            JSON FORMATTING:\n\
            - Use proper JSON format for all filter values\n\
            - Special characters like @ # $ are valid in JSON strings\n\
            - Example: {\"Value\": \"@username\"}, {\"HashKey\": \"@mention\"}\n\n\
            CRITICAL: Prefer key-based filters over Value filter.\n\
            Check Hash Key and Range Key fields to determine correct filter.\n\
            If search matches a key field, use key filter (HashKey/RangePrefix), NOT Value filter.\n\n\
            IMPORTANT: Return ONLY the JSON object."
        );

        prompt
    }
}
