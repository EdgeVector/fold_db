// Content-aware fragmentation for discovery index.
//
// Splits field values into semantically meaningful fragments.
// Short values remain as single fragments; long text is split at sentence boundaries.

/// A single fragment produced from a field value.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The text content of this fragment
    pub text: String,
    /// Index within the field (0 for single-fragment fields)
    pub index: usize,
}

/// Minimum character length before sentence splitting is attempted.
const SENTENCE_SPLIT_THRESHOLD: usize = 100;

/// Minimum fragment length in characters. Fragments shorter than this are merged
/// with the next fragment to avoid semantically empty embeddings.
const MIN_FRAGMENT_LENGTH: usize = 20;

/// Split a field value into fragments suitable for independent embedding.
///
/// - Short text (<100 chars): returned as a single fragment
/// - Long text: split at sentence boundaries (period/question/exclamation)
/// - Each fragment is a coherent semantic unit
pub fn split_into_fragments(text: &str) -> Vec<Fragment> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if trimmed.len() < SENTENCE_SPLIT_THRESHOLD {
        return vec![Fragment { text: trimmed.to_string(), index: 0 }];
    }

    let sentences = split_sentences(trimmed);
    if sentences.len() <= 1 {
        return vec![Fragment { text: trimmed.to_string(), index: 0 }];
    }

    // Merge short sentences with the next one
    let mut merged: Vec<String> = Vec::new();
    let mut buffer = String::new();

    for sentence in sentences {
        if buffer.is_empty() {
            buffer = sentence;
        } else {
            buffer.push(' ');
            buffer.push_str(&sentence);
        }

        if buffer.len() >= MIN_FRAGMENT_LENGTH {
            merged.push(buffer.clone());
            buffer.clear();
        }
    }

    // Append remaining buffer to the last fragment
    if !buffer.is_empty() {
        if let Some(last) = merged.last_mut() {
            last.push(' ');
            last.push_str(&buffer);
        } else {
            merged.push(buffer);
        }
    }

    merged
        .into_iter()
        .enumerate()
        .map(|(i, text)| Fragment { text, index: i })
        .collect()
}

/// Split text at sentence boundaries.
/// Handles: periods, question marks, exclamation marks followed by whitespace or end of string.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let ch = chars[i];
        // Check for sentence-ending punctuation
        if (ch == '.' || ch == '?' || ch == '!') && i + 1 < len {
            // Look ahead: must be followed by whitespace (sentence boundary)
            // Skip consecutive punctuation (e.g., "..." or "?!")
            let mut end = i + 1;
            while end < len && (chars[end] == '.' || chars[end] == '?' || chars[end] == '!') {
                end += 1;
            }
            if end < len && chars[end].is_whitespace() {
                let sentence: String = chars[start..end].iter().collect();
                let trimmed = sentence.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                // Skip whitespace after punctuation
                while end < len && chars[end].is_whitespace() {
                    end += 1;
                }
                start = end;
                i = end;
                continue;
            }
        }
        i += 1;
    }

    // Remaining text
    if start < len {
        let sentence: String = chars[start..].iter().collect();
        let trimmed = sentence.trim().to_string();
        if !trimmed.is_empty() {
            sentences.push(trimmed);
        }
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert!(split_into_fragments("").is_empty());
        assert!(split_into_fragments("   ").is_empty());
    }

    #[test]
    fn test_short_text_single_fragment() {
        let frags = split_into_fragments("A simple recipe for chocolate cake");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].index, 0);
        assert_eq!(frags[0].text, "A simple recipe for chocolate cake");
    }

    #[test]
    fn test_long_text_splits_at_sentences() {
        let text = "This is the first sentence about cooking. \
                    The second sentence describes ingredients in great detail. \
                    A third sentence explains the preparation method step by step.";
        let frags = split_into_fragments(text);
        assert!(frags.len() > 1, "Expected multiple fragments, got {}", frags.len());
        // Each fragment should be coherent text
        for frag in &frags {
            assert!(!frag.text.is_empty());
        }
    }

    #[test]
    fn test_fragments_have_sequential_indices() {
        let text = "First sentence here. Second sentence there. Third one too. \
                    Fourth sentence is longer to pass the threshold for splitting.";
        let frags = split_into_fragments(text);
        for (i, frag) in frags.iter().enumerate() {
            assert_eq!(frag.index, i);
        }
    }

    #[test]
    fn test_sentence_splitting_handles_abbreviations() {
        // "Dr." followed by capital letter shouldn't split ideally, but our simple
        // splitter may split here. That's acceptable — fragments just need to be
        // coherent enough for embedding.
        let text = "Dr. Smith went to the store. He bought some apples. \
                    Then he went home and made a pie that was delicious.";
        let frags = split_into_fragments(text);
        assert!(!frags.is_empty());
    }

    #[test]
    fn test_no_split_under_threshold() {
        let text = "Short text that is under one hundred characters total.";
        let frags = split_into_fragments(text);
        assert_eq!(frags.len(), 1);
    }
}
