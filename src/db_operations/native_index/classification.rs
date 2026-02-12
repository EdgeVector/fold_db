use serde::{Deserialize, Serialize};

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
}
