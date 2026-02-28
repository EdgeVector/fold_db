//! Key value extraction from JSON data based on schema key configuration.
//!
//! Extracts hash and range key values from ingested data, including
//! support for nested field paths and date normalization.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::SchemaCore;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ---- Date normalization ----

/// Try to normalize a date string to "YYYY-MM-DD HH:MM:SS" format for
/// chronological sorting. Returns the original string if it cannot be
/// parsed as a date.
pub(crate) fn try_normalize_date(value: &str) -> String {
    let trimmed = value.trim();

    // Already normalized — skip parsing
    if NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S").is_ok() {
        return trimmed.to_string();
    }

    // RFC 3339 / ISO 8601 with timezone (e.g. "2024-01-05T15:30:00Z", "2024-01-05T15:30:00+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    // RFC 2822 (e.g. "Mon, 05 Jan 2024 15:30:00 +0000")
    // Try built-in first, then strip day-of-week prefix for lenient parsing
    // (source data may have incorrect day names).
    if let Ok(dt) = DateTime::parse_from_rfc2822(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    if let Some(rest) = trimmed.split_once(", ").map(|(_, r)| r) {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(rest, "%d %b %Y %H:%M:%S %z") {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Twitter format: "Mon Jan 05 15:30:00 +0000 2024"
    // chrono can't parse %z followed by %Y, so strip the tz offset and parse
    // the rest as naive datetime with the year moved.
    if let Some(dt) = try_parse_twitter_date(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    // Timezone-aware formats
    let tz_formats = [
        "%Y-%m-%dT%H:%M:%S%z",        // "2024-01-05T15:30:00+0000"
        "%Y-%m-%dT%H:%M:%S%.f%z",     // "2024-01-05T15:30:00.000+0000"
    ];
    for fmt in &tz_formats {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(trimmed, fmt) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Naive datetime formats (no timezone)
    let naive_dt_formats = [
        "%Y-%m-%dT%H:%M:%S",          // "2024-01-05T15:30:00"
        "%m/%d/%Y %H:%M:%S",          // "01/05/2024 15:30:00"
        "%Y-%m-%d %H:%M",             // "2024-01-05 15:30"
    ];
    for fmt in &naive_dt_formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Date-only formats — normalize to midnight
    let date_formats = [
        "%Y-%m-%d",                    // "2024-01-05"
        "%m/%d/%Y",                    // "01/05/2024"
        "%B %d, %Y",                  // "January 5, 2024"
        "%b %d, %Y",                  // "Jan 5, 2024"
        "%d %B %Y",                   // "5 January 2024"
        "%d %b %Y",                   // "5 Jan 2024"
    ];
    for fmt in &date_formats {
        if let Ok(d) = NaiveDate::parse_from_str(trimmed, fmt) {
            return d.format("%Y-%m-%d 00:00:00").to_string();
        }
    }

    // Not a recognized date format — return original
    value.to_string()
}

/// Parse Twitter-style dates: "Mon Jan 05 15:30:00 +0000 2024"
/// Skips the day-of-week name and timezone offset, parses the rest.
/// This avoids chrono's strict day-of-week validation (source data may
/// have incorrect day names).
fn try_parse_twitter_date(value: &str) -> Option<NaiveDateTime> {
    // Pattern: "DDD MMM DD HH:MM:SS +ZZZZ YYYY"
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    // parts[4] should be a timezone offset like "+0000"
    let tz_part = parts[4];
    if !(tz_part.starts_with('+') || tz_part.starts_with('-')) || tz_part.len() != 5 {
        return None;
    }
    // Skip day-of-week (parts[0]) and timezone (parts[4]):
    // "Jan 05 15:30:00 2024"
    let without_dow_tz = format!("{} {} {} {}", parts[1], parts[2], parts[3], parts[5]);
    NaiveDateTime::parse_from_str(&without_dow_tz, "%b %d %H:%M:%S %Y").ok()
}

// ---- Key extraction ----

/// Extract key values from JSON data based on schema key fields.
/// Looks up the schema in the node's schema manager to find key configuration,
/// then extracts the corresponding values from the data.
pub(crate) async fn extract_key_values_from_data(
    fields_and_values: &HashMap<String, Value>,
    schema_name: &str,
    schema_manager: &Arc<SchemaCore>,
) -> IngestionResult<HashMap<String, String>> {
    let mut keys_and_values = HashMap::new();

    match schema_manager.get_schema_metadata(schema_name) {
        Ok(Some(schema)) => {
            if let Some(key_def) = &schema.key {
                // Extract hash field value if present
                if let Some(hash_field) = &key_def.hash_field {
                    if let Some(hash_value) = fields_and_values.get(hash_field) {
                        if let Some(hash_str) = hash_value.as_str() {
                            keys_and_values.insert("hash_field".to_string(), hash_str.to_string());
                        } else if let Some(hash_num) = hash_value.as_f64() {
                            keys_and_values.insert("hash_field".to_string(), hash_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Hash field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                hash_field, schema_name, hash_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Hash field '{}' not found in data for schema '{}'",
                            hash_field, schema_name
                        );
                    }
                }

                // Extract range field value if present, normalizing dates to
                // YYYY-MM-DD HH:MM:SS so records sort chronologically.
                if let Some(range_field) = &key_def.range_field {
                    if let Some(range_value) =
                        extract_nested_field_value(fields_and_values, range_field)
                    {
                        if let Some(range_str) = range_value.as_str() {
                            keys_and_values.insert("range_field".to_string(), try_normalize_date(range_str));
                        } else if let Some(range_num) = range_value.as_f64() {
                            keys_and_values.insert("range_field".to_string(), range_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Range field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                range_field, schema_name, range_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Range field '{}' not found in data for schema '{}'",
                            range_field, schema_name
                        );
                    }
                }
            }
        }
        Ok(None) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Schema '{}' not found — cannot extract key values",
                schema_name
            );
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to get schema '{}' for key extraction: {}",
                schema_name, e
            );
        }
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Extracted key values for schema '{}': {:?}",
        schema_name,
        keys_and_values
    );

    Ok(keys_and_values)
}

/// Extract nested field value from JSON data using dot notation.
pub(crate) fn extract_nested_field_value<'a>(
    fields_and_values: &'a HashMap<String, Value>,
    field_path: &str,
) -> Option<&'a Value> {
    // First try direct field access
    if let Some(value) = fields_and_values.get(field_path) {
        return Some(value);
    }

    // Then try nested field access (e.g., "like.tweetId")
    if field_path.contains('.') {
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.len() == 2 {
            if let Some(parent_value) = fields_and_values.get(parts[0]) {
                if let Some(parent_obj) = parent_value.as_object() {
                    if let Some(result) = parent_obj.get(parts[1]) {
                        return Some(result);
                    }
                }
            }
        }
    }

    // Try to find the field in nested objects
    for value in fields_and_values.values() {
        if let Some(obj) = value.as_object() {
            if let Some(nested_value) = obj.get(field_path) {
                return Some(nested_value);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_twitter_date() {
        // Correct day-of-week
        assert_eq!(
            try_normalize_date("Fri Jan 05 15:30:00 +0000 2024"),
            "2024-01-05 15:30:00"
        );
        assert_eq!(
            try_normalize_date("Fri Dec 20 08:45:12 +0000 2024"),
            "2024-12-20 08:45:12"
        );
        // Incorrect day-of-week (should still parse — real data may be wrong)
        assert_eq!(
            try_normalize_date("Mon Jan 05 15:30:00 +0000 2024"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_iso8601() {
        assert_eq!(
            try_normalize_date("2024-01-05T15:30:00+0000"),
            "2024-01-05 15:30:00"
        );
        assert_eq!(
            try_normalize_date("2024-01-05T15:30:00"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_already_normalized() {
        assert_eq!(
            try_normalize_date("2024-01-05 15:30:00"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_date_only() {
        assert_eq!(
            try_normalize_date("2024-01-05"),
            "2024-01-05 00:00:00"
        );
        assert_eq!(
            try_normalize_date("January 5, 2024"),
            "2024-01-05 00:00:00"
        );
    }

    #[test]
    fn test_normalize_rfc2822() {
        // Correct day-of-week
        assert_eq!(
            try_normalize_date("Fri, 05 Jan 2024 15:30:00 +0000"),
            "2024-01-05 15:30:00"
        );
        // Incorrect day-of-week (lenient parsing)
        assert_eq!(
            try_normalize_date("Mon, 05 Jan 2024 15:30:00 +0000"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_non_date() {
        assert_eq!(try_normalize_date("not-a-date"), "not-a-date");
        assert_eq!(try_normalize_date("12345"), "12345");
        assert_eq!(try_normalize_date("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_chronological_ordering() {
        // These Twitter-format dates sort incorrectly by day name:
        // "Fri..." < "Mon..." < "Wed..." alphabetically
        let dates = [
            "Wed Jan 01 00:00:00 +0000 2025",
            "Fri Jan 03 00:00:00 +0000 2025",
            "Mon Jan 06 00:00:00 +0000 2025",
        ];
        let mut normalized: Vec<String> = dates.iter().map(|d| try_normalize_date(d)).collect();
        let sorted = normalized.clone();
        normalized.sort();
        assert_eq!(normalized, sorted, "Normalized dates should already be in chronological order");
    }
}
