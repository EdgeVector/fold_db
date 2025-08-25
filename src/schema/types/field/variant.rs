use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::{
    Field, FieldCommon, FieldType, HashRangeField, RangeField, SingleField,
};
use crate::schema::types::Transform;

/// Enumeration over all field variants.
#[derive(Debug, Clone)]
pub enum FieldVariant {
    /// Single value field
    Single(SingleField),
    /// Range of values
    Range(RangeField),
    /// Hash-range field for complex indexing
    HashRange(HashRangeField),
}

impl Field for FieldVariant {
    fn permission_policy(&self) -> &PermissionsPolicy {
        match self {
            Self::Single(f) => f.permission_policy(),
            Self::Range(f) => f.permission_policy(),
            Self::HashRange(f) => f.permission_policy(),
        }
    }

    fn payment_config(&self) -> &FieldPaymentConfig {
        match self {
            Self::Single(f) => f.payment_config(),
            Self::Range(f) => f.payment_config(),
            Self::HashRange(f) => f.payment_config(),
        }
    }

    fn molecule_uuid(&self) -> Option<&String> {
        match self {
            Self::Single(f) => f.molecule_uuid(),
            Self::Range(f) => f.molecule_uuid(),
            Self::HashRange(f) => f.molecule_uuid(),
        }
    }

    fn set_molecule_uuid(&mut self, uuid: String) {
        match self {
            Self::Single(f) => f.set_molecule_uuid(uuid),
            Self::Range(f) => f.set_molecule_uuid(uuid),
            Self::HashRange(f) => f.set_molecule_uuid(uuid),
        }
    }

    fn field_mappers(&self) -> &HashMap<String, String> {
        match self {
            Self::Single(f) => f.field_mappers(),
            Self::Range(f) => f.field_mappers(),
            Self::HashRange(f) => f.field_mappers(),
        }
    }

    fn set_field_mappers(&mut self, mappers: HashMap<String, String>) {
        match self {
            Self::Single(f) => f.set_field_mappers(mappers),
            Self::Range(f) => f.set_field_mappers(mappers),
            Self::HashRange(f) => f.set_field_mappers(mappers),
        }
    }

    fn transform(&self) -> Option<&Transform> {
        match self {
            Self::Single(f) => f.transform(),
            Self::Range(f) => f.transform(),
            Self::HashRange(f) => f.transform(),
        }
    }

    fn set_transform(&mut self, transform: Transform) {
        match self {
            Self::Single(f) => f.set_transform(transform),
            Self::Range(f) => f.set_transform(transform),
            Self::HashRange(f) => f.set_transform(transform),
        }
    }

    fn writable(&self) -> bool {
        match self {
            Self::Single(f) => f.writable(),
            Self::Range(f) => f.writable(),
            Self::HashRange(f) => f.writable(),
        }
    }

    fn set_writable(&mut self, writable: bool) {
        match self {
            Self::Single(f) => f.set_writable(writable),
            Self::Range(f) => f.set_writable(writable),
            Self::HashRange(f) => f.set_writable(writable),
        }
    }
}

impl Serialize for FieldVariant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a> {
            #[serde(flatten)]
            inner: &'a FieldCommon,
            field_type: FieldType,
            #[serde(skip_serializing_if = "Option::is_none")]
            hash_field: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            range_field: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            atom_uuid: Option<&'a String>,
        }

        let helper = match self {
            Self::Single(f) => Helper {
                inner: &f.inner,
                field_type: FieldType::Single,
                hash_field: None,
                range_field: None,
                atom_uuid: None,
            },
            Self::Range(f) => Helper {
                inner: &f.inner,
                field_type: FieldType::Range,
                hash_field: None,
                range_field: None,
                atom_uuid: None,
            },
            Self::HashRange(f) => Helper {
                inner: &f.inner,
                field_type: FieldType::HashRange,
                hash_field: Some(&f.hash_field),
                range_field: Some(&f.range_field),
                atom_uuid: Some(&f.atom_uuid),
            },
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FieldVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(flatten)]
            inner: FieldCommon,
            field_type: Option<FieldType>,
            hash_field: Option<String>,
            range_field: Option<String>,
            atom_uuid: Option<String>,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(match helper.field_type.unwrap_or(FieldType::Single) {
            FieldType::Single => Self::Single(SingleField {
                inner: helper.inner,
            }),
            FieldType::Range => {
                let mut range_field = RangeField {
                    inner: helper.inner,
                    molecule_range: None,
                };
                // If there's a molecule_uuid, we need to initialize the molecule_range
                if let Some(_molecule_uuid) = range_field.inner.molecule_uuid.as_ref() {
                    // We'll initialize it with an empty pub key for now - it will be populated when data is loaded
                    range_field.molecule_range =
                        Some(crate::atom::MoleculeRange::new(String::new()));
                }
                Self::Range(range_field)
            }
            FieldType::HashRange => {
                let hash_field = helper.hash_field.ok_or_else(|| {
                    serde::de::Error::missing_field("hash_field")
                })?;
                let range_field = helper.range_field.ok_or_else(|| {
                    serde::de::Error::missing_field("range_field")
                })?;
                let atom_uuid = helper.atom_uuid.ok_or_else(|| {
                    serde::de::Error::missing_field("atom_uuid")
                })?;

                Self::HashRange(HashRangeField {
                    inner: helper.inner,
                    hash_field,
                    range_field,
                    atom_uuid,
                    cached_chains: None,
                })
            }
        })
    }
}
