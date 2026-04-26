//! Prompt templates for field sensitivity and domain classification.
//!
//! Used by the schema service to classify new canonical fields.

/// Build the classification prompt for a single field.
///
/// The LLM should return a JSON object with `sensitivity_level` (0–4) and `data_domain`.
pub fn build_classification_prompt(field_name: &str, description: &str) -> String {
    format!(
        r#"Classify this database field's data sensitivity. Return ONLY a JSON object with two fields, no explanation.

Field name: "{field_name}"
Description: "{description}"

Sensitivity levels:
0 = Public (freely distributable, no restrictions)
1 = Internal (not sensitive but not for public release)
2 = Confidential (business-sensitive, competitive value)
3 = Restricted (personally identifiable or individually attributable)
4 = Highly Restricted (regulated data: HIPAA, financial records, biometric)

Data domains: "general", "financial", "medical", "identity", "behavioral", "location"

Return format: {{"sensitivity_level": <0-4>, "data_domain": "<domain>"}}"#
    )
}

/// Valid interest categories for field-to-category mapping.
/// This is the single source of truth — used by the LLM prompt, the schema service,
/// and downstream discovery features.
pub const INTEREST_CATEGORIES: &[&str] = &[
    "Photography",
    "Cooking",
    "Running",
    "Software Engineering",
    "Music",
    "Travel",
    "Fitness",
    "Reading",
    "Gaming",
    "Finance",
    "Gardening",
    "Art & Design",
    "Parenting",
    "Health & Wellness",
    "Sports",
    "Movies & TV",
    "Science",
    "Writing",
    "Fashion",
    "Home Improvement",
    "Pets",
    "Automotive",
    "Productivity",
    "Social Media",
    "Education",
];

/// Build the interest category classification prompt for a single field.
///
/// The LLM should return a JSON object with `interest_category` (one of the valid
/// categories, or null if the field doesn't map to a user interest).
pub fn build_interest_category_prompt(field_name: &str, description: &str) -> String {
    let categories = INTEREST_CATEGORIES.join(", ");
    format!(
        r#"Classify this database field into a user interest category. Return ONLY a JSON object with one field, no explanation.

Field name: "{field_name}"
Description: "{description}"

Valid interest categories: {categories}

If this field clearly relates to one of the above interests, return that category.
If this field is a structural/metadata field (like id, hash, timestamp, source, content_hash) or doesn't map to any interest, return null.

Return format: {{"interest_category": "<category>" | null}}"#
    )
}

/// Build a batch classification prompt for multiple fields at once.
///
/// Combines sensitivity classification and interest category into a single LLM call.
/// Returns a prompt that asks the LLM for a JSON object mapping field names to their
/// classification and interest category.
pub fn build_batch_classification_prompt(fields: &[(&str, &str)]) -> String {
    let categories = INTEREST_CATEGORIES.join(", ");

    let field_list: String = fields
        .iter()
        .map(|(name, desc)| format!("  - \"{name}\": \"{desc}\""))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"Classify each database field's sensitivity and interest category. Return ONLY a JSON object, no explanation.

Fields (name: description):
{field_list}

For each field, return an entry with:
- "sensitivity_level": 0-4 (0=Public, 1=Internal, 2=Confidential, 3=Restricted/PII, 4=Highly Restricted/regulated)
- "data_domain": one of "general", "financial", "medical", "identity", "behavioral", "location"
- "interest_category": one of [{categories}] or null if structural/metadata field

Return format:
{{
  "<field_name>": {{"sensitivity_level": <0-4>, "data_domain": "<domain>", "interest_category": "<category>" | null}},
  ...
}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_field_name_and_description() {
        let prompt = build_classification_prompt("salary", "employee annual salary");
        assert!(prompt.contains("salary"));
        assert!(prompt.contains("employee annual salary"));
        assert!(prompt.contains("sensitivity_level"));
        assert!(prompt.contains("data_domain"));
    }

    #[test]
    fn prompt_lists_all_sensitivity_levels() {
        let prompt = build_classification_prompt("x", "y");
        for level in 0..=4 {
            assert!(prompt.contains(&format!("{} =", level)));
        }
    }

    #[test]
    fn prompt_lists_all_domains() {
        let prompt = build_classification_prompt("x", "y");
        for domain in &[
            "general",
            "financial",
            "medical",
            "identity",
            "behavioral",
            "location",
        ] {
            assert!(prompt.contains(domain));
        }
    }

    #[test]
    fn interest_prompt_contains_field_name_and_description() {
        let prompt =
            build_interest_category_prompt("photo_album", "the album containing the photo");
        assert!(prompt.contains("photo_album"));
        assert!(prompt.contains("the album containing the photo"));
        assert!(prompt.contains("interest_category"));
    }

    #[test]
    fn interest_prompt_lists_all_categories() {
        let prompt = build_interest_category_prompt("x", "y");
        for category in INTEREST_CATEGORIES {
            assert!(prompt.contains(category), "Missing category: {}", category);
        }
    }

    #[test]
    fn interest_categories_are_non_empty() {
        assert!(!INTEREST_CATEGORIES.is_empty());
        for cat in INTEREST_CATEGORIES {
            assert!(!cat.is_empty());
        }
    }
}
