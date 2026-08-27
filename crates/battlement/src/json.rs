//! JSON encoding for Battlement protocol values.

use serde::{Deserialize, Serialize};

/// Encodes one value as minified UTF-8 JSON.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
  T: ?Sized + Serialize,
{
  serde_json::to_vec(value)
}

/// Decodes exactly one JSON value from UTF-8 bytes.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T, serde_json::Error>
where
  T: for<'de> Deserialize<'de>,
{
  let mut deserializer = serde_json::Deserializer::from_slice(bytes);
  let value = T::deserialize(&mut deserializer)?;
  deserializer.end()?;
  Ok(value)
}
