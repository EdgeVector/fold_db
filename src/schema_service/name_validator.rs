//! Generic schema name detection and rejection.
//!
//! Schema descriptive names must describe the *content topic* — e.g.
//! "Family Vacation Photos" or "Technical Architecture Notes" — not
//! structural terms like "Document Collection" or "Data Records".
//!
//! Used by both the ingestion pipeline (to re-prompt the AI) and the
//! schema service (to reject at registration time).

use std::collections::HashSet;

/// Words that describe structure/format rather than content.
/// A name composed entirely of these words is generic.
/// Words that describe structure, format, or container type rather than content.
/// A name composed entirely of these words is generic and will be rejected.
/// Names must have at least one content-specific word (e.g., "Family" in
/// "Family Photos" or "Concert" in "Concert Videos").
const GENERIC_WORDS: &[&str] = &[
    // Container / structure words
    "album",
    "archive",
    "catalog",
    "catalogue",
    "collection",
    "database",
    "entries",
    "entry",
    "gallery",
    "item",
    "items",
    "library",
    "list",
    "log",
    "metadata",
    "object",
    "objects",
    "record",
    "records",
    "repository",
    "set",
    "store",
    // Format / media words (describe the file type, not the content topic)
    "audio",
    "content",
    "data",
    "document",
    "documents",
    "file",
    "files",
    "image",
    "images",
    "photo",
    "photograph",
    "photographs",
    "photos",
    "picture",
    "pictures",
    "text",
    "video",
    "videos",
    // Filler words
    "general",
    "generic",
    "information",
    "misc",
    "miscellaneous",
    "mixed",
    "my",
    "other",
    // Stop words
    "the",
    "with",
    "and",
    "of",
    "a",
    "an",
];

/// Returns `true` if the name is too generic to be useful as a schema name.
///
/// A name is generic if **every meaningful word** (after lowercasing) is in
/// the [`GENERIC_WORDS`] set. Names with at least one content-specific word
/// pass — e.g. "Medical Records" passes because "medical" is specific.
///
/// # Examples
///
/// ```
/// use fold_db::schema_service::name_validator::is_generic_name;
///
/// assert!(is_generic_name("Document Collection"));
/// assert!(is_generic_name("Data Records"));
/// assert!(is_generic_name("text content"));
///
/// assert!(!is_generic_name("Family Vacation Photos"));
/// assert!(!is_generic_name("Medical Records"));
/// assert!(!is_generic_name("Technical Notes"));
/// ```
pub fn is_generic_name(name: &str) -> bool {
    let generic_set: HashSet<&str> = GENERIC_WORDS.iter().copied().collect();

    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() {
        return true;
    }

    // Every word (lowercased) must be generic for the name to be rejected
    words
        .iter()
        .all(|w| generic_set.contains(w.to_lowercase().as_str()))
}

/// Returns `Err` with a descriptive message if the name is generic.
///
/// The error message is designed to be included in AI retry prompts.
pub fn reject_generic_name(name: &str) -> Result<(), String> {
    if is_generic_name(name) {
        Err(format!(
            "Schema descriptive_name '{}' is too generic. \
             The name must describe the CONTENT TOPIC — read the actual data and name it \
             specifically (e.g., 'Family Vacation Photos', 'Technical Architecture Notes', \
             'Weekly Meeting Minutes').",
            name
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Names that SHOULD be rejected (generic) ---

    #[test]
    fn reject_document_collection() {
        assert!(is_generic_name("Document Collection"));
    }

    #[test]
    fn reject_data_records() {
        assert!(is_generic_name("Data Records"));
    }

    #[test]
    fn reject_text_content() {
        assert!(is_generic_name("Text Content"));
    }

    #[test]
    fn reject_document_entry_collection() {
        assert!(is_generic_name("Document Entry Collection"));
    }

    #[test]
    fn reject_file_metadata() {
        assert!(is_generic_name("File Metadata"));
    }

    #[test]
    fn reject_record_list() {
        assert!(is_generic_name("Record List"));
    }

    #[test]
    fn reject_general_information() {
        assert!(is_generic_name("General Information"));
    }

    #[test]
    fn reject_document_records_with_metadata() {
        assert!(is_generic_name("Document Records with Metadata"));
    }

    #[test]
    fn reject_data_entries() {
        assert!(is_generic_name("Data Entries"));
    }

    #[test]
    fn reject_mixed_content_collection() {
        assert!(is_generic_name("Mixed Content Collection"));
    }

    #[test]
    fn reject_the_document_collection() {
        assert!(is_generic_name("The Document Collection"));
    }

    #[test]
    fn reject_empty_string() {
        assert!(is_generic_name(""));
    }

    // --- Names that SHOULD be accepted (content-specific) ---

    #[test]
    fn accept_technical_architecture_notes() {
        assert!(!is_generic_name("Technical Architecture Notes"));
    }

    #[test]
    fn accept_personal_journal_entries() {
        assert!(!is_generic_name("Personal Journal Entries"));
    }

    #[test]
    fn accept_medical_records() {
        assert!(!is_generic_name("Medical Records"));
    }

    #[test]
    fn accept_cooking_recipes() {
        assert!(!is_generic_name("Cooking Recipes"));
    }

    #[test]
    fn accept_meeting_notes_q1() {
        assert!(!is_generic_name("Meeting Notes Q1 2026"));
    }

    #[test]
    fn accept_tax_documents_2025() {
        assert!(!is_generic_name("Tax Documents 2025"));
    }

    #[test]
    fn accept_blog_posts() {
        assert!(!is_generic_name("Blog Posts"));
    }

    #[test]
    fn accept_customer_orders() {
        assert!(!is_generic_name("Customer Orders"));
    }

    #[test]
    fn accept_workout_log() {
        assert!(!is_generic_name("Workout Log"));
    }

    #[test]
    fn reject_photo_collection() {
        assert!(is_generic_name("Photo Collection"));
    }

    #[test]
    fn reject_image_collection() {
        assert!(is_generic_name("Image Collection"));
    }

    #[test]
    fn reject_image_library() {
        assert!(is_generic_name("Image Library"));
    }

    #[test]
    fn reject_photo_gallery() {
        assert!(is_generic_name("Photo Gallery"));
    }

    #[test]
    fn reject_photo_album() {
        assert!(is_generic_name("Photo Album"));
    }

    #[test]
    fn reject_video_files() {
        assert!(is_generic_name("Video Files"));
    }

    #[test]
    fn reject_audio_collection() {
        assert!(is_generic_name("Audio Collection"));
    }

    #[test]
    fn reject_image_data() {
        assert!(is_generic_name("Image Data"));
    }

    #[test]
    fn reject_my_photos() {
        assert!(is_generic_name("My Photos"));
    }

    #[test]
    fn reject_picture_archive() {
        assert!(is_generic_name("Picture Archive"));
    }

    #[test]
    fn accept_family_vacation_photos() {
        // "Family" and "Vacation" are content-specific
        assert!(!is_generic_name("Family Vacation Photos"));
    }

    #[test]
    fn accept_concert_videos() {
        // "Concert" is content-specific
        assert!(!is_generic_name("Concert Videos"));
    }

    #[test]
    fn accept_podcast_audio() {
        // "Podcast" is content-specific
        assert!(!is_generic_name("Podcast Audio"));
    }

    #[test]
    fn accept_landscape_paintings() {
        assert!(!is_generic_name("Landscape Paintings"));
    }

    #[test]
    fn accept_email_archive() {
        // "Email" is content-specific
        assert!(!is_generic_name("Email Archive"));
    }

    #[test]
    fn accept_architectural_diagrams() {
        assert!(!is_generic_name("Architectural Diagrams"));
    }

    #[test]
    fn accept_financial_transactions() {
        assert!(!is_generic_name("Financial Transactions"));
    }

    #[test]
    fn accept_travel_itinerary() {
        assert!(!is_generic_name("Travel Itinerary"));
    }

    // --- reject_generic_name function ---

    #[test]
    fn reject_returns_error_for_generic() {
        let result = reject_generic_name("Document Collection");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too generic"));
    }

    #[test]
    fn reject_returns_ok_for_specific() {
        assert!(reject_generic_name("Family Vacation Photos").is_ok());
    }
}
