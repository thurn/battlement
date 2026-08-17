//! Strongly typed identifiers used by protocol records.

use std::{borrow::Cow, error::Error, fmt, str::FromStr};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use uuid::Uuid;

const UUID_PATTERN: &str = concat!(
    "^(?!00000000-0000-0000-0000-000000000000$)",
    "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
);

/// The reason a string could not be used as a Masonry identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdError {
    /// The value was not a lowercase, hyphenated UUID.
    InvalidFormat,
    /// Masonry reserves the all-zero UUID and does not accept it as an ID.
    Nil,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => formatter.write_str("expected a lowercase, hyphenated UUID"),
            Self::Nil => formatter.write_str("the all-zero UUID is not a valid Masonry ID"),
        }
    }
}

impl Error for IdError {}

fn parse_uuid(value: &str) -> Result<Uuid, IdError> {
    let uuid = Uuid::parse_str(value).map_err(|_| IdError::InvalidFormat)?;
    if uuid.is_nil() {
        return Err(IdError::Nil);
    }
    if uuid.hyphenated().to_string() != value {
        return Err(IdError::InvalidFormat);
    }
    Ok(uuid)
}

macro_rules! define_id {
    ($name:ident, $schema_name:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates an identifier from a UUID, rejecting the all-zero value.
            pub fn from_uuid(value: Uuid) -> Result<Self, IdError> {
                if value.is_nil() {
                    Err(IdError::Nil)
                } else {
                    Ok(Self(value))
                }
            }

            /// Generates a random version-4 identifier.
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4())
            }

            /// Returns the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Consumes this value and returns the underlying UUID.
            #[must_use]
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                parse_uuid(value).map(Self)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct IdVisitor;

                impl de::Visitor<'_> for IdVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("a nonzero lowercase, hyphenated UUID")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        value.parse().map_err(E::custom)
                    }
                }

                deserializer.deserialize_str(IdVisitor)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed($schema_name)
            }

            fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
                json_schema!({
                    "type": "string",
                    "format": "uuid",
                    "pattern": UUID_PATTERN,
                })
            }
        }
    };
}

define_id!(
    SessionId,
    "SessionId",
    "Identifies one connection or reconnection session. IDs are lowercase, hyphenated, and nonzero."
);
define_id!(
    ActionId,
    "ActionId",
    "Identifies one client action for session-wide duplicate detection. IDs are lowercase, hyphenated, and nonzero."
);
define_id!(
    BatchId,
    "BatchId",
    "Identifies one ordered batch of commands. IDs are lowercase, hyphenated, and nonzero."
);
define_id!(
    CommandId,
    "CommandId",
    "Identifies one command and, when the command starts asynchronous work, that operation. IDs are lowercase, hyphenated, and nonzero."
);
define_id!(
    ObjectId,
    "ObjectId",
    "Identifies one runtime object root in a session. IDs are lowercase, hyphenated, and nonzero."
);
define_id!(
    SceneId,
    "SceneId",
    "Identifies one loaded content-scene instance. IDs are lowercase, hyphenated, and nonzero."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_require_the_canonical_wire_format() {
        let valid = "94fa422b-301d-442d-b9a7-10ea54318e78";
        assert_eq!(valid.parse::<SessionId>().unwrap().to_string(), valid);
        assert_eq!(
            valid.to_uppercase().parse::<SessionId>(),
            Err(IdError::InvalidFormat)
        );
        assert_eq!(
            "94fa422b301d442db9a710ea54318e78".parse::<SessionId>(),
            Err(IdError::InvalidFormat)
        );
        assert_eq!(
            "00000000-0000-0000-0000-000000000000".parse::<SessionId>(),
            Err(IdError::Nil)
        );
    }
}
