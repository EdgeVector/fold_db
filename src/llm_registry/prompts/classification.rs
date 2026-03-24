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
        for domain in &["general", "financial", "medical", "identity", "behavioral", "location"] {
            assert!(prompt.contains(domain));
        }
    }
}
