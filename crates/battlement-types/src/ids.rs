//! Strongly typed identifiers shared by protocol domains.

use std::{error::Error, fmt, marker::PhantomData, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use uuid::Uuid;

/// The reason a string could not be used as a Battlement identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdError {
    /// The value was not a UUID.
    InvalidFormat,
    /// Battlement reserves the all-zero UUID and does not accept it as an ID.
    Nil,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => formatter.write_str("expected a UUID"),
            Self::Nil => formatter.write_str("the all-zero UUID is not a valid Battlement ID"),
        }
    }
}

impl Error for IdError {}

/// A nonzero UUID tagged at the type level with its protocol role.
///
/// Use the role-specific aliases such as [`SessionId`] and [`ObjectId`] in
/// public APIs. The generic representation centralizes parsing, serialization,
/// and UUID conversion without making identifiers for different protocol roles
/// interchangeable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolId<K> {
    uuid: Uuid,
    kind: PhantomData<fn() -> K>,
}

impl<K> ProtocolId<K> {
    /// Creates an identifier from a UUID, rejecting the all-zero value.
    pub const fn from_uuid(uuid: Uuid) -> Result<Self, IdError> {
        if uuid.is_nil() {
            Err(IdError::Nil)
        } else {
            Ok(Self::from_non_nil_uuid(uuid))
        }
    }

    /// Generates a random version-4 identifier.
    #[must_use]
    pub fn new_v4() -> Self {
        Self::from_non_nil_uuid(Uuid::new_v4())
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.uuid
    }

    /// Consumes this value and returns the underlying UUID.
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.uuid
    }

    const fn from_non_nil_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            kind: PhantomData,
        }
    }
}

impl<K> fmt::Display for ProtocolId<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.uuid.hyphenated().fmt(formatter)
    }
}

impl<K> FromStr for ProtocolId<K> {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map_err(|_| IdError::InvalidFormat)
            .and_then(Self::from_uuid)
    }
}

impl<K> TryFrom<&str> for ProtocolId<K> {
    type Error = IdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<K> TryFrom<String> for ProtocolId<K> {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<K> From<ProtocolId<K>> for Uuid {
    fn from(value: ProtocolId<K>) -> Self {
        value.uuid
    }
}

impl<K> Serialize for ProtocolId<K> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.uuid.serialize(serializer)
    }
}

impl<'de, K> Deserialize<'de> for ProtocolId<K> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_uuid(Uuid::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

mod kind {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Session;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Action;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Batch;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Command;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Object;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Scene;
}

/// Identifies one connection or reconnection session.
pub type SessionId = ProtocolId<kind::Session>;
/// Identifies one client action for session-wide duplicate detection.
pub type ActionId = ProtocolId<kind::Action>;
/// Identifies one ordered batch of commands.
pub type BatchId = ProtocolId<kind::Batch>;
/// Identifies one command and any operation started by that command.
pub type CommandId = ProtocolId<kind::Command>;
/// Identifies one game object in a session.
pub type ObjectId = ProtocolId<kind::Object>;
/// Identifies one loaded content-scene instance.
pub type SceneId = ProtocolId<kind::Scene>;

/// Creates a constant [`ObjectId`] from a UUID literal.
///
/// Invalid or nil UUID literals fail during compilation when used in a constant.
#[macro_export]
macro_rules! object_id {
    ($value:literal) => {{
        const UUID: $crate::__private::Uuid = $crate::__private::uuid!($value);
        match $crate::ObjectId::from_uuid(UUID) {
            Ok(id) => id,
            Err(_) => panic!("object ID must not be nil"),
        }
    }};
}

/// Creates a constant [`SceneId`] from a UUID literal.
///
/// Invalid or nil UUID literals fail during compilation when used in a constant.
#[macro_export]
macro_rules! scene_id {
    ($value:literal) => {{
        const UUID: $crate::__private::Uuid = $crate::__private::uuid!($value);
        match $crate::SceneId::from_uuid(UUID) {
            Ok(id) => id,
            Err(_) => panic!("scene ID must not be nil"),
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_accept_uuid_text_and_reject_nil() {
        let valid = "94fa422b-301d-442d-b9a7-10ea54318e78";
        assert_eq!(valid.parse::<SessionId>().unwrap().to_string(), valid);
        assert!(valid.to_uppercase().parse::<SessionId>().is_ok());
        assert_eq!(
            Uuid::nil().to_string().parse::<SessionId>(),
            Err(IdError::Nil)
        );
    }
}
