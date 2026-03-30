use serde::{Deserialize, Serialize};

/// Privacy classification for a schema field, controlling whether its fragments
/// can be published to the discovery network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldPrivacyClass {
    /// Fragments from this field are never published (contains PII by nature).
    NeverPublish,
    /// Fragments are published only if they pass anonymity checks (NER + entropy).
    PublishIfAnonymous,
    /// Fragments are always published (structural/categorical data, no PII risk).
    AlwaysPublish,
}

/// Decision for a single fragment after anonymity evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum FragmentDecision {
    /// Fragment passes all checks and can be published.
    Accept,
    /// Fragment contains PII or fails checks — do not publish.
    Reject(&'static str),
    /// Fragment passes local checks but should be submitted for network k-anonymity.
    SubmitForNetworkCheck,
}

/// Minimum Shannon entropy (bits per character) for a fragment to be publishable.
const MIN_ENTROPY_BITS: f64 = 1.5;
/// Minimum word count for a fragment to be publishable.
const MIN_WORD_COUNT: usize = 3;

/// Infer a default privacy class from a field name.
/// Fields whose names suggest PII are NeverPublish.
/// Fields whose names suggest categorical/structural data are AlwaysPublish.
/// Everything else is PublishIfAnonymous.
pub fn default_privacy_class(field_name: &str) -> FieldPrivacyClass {
    let lower = field_name.to_lowercase();

    // NeverPublish: fields that inherently contain PII
    const NEVER_PUBLISH: &[&str] = &[
        "name",
        "first_name",
        "last_name",
        "full_name",
        "email",
        "phone",
        "telephone",
        "mobile",
        "ssn",
        "social_security",
        "address",
        "street",
        "zip",
        "zipcode",
        "zip_code",
        "postal_code",
        "city",
        "state",
        "country",
        "dob",
        "date_of_birth",
        "birthday",
        "passport",
        "driver_license",
        "license_number",
        "credit_card",
        "card_number",
        "account_number",
        "ip_address",
        "mac_address",
        "username",
        "user_name",
        "password",
        "secret",
    ];

    for &pattern in NEVER_PUBLISH {
        if lower == pattern || lower.contains(pattern) {
            return FieldPrivacyClass::NeverPublish;
        }
    }

    // AlwaysPublish: categorical/structural fields with no PII risk
    const ALWAYS_PUBLISH: &[&str] = &[
        "category",
        "genre",
        "tags",
        "tag",
        "type",
        "kind",
        "status",
        "priority",
        "severity",
        "language",
        "format",
        "content_type",
        "mime_type",
        "color",
        "size",
        "count",
        "rating",
        "score",
        "level",
        "version",
        "platform",
        "os",
        "browser",
        "currency",
        "unit",
    ];

    for &pattern in ALWAYS_PUBLISH {
        if lower == pattern {
            return FieldPrivacyClass::AlwaysPublish;
        }
    }

    FieldPrivacyClass::PublishIfAnonymous
}

/// Check whether text contains named entities (PII patterns).
/// Returns true if any PII pattern is detected.
pub fn contains_named_entities(text: &str) -> bool {
    has_email(text) || has_phone(text) || has_url(text) || has_id_pattern(text) || has_address(text)
}

fn has_email(text: &str) -> bool {
    // Simple email pattern: word@word.word
    text.split_whitespace().any(|word| {
        let at_pos = word.find('@');
        let dot_after_at = at_pos.and_then(|pos| word[pos..].find('.'));
        at_pos.is_some() && dot_after_at.is_some()
    })
}

fn has_phone(text: &str) -> bool {
    // Count digit sequences that look like phone numbers (7+ digits with optional separators)
    let digits_only: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits_only.len() >= 7 {
        // Check for phone-like patterns: sequences of digits with separators
        let mut consecutive_phone_chars = 0;
        for ch in text.chars() {
            if ch.is_ascii_digit()
                || ch == '-'
                || ch == '('
                || ch == ')'
                || ch == ' '
                || ch == '+'
                || ch == '.'
            {
                consecutive_phone_chars += 1;
            } else {
                if consecutive_phone_chars >= 10 {
                    return true;
                }
                consecutive_phone_chars = 0;
            }
        }
        if consecutive_phone_chars >= 10 {
            return true;
        }
    }
    false
}

fn has_url(text: &str) -> bool {
    text.contains("http://") || text.contains("https://") || text.contains("www.")
}

fn has_id_pattern(text: &str) -> bool {
    // SSN pattern: XXX-XX-XXXX
    for word in text.split_whitespace() {
        let parts: Vec<&str> = word.split('-').collect();
        if parts.len() == 3 {
            let lens: Vec<usize> = parts.iter().map(|p| p.len()).collect();
            if lens == [3, 2, 4] && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                return true;
            }
        }
    }
    false
}

fn has_address(text: &str) -> bool {
    // Look for patterns like "123 Main St" or "456 Oak Avenue"
    let street_suffixes = [
        " st",
        " st.",
        " street",
        " ave",
        " ave.",
        " avenue",
        " blvd",
        " blvd.",
        " boulevard",
        " dr",
        " dr.",
        " drive",
        " rd",
        " rd.",
        " road",
        " ln",
        " ln.",
        " lane",
        " ct",
        " ct.",
        " court",
        " pl",
        " pl.",
        " place",
        " way",
        " cir",
        " circle",
        " pkwy",
        " parkway",
    ];
    let lower = text.to_lowercase();
    // Check every occurrence of each street suffix for a preceding digit
    for suffix in &street_suffixes {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(suffix) {
            let pos = search_from + rel;
            // Walk backwards up to ~30 chars, staying on a char boundary
            let start = lower[..pos]
                .char_indices()
                .rev()
                .nth(30)
                .map_or(0, |(i, _)| i);
            let preceding = &lower[start..pos];
            if preceding.chars().any(|c| c.is_ascii_digit()) {
                return true;
            }
            search_from = pos + suffix.len();
        }
    }
    false
}

/// Calculate Shannon entropy (bits per token) of text.
pub fn token_entropy(text: &str) -> f64 {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return 0.0;
    }

    let mut freq = std::collections::HashMap::new();
    let total = tokens.len() as f64;

    for token in &tokens {
        let lower = token.to_lowercase();
        *freq.entry(lower).or_insert(0u64) += 1;
    }

    freq.values()
        .map(|&count| {
            let p = count as f64 / total;
            -p * p.log2()
        })
        .sum()
}

/// Evaluate whether a fragment is safe to publish to the discovery network.
pub fn check_fragment_anonymity(
    field_name: &str,
    fragment_text: &str,
    privacy_class: FieldPrivacyClass,
) -> FragmentDecision {
    match privacy_class {
        FieldPrivacyClass::NeverPublish => {
            FragmentDecision::Reject("field is classified as NeverPublish")
        }
        FieldPrivacyClass::AlwaysPublish => FragmentDecision::Accept,
        FieldPrivacyClass::PublishIfAnonymous => {
            // Check word count
            let word_count = fragment_text.split_whitespace().count();
            if word_count < MIN_WORD_COUNT {
                return FragmentDecision::Reject("too few words for anonymity");
            }

            // Check entropy
            let entropy = token_entropy(fragment_text);
            if entropy < MIN_ENTROPY_BITS {
                return FragmentDecision::Reject("entropy too low");
            }

            // Check for PII patterns
            if contains_named_entities(fragment_text) {
                return FragmentDecision::Reject("contains PII patterns");
            }

            // Also check the field name itself for PII hints
            if default_privacy_class(field_name) == FieldPrivacyClass::NeverPublish {
                return FragmentDecision::Reject("field name suggests PII");
            }

            FragmentDecision::SubmitForNetworkCheck
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- default_privacy_class tests ---

    #[test]
    fn test_never_publish_fields() {
        assert_eq!(
            default_privacy_class("email"),
            FieldPrivacyClass::NeverPublish
        );
        assert_eq!(
            default_privacy_class("first_name"),
            FieldPrivacyClass::NeverPublish
        );
        assert_eq!(
            default_privacy_class("SSN"),
            FieldPrivacyClass::NeverPublish
        );
        assert_eq!(
            default_privacy_class("phone"),
            FieldPrivacyClass::NeverPublish
        );
        assert_eq!(
            default_privacy_class("user_email"),
            FieldPrivacyClass::NeverPublish
        );
        assert_eq!(
            default_privacy_class("ip_address"),
            FieldPrivacyClass::NeverPublish
        );
    }

    #[test]
    fn test_always_publish_fields() {
        assert_eq!(
            default_privacy_class("category"),
            FieldPrivacyClass::AlwaysPublish
        );
        assert_eq!(
            default_privacy_class("genre"),
            FieldPrivacyClass::AlwaysPublish
        );
        assert_eq!(
            default_privacy_class("tags"),
            FieldPrivacyClass::AlwaysPublish
        );
        assert_eq!(
            default_privacy_class("status"),
            FieldPrivacyClass::AlwaysPublish
        );
    }

    #[test]
    fn test_publish_if_anonymous_fields() {
        assert_eq!(
            default_privacy_class("description"),
            FieldPrivacyClass::PublishIfAnonymous
        );
        assert_eq!(
            default_privacy_class("content"),
            FieldPrivacyClass::PublishIfAnonymous
        );
        assert_eq!(
            default_privacy_class("notes"),
            FieldPrivacyClass::PublishIfAnonymous
        );
    }

    // --- NER detection tests ---

    #[test]
    fn test_ner_detects_email() {
        assert!(contains_named_entities("contact john@example.com today"));
        assert!(!contains_named_entities("no email here"));
    }

    #[test]
    fn test_ner_detects_phone() {
        assert!(contains_named_entities("call 555-123-4567 for info"));
        assert!(contains_named_entities("phone: (555) 123 4567"));
        assert!(!contains_named_entities("the year 2024"));
    }

    #[test]
    fn test_ner_detects_url() {
        assert!(contains_named_entities("visit https://example.com"));
        assert!(contains_named_entities("check www.example.com"));
        assert!(!contains_named_entities("no links here"));
    }

    #[test]
    fn test_ner_detects_ssn() {
        assert!(contains_named_entities("SSN is 123-45-6789"));
        assert!(!contains_named_entities("code ABC-DEF-GHI"));
    }

    // --- Entropy tests ---

    #[test]
    fn test_entropy_too_low() {
        // Repetitive tokens have low entropy
        let entropy = token_entropy("the the the");
        assert!(entropy < MIN_ENTROPY_BITS, "entropy was {}", entropy);
    }

    #[test]
    fn test_entropy_sufficient() {
        let entropy =
            token_entropy("delicious chocolate cake recipe with dark cocoa and fresh cream");
        assert!(
            entropy >= MIN_ENTROPY_BITS,
            "entropy was {} (expected >= {})",
            entropy,
            MIN_ENTROPY_BITS
        );
    }

    // --- Combined gate tests ---

    #[test]
    fn test_combined_gate_never_publish() {
        let decision = check_fragment_anonymity(
            "email",
            "some long text with many words here",
            FieldPrivacyClass::NeverPublish,
        );
        assert_eq!(
            decision,
            FragmentDecision::Reject("field is classified as NeverPublish")
        );
    }

    #[test]
    fn test_combined_gate_always_publish() {
        let decision =
            check_fragment_anonymity("category", "technology", FieldPrivacyClass::AlwaysPublish);
        assert_eq!(decision, FragmentDecision::Accept);
    }

    #[test]
    fn test_combined_gate_anonymous_passes() {
        let decision = check_fragment_anonymity(
            "description",
            "a beautiful sunset over the ocean with golden light",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(decision, FragmentDecision::SubmitForNetworkCheck);
    }

    #[test]
    fn test_combined_gate_anonymous_with_pii() {
        let decision = check_fragment_anonymity(
            "notes",
            "contact john@example.com for more details about this",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(decision, FragmentDecision::Reject("contains PII patterns"));
    }

    #[test]
    fn test_combined_gate_too_short() {
        let decision = check_fragment_anonymity(
            "description",
            "just two",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(
            decision,
            FragmentDecision::Reject("too few words for anonymity")
        );
    }

    #[test]
    fn test_combined_gate_field_name_pii() {
        // Even if privacy_class is PublishIfAnonymous, if the field name itself
        // matches a NeverPublish pattern, reject
        let decision = check_fragment_anonymity(
            "user_email",
            "some long text with many words and sufficient entropy",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(
            decision,
            FragmentDecision::Reject("field name suggests PII")
        );
    }

    #[test]
    fn test_entropy_token_level_unicode() {
        // Unicode text should get same entropy as ASCII with same token diversity
        let ascii_entropy = token_entropy("hello world foo bar baz");
        let unicode_entropy = token_entropy("café résumé naïve über straße");
        // Both have 5 unique tokens, so entropy should be similar
        assert!(
            (ascii_entropy - unicode_entropy).abs() < 0.1,
            "ascii={}, unicode={} — should be similar for same token count",
            ascii_entropy,
            unicode_entropy
        );
    }

    #[test]
    fn test_ner_detects_address() {
        assert!(contains_named_entities("lives at 123 Main St in town"));
        assert!(contains_named_entities("office is 456 Oak Avenue"));
        assert!(contains_named_entities("send to 789 Elm Blvd."));
        assert!(!contains_named_entities("no address information here"));
        // Second occurrence has the digit (first "Main St" has no number)
        assert!(contains_named_entities(
            "Main St is nice but 123 Elm St is better"
        ));
        // Unicode preceding the suffix must not panic
        assert!(contains_named_entities("café résumé 42 Oak Dr in town"));
        assert!(!contains_named_entities("café résumé naïve über straße"));
    }

    #[test]
    fn test_ner_detects_phone_with_dots() {
        assert!(contains_named_entities("call 555.123.4567 for info"));
    }
}
