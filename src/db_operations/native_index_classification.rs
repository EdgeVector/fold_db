use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Classification type for native index entries
/// These classify the actual content of field values (e.g., email addresses, phone numbers)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassificationType {
    /// General word-based search
    Word,
    /// Person name (e.g., "Jennifer Liu")
    NamePerson,
    /// Company/organization name (e.g., "Apple Inc")
    NameCompany,
    /// Place/location name (e.g., "San Francisco")
    NamePlace,
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// URL/domain
    Url,
    /// Date
    Date,
    /// Hashtag
    Hashtag,
    /// Username/handle (e.g., @username)
    Username,
    /// Custom classification type
    Custom(String),
}

impl ClassificationType {
    /// Get the prefix string for index keys
    pub fn prefix(&self) -> String {
        match self {
            ClassificationType::Word => "word".to_string(),
            ClassificationType::NamePerson => "name:person".to_string(),
            ClassificationType::NameCompany => "name:company".to_string(),
            ClassificationType::NamePlace => "name:place".to_string(),
            ClassificationType::Email => "email".to_string(),
            ClassificationType::Phone => "phone".to_string(),
            ClassificationType::Url => "url".to_string(),
            ClassificationType::Date => "date".to_string(),
            ClassificationType::Hashtag => "hashtag".to_string(),
            ClassificationType::Username => "username".to_string(),
            ClassificationType::Custom(name) => name.clone(),
        }
    }

    /// Parse a classification type from a prefix string
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "word" => Some(ClassificationType::Word),
            "name:person" => Some(ClassificationType::NamePerson),
            "name:company" => Some(ClassificationType::NameCompany),
            "name:place" => Some(ClassificationType::NamePlace),
            "email" => Some(ClassificationType::Email),
            "phone" => Some(ClassificationType::Phone),
            "url" => Some(ClassificationType::Url),
            "date" => Some(ClassificationType::Date),
            "hashtag" => Some(ClassificationType::Hashtag),
            "username" => Some(ClassificationType::Username),
            _ => Some(ClassificationType::Custom(prefix.to_string())),
        }
    }
}

/// Strategy for splitting/processing field values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SplitStrategy {
    /// Keep the entire value as one entity
    KeepWhole,
    /// Split into individual words
    SplitWords,
    /// Extract named entities using AI
    ExtractEntities,
}

/// An extracted entity from a field value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// The entity value (e.g., "Jennifer Liu")
    pub value: String,
    /// The classification type
    pub classification: ClassificationType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

/// Result of AI classification for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldClassification {
    /// Field name
    pub field_name: String,
    /// Classification types to apply (can be multiple)
    pub classifications: Vec<ClassificationType>,
    /// Split strategies per classification type
    pub strategies: HashMap<ClassificationType, SplitStrategy>,
    /// Extracted entities (if using ExtractEntities strategy)
    pub entities: Vec<ExtractedEntity>,
    /// Whether this classification should be cached
    pub cacheable: bool,
}

impl FieldClassification {
    /// Create a simple word-based classification (fallback)
    pub fn word_only(field_name: String) -> Self {
        let mut strategies = HashMap::new();
        strategies.insert(ClassificationType::Word, SplitStrategy::SplitWords);

        Self {
            field_name,
            classifications: vec![ClassificationType::Word],
            strategies,
            entities: Vec::new(),
            cacheable: true,
        }
    }

    /// Check if this classification uses a specific type
    pub fn has_classification(&self, classification: &ClassificationType) -> bool {
        self.classifications.contains(classification)
    }

    /// Get the strategy for a specific classification
    pub fn get_strategy(&self, classification: &ClassificationType) -> Option<&SplitStrategy> {
        self.strategies.get(classification)
    }
}

/// Request to classify a field using AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRequest {
    /// Schema name
    pub schema_name: String,
    /// Field name
    pub field_name: String,
    /// Sample values from the field (for context)
    pub sample_values: Vec<String>,
}

/// Cache key for field classifications
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClassificationCacheKey {
    pub schema_name: String,
    pub field_name: String,
}

impl ClassificationCacheKey {
    pub fn new(schema_name: String, field_name: String) -> Self {
        Self {
            schema_name,
            field_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_type_prefix() {
        assert_eq!(ClassificationType::Word.prefix(), "word");
        assert_eq!(ClassificationType::NamePerson.prefix(), "name:person");
        assert_eq!(ClassificationType::Email.prefix(), "email");
    }

    #[test]
    fn test_classification_type_from_prefix() {
        assert_eq!(
            ClassificationType::from_prefix("word"),
            Some(ClassificationType::Word)
        );
        assert_eq!(
            ClassificationType::from_prefix("name:person"),
            Some(ClassificationType::NamePerson)
        );
        assert_eq!(
            ClassificationType::from_prefix("email"),
            Some(ClassificationType::Email)
        );
    }

    #[test]
    fn test_word_only_classification() {
        let classification = FieldClassification::word_only("test_field".to_string());
        assert_eq!(classification.classifications.len(), 1);
        assert!(classification.has_classification(&ClassificationType::Word));
        assert_eq!(
            classification.get_strategy(&ClassificationType::Word),
            Some(&SplitStrategy::SplitWords)
        );
    }
}
