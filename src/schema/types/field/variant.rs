use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::{
    Field, FieldCommon, FieldType, RangeField, SingleField,
};
use crate::schema::types::Transform;

/// Enumeration over all field variants.
#[derive(Debug, Clone)]
pub enum FieldVariant {
    /// Single value field
    Single(SingleField),
    /// Range of values
    Range(RangeField),
}

impl Field for FieldVariant {
    fn permission_policy(&self) -> &PermissionsPolicy {
        match self {
            Self::Single(f) => f.permission_policy(),
            Self::Range(f) => f.permission_policy(),
        }
    }

    fn payment_config(&self) -> &FieldPaymentConfig {
        match self {
            Self::Single(f) => f.payment_config(),
            Self::Range(f) => f.payment_config(),
        }
    }

    fn ref_atom_uuid(&self) -> Option<&String> {
        match self {
            Self::Single(f) => f.ref_atom_uuid(),
            Self::Range(f) => f.ref_atom_uuid(),
        }
    }

    fn set_ref_atom_uuid(&mut self, uuid: String) {
        match self {
            Self::Single(f) => f.set_ref_atom_uuid(uuid),
            Self::Range(f) => f.set_ref_atom_uuid(uuid),
        }
    }

    fn field_mappers(&self) -> &HashMap<String, String> {
        match self {
            Self::Single(f) => f.field_mappers(),
            Self::Range(f) => f.field_mappers(),
        }
    }

    fn set_field_mappers(&mut self, mappers: HashMap<String, String>) {
        match self {
            Self::Single(f) => f.set_field_mappers(mappers),
            Self::Range(f) => f.set_field_mappers(mappers),
        }
    }

    fn transform(&self) -> Option<&Transform> {
        match self {
            Self::Single(f) => f.transform(),
            Self::Range(f) => f.transform(),
        }
    }

    fn set_transform(&mut self, transform: Transform) {
        match self {
            Self::Single(f) => f.set_transform(transform),
            Self::Range(f) => f.set_transform(transform),
        }
    }

    fn writable(&self) -> bool {
        match self {
            Self::Single(f) => f.writable(),
            Self::Range(f) => f.writable(),
        }
    }

    fn set_writable(&mut self, writable: bool) {
        match self {
            Self::Single(f) => f.set_writable(writable),
            Self::Range(f) => f.set_writable(writable),
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
        }

        let helper = match self {
            Self::Single(f) => Helper {
                inner: &f.inner,
                field_type: FieldType::Single,
            },
            Self::Range(f) => Helper {
                inner: &f.inner,
                field_type: FieldType::Range,
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
                // If there's a ref_atom_uuid, we need to initialize the atom_ref_range
                if let Some(_ref_atom_uuid) = range_field.inner.ref_atom_uuid.as_ref() {
                    // We'll initialize it with an empty pub key for now - it will be populated when data is loaded
                    range_field.molecule_range =
                        Some(crate::atom::MoleculeRange::new(String::new()));
                }
                Self::Range(range_field)
            }
        })
    }
}
