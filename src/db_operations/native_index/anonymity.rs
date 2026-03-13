// Pre-publication anonymity gate for discovery fragments.
//
// Every fragment must pass local checks before it can be submitted to the
// network discovery index. This prevents PII leakage and ensures fragments
// meet a minimum anonymity threshold.

/// Privacy classification for a schema field.
/// Determines whether field values can be published to the discovery index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPrivacyClass {
    /// PII fields — never published under any circumstances.
    /// Examples: name, email, phone, address, ssn, date_of_birth
    NeverPublish,

    /// Content fields — published only if they pass anonymity checks.
    /// Examples: description, notes, instructions, content, body
    PublishIfAnonymous,

    /// Category fields — always safe to publish (low-cardinality, non-identifying).
    /// Examples: genre, category, content_type, cuisine_type, tags
    AlwaysPublish,
}

/// Result of the local anonymity check on a fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FragmentDecision {
    /// Fragment passed all checks and can be submitted for network k-anonymity check.
    SubmitForNetworkCheck,
    /// Fragment was rejected locally. Reason is included.
    Reject(&'static str),
}

/// Minimum number of whitespace-separated tokens for a fragment to be publishable.
const MIN_TOKEN_COUNT: usize = 3;

/// Minimum Shannon entropy (bits) for a fragment's token distribution.
const MIN_ENTROPY_BITS: f64 = 1.5;

/// Infer the default privacy class for a field based on its name.
///
/// This is a heuristic — users can override per-field when opting into discovery.
pub fn default_privacy_class(field_name: &str) -> FieldPrivacyClass {
    let name = field_name.to_lowercase();

    const NEVER: &[&str] = &[
        "name", "first_name", "last_name", "full_name",
        "email", "phone", "address", "street", "city", "zip", "postal",
        "ssn", "social_security", "tax_id",
        "date_of_birth", "dob", "birthday",
        "ip_address", "ip", "mac_address",
        "passport", "license", "credit_card", "account_number",
        "username", "user_id", "user_name", "author",
        "url", "link", "website", "source_url",
        "sender", "recipient", "from", "to",
    ];

    const ALWAYS: &[&str] = &[
        "category", "genre", "type", "content_type", "kind",
        "cuisine", "language", "country", "tags", "label",
        "format", "status", "priority", "level", "rating",
    ];

    if NEVER.iter().any(|&p| name == p || name.ends_with(&format!("_{}", p)) || name.starts_with(&format!("{}_", p))) {
        FieldPrivacyClass::NeverPublish
    } else if ALWAYS.iter().any(|&p| name == p || name.ends_with(&format!("_{}", p)) || name.starts_with(&format!("{}_", p))) {
        FieldPrivacyClass::AlwaysPublish
    } else {
        FieldPrivacyClass::PublishIfAnonymous
    }
}

/// Run the local anonymity gate on a fragment.
///
/// This is Stage 1 of the two-stage anonymity check. Fragments that pass
/// this check are submitted to the network for Stage 2 (k-anonymity).
pub fn check_fragment_anonymity(
    fragment_text: &str,
    privacy_class: FieldPrivacyClass,
) -> FragmentDecision {
    match privacy_class {
        FieldPrivacyClass::NeverPublish => {
            FragmentDecision::Reject("PII field class")
        }
        FieldPrivacyClass::AlwaysPublish => {
            FragmentDecision::SubmitForNetworkCheck
        }
        FieldPrivacyClass::PublishIfAnonymous => {
            if contains_named_entities(fragment_text) {
                return FragmentDecision::Reject("contains named entities");
            }
            let token_count = fragment_text.split_whitespace().count();
            if token_count < MIN_TOKEN_COUNT {
                return FragmentDecision::Reject("too short for anonymity");
            }
            if token_entropy(fragment_text) < MIN_ENTROPY_BITS {
                return FragmentDecision::Reject("insufficient entropy");
            }
            FragmentDecision::SubmitForNetworkCheck
        }
    }
}

/// Rule-based named entity detection.
///
/// Conservative: prefers false positives (rejecting safe text) over false
/// negatives (publishing PII). No ML model needed.
pub fn contains_named_entities(text: &str) -> bool {
    has_email(text) || has_phone(text) || has_url(text)
        || has_address_pattern(text) || has_id_pattern(text)
}

/// Detect email addresses: word@word.tld
fn has_email(text: &str) -> bool {
    text.contains('@') && {
        text.split_whitespace().any(|token| {
            let parts: Vec<&str> = token.split('@').collect();
            parts.len() == 2
                && !parts[0].is_empty()
                && parts[1].contains('.')
                && parts[1].len() > 2
        })
    }
}

/// Detect phone numbers: sequences of 7+ digits with optional separators (hyphens, spaces, parens)
fn has_phone(text: &str) -> bool {
    // Look for phone-like patterns: groups of digits separated by hyphens/spaces/parens
    // that total 7+ digits within a compact span
    for token in text.split_whitespace() {
        // Strip non-digit, non-separator chars
        let digits_only: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits_only.len() >= 7 {
            // Check that the token looks phone-like (digits with separators, not prose)
            let non_phone_chars = token.chars().filter(|c| {
                !c.is_ascii_digit() && *c != '-' && *c != '(' && *c != ')' && *c != '+' && *c != '.'
            }).count();
            if non_phone_chars == 0 {
                return true;
            }
        }
    }
    // Also check for spaced phone numbers like "555 123 4567"
    // by scanning for runs of digit-groups separated by single spaces/hyphens
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut digit_count = 0;
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '-' || chars[i] == ' ' || chars[i] == '(' || chars[i] == ')' || chars[i] == '+' || chars[i] == '.') {
                if chars[i].is_ascii_digit() {
                    digit_count += 1;
                }
                i += 1;
            }
            if digit_count >= 7 {
                // Check the span isn't too wide (phone numbers are compact)
                let span_len = i - start;
                if span_len <= digit_count * 2 + 5 {
                    return true;
                }
            }
        } else {
            i += 1;
        }
    }
    false
}

/// Detect URLs: http(s)://... or www....
fn has_url(text: &str) -> bool {
    text.contains("http://") || text.contains("https://") || text.contains("www.")
}

/// Detect street address patterns: number + street name + (St|Ave|Blvd|Dr|Rd|Ln)
fn has_address_pattern(text: &str) -> bool {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    for window in words.windows(3) {
        if window[0].chars().all(|c| c.is_ascii_digit()) {
            let suffix = window[2].trim_end_matches(['.', ',']);
            if matches!(suffix, "st" | "ave" | "avenue" | "blvd" | "boulevard" | "dr" | "drive"
                | "rd" | "road" | "ln" | "lane" | "ct" | "court" | "way" | "pl" | "place"
                | "street" | "circle" | "terrace" | "pkwy" | "parkway") {
                return true;
            }
        }
    }
    false
}

/// Detect ID-like patterns: SSN (XXX-XX-XXXX), credit card (4 groups of 4 digits)
fn has_id_pattern(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in &words {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '-');
        // SSN pattern: 3-2-4 digits
        if cleaned.len() == 11 {
            let parts: Vec<&str> = cleaned.split('-').collect();
            if parts.len() == 3
                && parts[0].len() == 3 && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2 && parts[1].chars().all(|c| c.is_ascii_digit())
                && parts[2].len() == 4 && parts[2].chars().all(|c| c.is_ascii_digit())
            {
                return true;
            }
        }
    }
    false
}

/// Shannon entropy of a fragment's whitespace-separated tokens.
///
/// Higher entropy = more diverse token distribution = less identifying.
/// A fragment of all identical tokens has entropy 0.
pub fn token_entropy(text: &str) -> f64 {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return 0.0;
    }

    let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for token in &tokens {
        *freq.entry(token).or_default() += 1;
    }

    let n = tokens.len() as f64;
    freq.values()
        .map(|&count| {
            let p = count as f64 / n;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- FieldPrivacyClass defaults ---

    #[test]
    fn test_pii_fields_are_never_publish() {
        assert_eq!(default_privacy_class("email"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("phone"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("first_name"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("address"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("ssn"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("date_of_birth"), FieldPrivacyClass::NeverPublish);
        assert_eq!(default_privacy_class("user_name"), FieldPrivacyClass::NeverPublish);
    }

    #[test]
    fn test_category_fields_are_always_publish() {
        assert_eq!(default_privacy_class("category"), FieldPrivacyClass::AlwaysPublish);
        assert_eq!(default_privacy_class("genre"), FieldPrivacyClass::AlwaysPublish);
        assert_eq!(default_privacy_class("content_type"), FieldPrivacyClass::AlwaysPublish);
        assert_eq!(default_privacy_class("cuisine"), FieldPrivacyClass::AlwaysPublish);
        assert_eq!(default_privacy_class("tags"), FieldPrivacyClass::AlwaysPublish);
    }

    #[test]
    fn test_content_fields_are_publish_if_anonymous() {
        assert_eq!(default_privacy_class("description"), FieldPrivacyClass::PublishIfAnonymous);
        assert_eq!(default_privacy_class("notes"), FieldPrivacyClass::PublishIfAnonymous);
        assert_eq!(default_privacy_class("ingredients"), FieldPrivacyClass::PublishIfAnonymous);
        assert_eq!(default_privacy_class("content"), FieldPrivacyClass::PublishIfAnonymous);
    }

    // --- NER detection ---

    #[test]
    fn test_detects_email() {
        assert!(contains_named_entities("Contact me at john@example.com"));
        assert!(!contains_named_entities("A recipe for chocolate cake"));
    }

    #[test]
    fn test_detects_phone() {
        assert!(contains_named_entities("Call 555-123-4567 for info"));
        assert!(contains_named_entities("Phone: 5551234567"));
        assert!(!contains_named_entities("Mix 2 cups flour with 3 eggs"));
    }

    #[test]
    fn test_detects_url() {
        assert!(contains_named_entities("Visit https://example.com"));
        assert!(contains_named_entities("Go to www.example.com"));
        assert!(!contains_named_entities("A delicious curry recipe"));
    }

    #[test]
    fn test_detects_address() {
        assert!(contains_named_entities("Located at 123 Main St"));
        assert!(contains_named_entities("Visit us at 456 Oak Avenue"));
        assert!(!contains_named_entities("Bake at 350 degrees for 30 minutes"));
    }

    #[test]
    fn test_detects_ssn() {
        assert!(contains_named_entities("SSN is 123-45-6789"));
        assert!(!contains_named_entities("The ratio is 3-to-1"));
    }

    // --- Entropy ---

    #[test]
    fn test_entropy_single_token() {
        assert_eq!(token_entropy("hello"), 0.0);
    }

    #[test]
    fn test_entropy_all_unique() {
        let entropy = token_entropy("the quick brown fox jumps");
        assert!(entropy > 2.0, "Expected high entropy for all-unique tokens, got {}", entropy);
    }

    #[test]
    fn test_entropy_all_same() {
        assert_eq!(token_entropy("the the the the"), 0.0);
    }

    // --- Fragment decisions ---

    #[test]
    fn test_never_publish_always_rejects() {
        let result = check_fragment_anonymity("perfectly fine text here", FieldPrivacyClass::NeverPublish);
        assert_eq!(result, FragmentDecision::Reject("PII field class"));
    }

    #[test]
    fn test_always_publish_always_accepts() {
        let result = check_fragment_anonymity("x", FieldPrivacyClass::AlwaysPublish);
        assert_eq!(result, FragmentDecision::SubmitForNetworkCheck);
    }

    #[test]
    fn test_rejects_short_fragments() {
        let result = check_fragment_anonymity("hi", FieldPrivacyClass::PublishIfAnonymous);
        assert_eq!(result, FragmentDecision::Reject("too short for anonymity"));
    }

    #[test]
    fn test_rejects_fragments_with_email() {
        let result = check_fragment_anonymity(
            "Send your recipes to chef@kitchen.com for review",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(result, FragmentDecision::Reject("contains named entities"));
    }

    #[test]
    fn test_accepts_anonymous_content() {
        let result = check_fragment_anonymity(
            "A rich Japanese curry with potatoes and carrots simmered in coconut milk",
            FieldPrivacyClass::PublishIfAnonymous,
        );
        assert_eq!(result, FragmentDecision::SubmitForNetworkCheck);
    }
}
