//! Rules-based keyword extraction for native indexing.
//!
//! Tokenizes text, removes stopwords, stems words, and deduplicates.
//! No LLM dependency — works offline, instant, deterministic.

use chrono::NaiveDate;
use regex::Regex;
use rust_stemmers::{Algorithm, Stemmer};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Lazily-initialized English stopword set.
fn stopwords() -> &'static HashSet<String> {
    static STOP: OnceLock<HashSet<String>> = OnceLock::new();
    STOP.get_or_init(|| {
        stop_words::get(stop_words::LANGUAGE::English)
            .iter()
            .map(|w| w.to_lowercase())
            .collect()
    })
}

/// Extract keywords per field from a map of field values.
///
/// For each field, tokenizes the value text, removes stopwords,
/// stems each token, and deduplicates. Returns only fields that
/// produced at least one keyword.
pub fn extract_keywords_per_field(
    fields: &HashMap<String, Value>,
) -> HashMap<String, Vec<String>> {
    let stemmer = Stemmer::create(Algorithm::English);
    let stops = stopwords();

    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (field_name, value) in fields {
        let text = value_to_text(value);
        let keywords = extract_from_text(&text, &stemmer, stops);
        if !keywords.is_empty() {
            result.insert(field_name.clone(), keywords);
        }
    }
    result
}

/// Convert a JSON value to searchable text.
fn value_to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => arr
            .iter()
            .map(value_to_text)
            .collect::<Vec<_>>()
            .join(" "),
        Value::Object(obj) => obj
            .values()
            .map(value_to_text)
            .collect::<Vec<_>>()
            .join(" "),
        Value::Null => String::new(),
    }
}

/// Lazily-initialized email regex.
fn email_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap()
    })
}

/// Extract email addresses from raw text before tokenization.
///
/// Returns deduplicated, lowercased email strings.
fn extract_emails(text: &str) -> Vec<String> {
    let pattern = email_pattern();
    let mut seen = HashSet::new();
    let mut emails = Vec::new();

    for mat in pattern.find_iter(text) {
        let email = mat.as_str().to_lowercase();
        if seen.insert(email.clone()) {
            emails.push(email);
        }
    }

    emails
}

/// Lazily-initialized date regex patterns.
fn date_patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // ISO: 2024-01-05
            Regex::new(r"\b(\d{4})-(\d{1,2})-(\d{1,2})\b").unwrap(),
            // US slash: 01/05/2024
            Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b").unwrap(),
            // Dot: 01.05.2024
            Regex::new(r"\b(\d{1,2})\.(\d{1,2})\.(\d{4})\b").unwrap(),
            // Named month (long): January 5th, 2024
            Regex::new(r"(?i)\b(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{1,2})(?:st|nd|rd|th)?,?\s+(\d{4})\b").unwrap(),
            // Named month (short): Jan 5, 2024
            Regex::new(r"(?i)\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})(?:st|nd|rd|th)?,?\s+(\d{4})\b").unwrap(),
            // Day-first named (long): 5th January 2024
            Regex::new(r"(?i)\b(\d{1,2})(?:st|nd|rd|th)?\s+(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{4})\b").unwrap(),
            // Day-first named (short): 5 Jan 2024
            Regex::new(r"(?i)\b(\d{1,2})(?:st|nd|rd|th)?\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\b").unwrap(),
        ]
    })
}

/// Parse a month name (long or short) into a month number.
fn month_name_to_num(name: &str) -> Option<u32> {
    match name.to_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

/// Try to parse a single regex capture into a NaiveDate based on the pattern index.
fn parse_date_capture(cap: &regex::Captures, pattern_index: usize) -> Option<NaiveDate> {
    match pattern_index {
        // ISO: year-month-day
        0 => {
            let year: i32 = cap[1].parse().ok()?;
            let month: u32 = cap[2].parse().ok()?;
            let day: u32 = cap[3].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        // US slash / dot: month/day/year or month.day.year
        1 | 2 => {
            let month: u32 = cap[1].parse().ok()?;
            let day: u32 = cap[2].parse().ok()?;
            let year: i32 = cap[3].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        // Named month first: Month day, year
        3 | 4 => {
            let month = month_name_to_num(&cap[1])?;
            let day: u32 = cap[2].parse().ok()?;
            let year: i32 = cap[3].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        // Day-first named: day Month year
        5 | 6 => {
            let day: u32 = cap[1].parse().ok()?;
            let month = month_name_to_num(&cap[2])?;
            let year: i32 = cap[3].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        _ => None,
    }
}

/// Extract dates from raw text and return normalized ISO-8601 strings (YYYY-MM-DD).
///
/// Runs regex patterns against the raw text before tokenization would break
/// date strings apart. Invalid dates (e.g. month 13) are silently skipped.
fn extract_dates(text: &str) -> Vec<String> {
    let patterns = date_patterns();
    let mut seen = HashSet::new();
    let mut dates = Vec::new();

    for (i, pat) in patterns.iter().enumerate() {
        for cap in pat.captures_iter(text) {
            if let Some(date) = parse_date_capture(&cap, i) {
                let iso = date.format("%Y-%m-%d").to_string();
                if seen.insert(iso.clone()) {
                    dates.push(iso);
                }
            }
        }
    }

    dates
}

/// Tokenize text, filter stopwords, stem, and deduplicate.
///
/// Returns both the stemmed form and the original token when they differ,
/// so searches match either form.
fn extract_from_text(text: &str, stemmer: &Stemmer, stops: &HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut keywords = Vec::new();

    // Extract emails from raw text before tokenization breaks them
    for email in extract_emails(text) {
        if seen.insert(email.clone()) {
            keywords.push(email);
        }
    }

    // Extract normalized dates from raw text before tokenization breaks them
    for date in extract_dates(text) {
        if seen.insert(date.clone()) {
            keywords.push(date);
        }
    }

    for token in tokenize(text) {
        if token.len() < 2 || stops.contains(&token) {
            continue;
        }

        // Add the original token
        if seen.insert(token.clone()) {
            keywords.push(token.clone());
        }

        // Add the stemmed form if different
        let stemmed = stemmer.stem(&token).to_string();
        if stemmed.len() >= 2 && stemmed != token && seen.insert(stemmed.clone()) {
            keywords.push(stemmed);
        }
    }

    keywords
}

/// Split text into lowercase tokens on whitespace and punctuation.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '@' && c != '#')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_keywords_per_field_basic() {
        let fields = HashMap::from([
            ("content".to_string(), json!("Rust is a systems programming language")),
            ("author".to_string(), json!("Alice Johnson")),
        ]);

        let result = extract_keywords_per_field(&fields);

        // "content" should have keywords (rust, systems, programming, language, etc.)
        let content_kws = result.get("content").expect("content should have keywords");
        assert!(content_kws.iter().any(|k| k == "rust"), "Should contain 'rust': {:?}", content_kws);
        // "is" and "a" are stopwords, should be filtered
        assert!(!content_kws.iter().any(|k| k == "is"), "Should not contain stopword 'is': {:?}", content_kws);
        assert!(!content_kws.iter().any(|k| k == "a"), "Should not contain stopword 'a': {:?}", content_kws);

        // "author" should have keywords
        let author_kws = result.get("author").expect("author should have keywords");
        assert!(author_kws.iter().any(|k| k == "alice"), "Should contain 'alice': {:?}", author_kws);
        assert!(author_kws.iter().any(|k| k == "johnson"), "Should contain 'johnson': {:?}", author_kws);
    }

    #[test]
    fn test_stemming_produces_both_forms() {
        let fields = HashMap::from([
            ("text".to_string(), json!("programming languages")),
        ]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").expect("text should have keywords");

        // Should have original "programming" and stemmed "program"
        assert!(kws.iter().any(|k| k == "programming"), "Should contain 'programming': {:?}", kws);
        assert!(kws.iter().any(|k| k == "program"), "Should contain stemmed 'program': {:?}", kws);
    }

    #[test]
    fn test_numbers_preserved() {
        let fields = HashMap::from([
            ("version".to_string(), json!("v2.0 release 2024")),
        ]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("version").expect("version should have keywords");
        assert!(kws.iter().any(|k| k == "2024"), "Should preserve numbers: {:?}", kws);
    }

    #[test]
    fn test_handles_at_and_hash() {
        let fields = HashMap::from([
            ("text".to_string(), json!("mention @devlead42 and #rustlang")),
        ]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").expect("text should have keywords");
        assert!(kws.iter().any(|k| k == "@devlead42"), "Should preserve @mentions: {:?}", kws);
        assert!(kws.iter().any(|k| k == "#rustlang"), "Should preserve #hashtags: {:?}", kws);
    }

    #[test]
    fn test_empty_and_null_values() {
        let fields = HashMap::from([
            ("empty".to_string(), json!("")),
            ("null".to_string(), Value::Null),
        ]);

        let result = extract_keywords_per_field(&fields);
        assert!(result.is_empty(), "Empty/null values should produce no keywords");
    }

    #[test]
    fn test_nested_json_values() {
        let fields = HashMap::from([
            ("data".to_string(), json!({"name": "Alice", "tags": ["rust", "programming"]})),
        ]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("data").expect("data should have keywords");
        assert!(kws.iter().any(|k| k == "alice"), "Should extract from nested objects: {:?}", kws);
        assert!(kws.iter().any(|k| k == "rust"), "Should extract from nested arrays: {:?}", kws);
    }

    #[test]
    fn test_deduplication() {
        let fields = HashMap::from([
            ("text".to_string(), json!("rust Rust RUST rust")),
        ]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").expect("text should have keywords");
        let rust_count = kws.iter().filter(|k| *k == "rust").count();
        assert_eq!(rust_count, 1, "Should deduplicate: {:?}", kws);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokens = tokenize("hello, world! foo-bar baz_qux");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"foo".to_string()));
        assert!(tokens.contains(&"bar".to_string()));
        assert!(tokens.contains(&"baz_qux".to_string())); // underscore preserved
    }

    #[test]
    fn test_stopwords_are_filtered() {
        let stops = stopwords();

        // Common English words that SHOULD be filtered
        let filtered = [
            "the", "is", "a", "an", "and", "or", "but", "in", "on", "at",
            "to", "for", "of", "with", "by", "from", "as", "into", "about",
            "it", "he", "she", "we", "they", "this", "that", "was", "were",
            "be", "been", "being", "have", "has", "had", "do", "does", "did",
            "will", "would", "could", "should", "may", "might", "can",
            "not", "no", "so", "if", "then", "than", "too", "very",
            "just", "how", "what", "when", "where", "who", "which", "why",
            "all", "each", "every", "both", "few", "more", "most", "some",
            "any", "other", "its", "my", "your", "his", "her", "our", "their",
        ];
        for word in &filtered {
            assert!(
                stops.contains(*word),
                "'{}' should be a stopword but is NOT in the list",
                word
            );
        }

        // Print which common words the library considers stopwords.
        // The stop-words crate uses a broad list (~1300 words), much larger
        // than the classic NLTK 179-word list.
        let probe_words = [
            // Greetings / filler
            "hello", "hi", "hey", "goodbye", "yes", "no", "ok", "please", "thanks",
            // Common adjectives / adverbs
            "good", "bad", "new", "old", "big", "small", "great", "little",
            "first", "last", "long", "right", "high", "low", "best", "next",
            "well", "even", "back", "still", "much", "never", "always",
            // Common nouns that might surprise
            "world", "name", "time", "day", "way", "part", "place",
            "computer", "science", "music", "server", "network",
            // Domain words that should definitely survive
            "rust", "alice", "programming", "database", "twitter",
            "photo", "document", "project", "recipe", "travel",
            "schema", "mutation", "molecule", "bitcoin", "ethereum",
        ];

        let mut stopped: Vec<&str> = Vec::new();
        let mut kept: Vec<&str> = Vec::new();
        for word in &probe_words {
            if stops.contains(*word) {
                stopped.push(word);
            } else {
                kept.push(word);
            }
        }
        println!("\n--- Stopword probe results ({} total stopwords) ---", stops.len());
        println!("FILTERED (in stopword list): {:?}", stopped);
        println!("KEPT (not in stopword list):  {:?}", kept);

        // Domain-specific words must never be stopwords
        let must_survive = ["rust", "alice", "programming", "database", "schema", "mutation", "bitcoin"];
        for word in &must_survive {
            assert!(!stops.contains(*word), "'{}' is a domain word and must NOT be a stopword", word);
        }
    }

    #[test]
    fn test_stopwords_filtered_in_extraction() {
        // Verify stopwords are actually removed during keyword extraction
        let fields = HashMap::from([(
            "text".to_string(),
            json!("The quick brown fox jumps over the lazy dog and it was very happy"),
        )]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").expect("text should have keywords");

        // Stopwords should be gone
        let stopwords_that_should_be_absent = ["the", "over", "and", "it", "was", "very"];
        for word in &stopwords_that_should_be_absent {
            assert!(
                !kws.iter().any(|k| k == word),
                "Stopword '{}' should have been filtered but found in: {:?}",
                word, kws
            );
        }

        // Content words should remain
        let content_that_should_be_present = ["quick", "brown", "fox", "jumps", "lazy", "dog", "happy"];
        for word in &content_that_should_be_present {
            assert!(
                kws.iter().any(|k| k == word),
                "Content word '{}' should be present but missing from: {:?}",
                word, kws
            );
        }
    }

    #[test]
    fn test_stopword_list_size() {
        let stops = stopwords();
        // The NLTK English stopword list has ~179 words.
        // Verify we have a substantial list loaded, not an empty or tiny set.
        assert!(
            stops.len() > 100,
            "Stopword list should have 100+ entries, got {}",
            stops.len()
        );
        println!("Stopword list contains {} words", stops.len());
    }

    #[test]
    fn test_single_char_tokens_filtered() {
        // Single-character tokens are filtered by the len < 2 check,
        // even if they aren't in the stopword list
        let fields = HashMap::from([(
            "text".to_string(),
            json!("I went to a B and B"),
        )]);

        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text");
        // "I", "a" are single chars; "to", "and" are stopwords
        // "went" should survive; "B" is single char
        if let Some(kws) = kws {
            assert!(!kws.iter().any(|k| k.len() < 2), "No single-char tokens should survive: {:?}", kws);
            assert!(kws.iter().any(|k| k == "went"), "Content word 'went' should survive: {:?}", kws);
        }
    }

    // --- Email extraction tests ---

    #[test]
    fn test_extract_emails_basic() {
        let emails = extract_emails("Contact alice@example.com for details");
        assert_eq!(emails, vec!["alice@example.com"]);
    }

    #[test]
    fn test_extract_emails_multiple() {
        let emails = extract_emails("Email alice@example.com or bob@test.org");
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"alice@example.com".to_string()));
        assert!(emails.contains(&"bob@test.org".to_string()));
    }

    #[test]
    fn test_extract_emails_dedup() {
        let emails = extract_emails("alice@example.com and Alice@Example.com again");
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0], "alice@example.com");
    }

    #[test]
    fn test_emails_and_keywords_coexist() {
        let fields = HashMap::from([(
            "text".to_string(),
            json!("Meeting with alice@example.com on January 5th, 2024 about Rust"),
        )]);
        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").unwrap();
        assert!(kws.contains(&"alice@example.com".to_string()), "Should contain email: {:?}", kws);
        assert!(kws.contains(&"2024-01-05".to_string()), "Should contain date: {:?}", kws);
        assert!(kws.contains(&"rust".to_string()), "Should contain 'rust': {:?}", kws);
    }

    // --- Date extraction tests ---

    #[test]
    fn test_extract_dates_iso() {
        let dates = extract_dates("Event on 2024-01-05 was great");
        assert_eq!(dates, vec!["2024-01-05"]);
    }

    #[test]
    fn test_extract_dates_us_slash() {
        let dates = extract_dates("Scheduled for 01/05/2024");
        assert_eq!(dates, vec!["2024-01-05"]);
    }

    #[test]
    fn test_extract_dates_named_month() {
        let dates = extract_dates("January 5th, 2024 is the date");
        assert_eq!(dates, vec!["2024-01-05"]);

        let dates = extract_dates("Jan 5, 2024 is the date");
        assert_eq!(dates, vec!["2024-01-05"]);
    }

    #[test]
    fn test_extract_dates_day_first() {
        let dates = extract_dates("5th January 2024 is confirmed");
        assert_eq!(dates, vec!["2024-01-05"]);

        let dates = extract_dates("5 Jan 2024 confirmed");
        assert_eq!(dates, vec!["2024-01-05"]);
    }

    #[test]
    fn test_extract_dates_dot_format() {
        let dates = extract_dates("Date: 01.05.2024");
        assert_eq!(dates, vec!["2024-01-05"]);
    }

    #[test]
    fn test_dates_normalized_in_extraction() {
        let fields = HashMap::from([(
            "text".to_string(),
            json!("Event on 01/05/2024 confirmed for January 5th, 2024"),
        )]);
        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").unwrap();
        // "2024-01-05" should appear exactly once despite two date formats
        assert_eq!(kws.iter().filter(|k| *k == "2024-01-05").count(), 1,
            "Normalized date should appear exactly once: {:?}", kws);
    }

    #[test]
    fn test_mixed_dates_and_keywords() {
        let fields = HashMap::from([(
            "text".to_string(),
            json!("Meeting with Alice on January 5th, 2024 about Rust programming"),
        )]);
        let result = extract_keywords_per_field(&fields);
        let kws = result.get("text").unwrap();
        assert!(kws.contains(&"2024-01-05".to_string()), "Should contain date: {:?}", kws);
        assert!(kws.contains(&"alice".to_string()), "Should contain 'alice': {:?}", kws);
        assert!(kws.contains(&"rust".to_string()), "Should contain 'rust': {:?}", kws);
        assert!(kws.contains(&"programming".to_string()) || kws.contains(&"program".to_string()),
            "Should contain 'programming' or 'program': {:?}", kws);
    }

    #[test]
    fn test_invalid_dates_ignored() {
        // Invalid month/day combinations should not produce date keywords
        let dates = extract_dates("13/45/2024 is not a valid date");
        assert!(dates.is_empty(), "Invalid dates should be ignored: {:?}", dates);
    }

    #[test]
    fn test_no_dates_in_plain_text() {
        let dates = extract_dates("Hello world this is plain text without dates");
        assert!(dates.is_empty(), "No dates should be found: {:?}", dates);
    }

    #[test]
    fn test_multiple_different_dates() {
        let dates = extract_dates("From 2024-01-05 to 2024-12-31");
        assert_eq!(dates.len(), 2);
        assert!(dates.contains(&"2024-01-05".to_string()));
        assert!(dates.contains(&"2024-12-31".to_string()));
    }
}
