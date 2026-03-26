use serde_json::Value;

/// Split text into sentence-level fragments for independent embedding.
/// Short text (<100 chars) is returned as a single fragment.
/// Long text is split on sentence boundaries (`.` `!` `?` followed by whitespace).
pub fn split_into_fragments(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.len() < 100 {
        return vec![trimmed.to_string()];
    }
    split_sentences(trimmed)
}

/// Split text on sentence-ending punctuation followed by whitespace.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();

    for i in 0..chars.len() {
        let is_sentence_end = matches!(chars[i], '.' | '!' | '?');
        let followed_by_space = i + 1 < chars.len() && chars[i + 1].is_whitespace();
        let is_last_char = i + 1 == chars.len();

        if is_sentence_end && (followed_by_space || is_last_char) {
            let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
            let byte_end = chars[..=i].iter().map(|c| c.len_utf8()).sum::<usize>();
            let sentence = text[byte_start..byte_end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            // Skip whitespace after sentence
            let mut next = i + 1;
            while next < chars.len() && chars[next].is_whitespace() {
                next += 1;
            }
            start = next;
        }
    }

    // Remaining text that didn't end with sentence punctuation
    if start < chars.len() {
        let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
        let remaining = text[byte_start..].trim();
        if !remaining.is_empty() {
            sentences.push(remaining.to_string());
        }
    }

    // If no splits were found, return original as single fragment
    if sentences.is_empty() {
        sentences.push(text.trim().to_string());
    }

    sentences
}

/// Convert a JSON value into fragments for embedding.
/// Strings are split into sentences. Arrays are split per-element.
/// Objects have their values concatenated. Numbers/bools are single fragments.
pub fn value_to_fragments(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => split_into_fragments(s),
        Value::Number(n) => vec![n.to_string()],
        Value::Bool(b) => vec![b.to_string()],
        Value::Array(arr) => arr.iter().flat_map(value_to_fragments).collect(),
        Value::Object(obj) => {
            let combined: String = obj
                .values()
                .map(value_to_text)
                .collect::<Vec<_>>()
                .join(" ");
            split_into_fragments(&combined)
        }
        Value::Null => Vec::new(),
    }
}

/// Convert a value to plain text (for combining object fields).
fn value_to_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => arr.iter().map(value_to_text).collect::<Vec<_>>().join(" "),
        Value::Object(obj) => obj
            .values()
            .map(value_to_text)
            .collect::<Vec<_>>()
            .join(" "),
        Value::Null => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_single_fragment() {
        let frags = split_into_fragments("Hello world");
        assert_eq!(frags, vec!["Hello world"]);
    }

    #[test]
    fn test_empty_text_no_fragments() {
        assert!(split_into_fragments("").is_empty());
        assert!(split_into_fragments("   ").is_empty());
    }

    #[test]
    fn test_long_text_splits_on_sentences() {
        let text = "The quick brown fox jumps over the lazy dog. \
                     The dog barked loudly at the fox. \
                     Then they became friends and lived happily ever after.";
        let frags = split_into_fragments(text);
        assert_eq!(frags.len(), 3);
        assert!(frags[0].starts_with("The quick"));
        assert!(frags[1].starts_with("The dog"));
        assert!(frags[2].starts_with("Then they"));
    }

    #[test]
    fn test_exclamation_and_question_marks() {
        let text = "What is happening here in this very long sentence that keeps going and going? I don't know what to make of all this! But it seems like something important is going on right now.";
        let frags = split_into_fragments(text);
        assert_eq!(frags.len(), 3);
    }

    #[test]
    fn test_no_sentence_boundary_returns_whole() {
        let text = "This is a long piece of text without any sentence-ending punctuation that goes on and on and on and on";
        let frags = split_into_fragments(text);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], text);
    }

    #[test]
    fn test_value_to_fragments_string() {
        let v = serde_json::json!("short text");
        let frags = value_to_fragments(&v);
        assert_eq!(frags, vec!["short text"]);
    }

    #[test]
    fn test_value_to_fragments_array() {
        let v = serde_json::json!(["hello", "world"]);
        let frags = value_to_fragments(&v);
        assert_eq!(frags, vec!["hello", "world"]);
    }

    #[test]
    fn test_value_to_fragments_null() {
        let frags = value_to_fragments(&Value::Null);
        assert!(frags.is_empty());
    }

    #[test]
    fn test_value_to_fragments_number() {
        let frags = value_to_fragments(&serde_json::json!(42));
        assert_eq!(frags, vec!["42"]);
    }
}
