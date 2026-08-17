//! Strongly typed identifiers used by protocol records.

use std::{borrow::Cow, error::Error, fmt, marker::PhantomData, str::FromStr};

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::{Map, Value};
use uuid::Uuid;

const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";
const UUID_PATTERN: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

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

/// A nonzero UUID tagged at the type level with its protocol role.
///
/// Use the role-specific aliases such as [`SessionId`] and [`ObjectId`] in
/// public APIs. The generic representation centralizes canonical parsing,
/// serialization, UUID conversion, and JSON Schema generation without making
/// identifiers for different protocol roles interchangeable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolId<K> {
    uuid: Uuid,
    kind: PhantomData<fn() -> K>,
}

impl<K> ProtocolId<K> {
    /// Creates an identifier from a UUID, rejecting the all-zero value.
    pub fn from_uuid(uuid: Uuid) -> Result<Self, IdError> {
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
        let uuid = parse_canonical_uuid(value)?;
        Self::from_uuid(uuid)
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
        serializer.collect_str(self)
    }
}

impl<'de, K> Deserialize<'de> for ProtocolId<K> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ProtocolIdVisitor(PhantomData))
    }
}

struct ProtocolIdVisitor<K>(PhantomData<fn() -> K>);

impl<K> de::Visitor<'_> for ProtocolIdVisitor<K> {
    type Value = ProtocolId<K>;

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

impl<K: kind::IdKind> JsonSchema for ProtocolId<K> {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed(K::SCHEMA_NAME)
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        let mut excluded_value = Map::new();
        excluded_value.insert("const".to_owned(), Value::String(NIL_UUID.to_owned()));

        let mut schema = Map::new();
        schema.insert("type".to_owned(), Value::String("string".to_owned()));
        schema.insert("format".to_owned(), Value::String("uuid".to_owned()));
        schema.insert("pattern".to_owned(), Value::String(UUID_PATTERN.to_owned()));
        schema.insert("not".to_owned(), Value::Object(excluded_value));
        Schema::from(schema)
    }
}

fn parse_canonical_uuid(value: &str) -> Result<Uuid, IdError> {
    let uuid = Uuid::parse_str(value).map_err(|_| IdError::InvalidFormat)?;
    let mut buffer = Uuid::encode_buffer();
    if uuid.hyphenated().encode_lower(&mut buffer) != value {
        return Err(IdError::InvalidFormat);
    }
    Ok(uuid)
}

mod kind {
    pub trait IdKind {
        const SCHEMA_NAME: &'static str;
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Session;

    impl IdKind for Session {
        const SCHEMA_NAME: &'static str = "SessionId";
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Action;

    impl IdKind for Action {
        const SCHEMA_NAME: &'static str = "ActionId";
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Batch;

    impl IdKind for Batch {
        const SCHEMA_NAME: &'static str = "BatchId";
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Command;

    impl IdKind for Command {
        const SCHEMA_NAME: &'static str = "CommandId";
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Object;

    impl IdKind for Object {
        const SCHEMA_NAME: &'static str = "ObjectId";
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Scene;

    impl IdKind for Scene {
        const SCHEMA_NAME: &'static str = "SceneId";
    }
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

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use schemars::generate::SchemaSettings;
    use serde_json::Value;

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
        assert_eq!(NIL_UUID.parse::<SessionId>(), Err(IdError::Nil));
    }

    #[test]
    fn role_aliases_are_distinct_and_keep_their_schema_names() {
        let session: SessionId = "94fa422b-301d-442d-b9a7-10ea54318e78".parse().unwrap();
        let object = ObjectId::from_uuid(session.into_uuid()).unwrap();

        assert_eq!(SessionId::schema_name(), "SessionId");
        assert_eq!(ObjectId::schema_name(), "ObjectId");
        assert_ne!(TypeId::of::<SessionId>(), TypeId::of::<ObjectId>());
        assert_eq!(session.to_string(), object.to_string());
    }

    #[test]
    fn draft_7_schema_avoids_nonportable_regex_lookaround() {
        let schema: Value = serde_json::to_value(
            SchemaSettings::draft07()
                .into_generator()
                .into_root_schema_for::<SessionId>(),
        )
        .unwrap();

        assert_eq!(schema["pattern"], UUID_PATTERN);
        assert_eq!(schema["not"]["const"], NIL_UUID);
        assert!(!schema["pattern"].as_str().unwrap().contains("?!"));
    }
}
