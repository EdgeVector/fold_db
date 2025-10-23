use super::native_index_classification::{
    ClassificationRequest, ClassificationType, ExtractedEntity, FieldClassification, SplitStrategy,
};
use crate::ingestion::config::IngestionConfig;
use crate::schema::SchemaError;
// TODO: Full AI-driven classification will be implemented in a future update
// For now, we use heuristic-based classification

pub struct NativeIndexAIClassifier {
    #[allow(dead_code)] // TODO: Will be used for full AI classification
    config: IngestionConfig,
}

impl NativeIndexAIClassifier {
    pub fn new(config: IngestionConfig) -> Self {
        Self { config }
    }

    /// Classify a field using heuristics (AI classification is TODO)
    pub async fn classify_field(
        &self,
        request: ClassificationRequest,
    ) -> Result<FieldClassification, SchemaError> {
        // Use heuristic-based classification for now
        // TODO: Implement full AI classification
        Ok(self.classify_field_heuristic(&request.field_name, &request.sample_values))
    }

    /// Heuristic-based field classification
    fn classify_field_heuristic(
        &self,
        field_name: &str,
        _sample_values: &[String], // TODO: Will use sample values for better classification
    ) -> FieldClassification {
        let field_lower = field_name.to_lowercase();
        let mut classifications = Vec::new();
        let mut strategies = std::collections::HashMap::new();

        // Check for common field patterns
        if field_lower.contains("email") {
            classifications.push(ClassificationType::Email);
            strategies.insert(ClassificationType::Email, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("phone") || field_lower.contains("mobile") {
            classifications.push(ClassificationType::Phone);
            strategies.insert(ClassificationType::Phone, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("url") || field_lower.contains("link") || field_lower.contains("website") {
            classifications.push(ClassificationType::Url);
            strategies.insert(ClassificationType::Url, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("date") || field_lower.contains("time") || field_lower.contains("created") || field_lower.contains("updated") {
            classifications.push(ClassificationType::Date);
            strategies.insert(ClassificationType::Date, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("name") || field_lower.contains("author") || field_lower.contains("user") {
            classifications.push(ClassificationType::NamePerson);
            strategies.insert(ClassificationType::NamePerson, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("company") || field_lower.contains("organization") {
            classifications.push(ClassificationType::NameCompany);
            strategies.insert(ClassificationType::NameCompany, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("location") || field_lower.contains("city") || field_lower.contains("country") || field_lower.contains("place") {
            classifications.push(ClassificationType::NamePlace);
            strategies.insert(ClassificationType::NamePlace, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("tag") || field_lower.contains("hashtag") {
            classifications.push(ClassificationType::Hashtag);
            strategies.insert(ClassificationType::Hashtag, SplitStrategy::KeepWhole);
        }

        if field_lower.contains("username") || field_lower.contains("handle") {
            classifications.push(ClassificationType::Username);
            strategies.insert(ClassificationType::Username, SplitStrategy::KeepWhole);
        }

        // Always add word classification for general text search
        classifications.push(ClassificationType::Word);
        strategies.insert(ClassificationType::Word, SplitStrategy::SplitWords);

        FieldClassification {
            field_name: field_name.to_string(),
            classifications,
            strategies,
            entities: Vec::new(),
            cacheable: true,
        }
    }

    /// Extract entities from value (stub for now - will use AI in future)
    pub async fn extract_entities_from_value(
        &self,
        _value: &str,
        _classification: &ClassificationType,
    ) -> Result<Vec<ExtractedEntity>, SchemaError> {
        // TODO: Implement AI-based entity extraction
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_classification_email() {
        let config = IngestionConfig::default();
        let classifier = NativeIndexAIClassifier::new(config);

        let result = classifier.classify_field_heuristic("email", &[]);

        assert!(result.has_classification(&ClassificationType::Email));
        assert!(result.has_classification(&ClassificationType::Word));
    }

    #[test]
    fn test_heuristic_classification_author() {
        let config = IngestionConfig::default();
        let classifier = NativeIndexAIClassifier::new(config);

        let result = classifier.classify_field_heuristic("author", &[]);

        assert!(result.has_classification(&ClassificationType::NamePerson));
        assert!(result.has_classification(&ClassificationType::Word));
    }
}

