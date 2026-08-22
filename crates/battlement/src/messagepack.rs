//! MessagePack encoding for Battlement protocol values.

use std::io::Cursor;

use rmp_serde::Deserializer;
use rmp_serde::decode::Error;
use serde::{Deserialize, Serialize};

/// Encodes one value using Battlement's compact MessagePack representation.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, rmp_serde::encode::Error>
where
    T: ?Sized + Serialize,
{
    rmp_serde::to_vec(value)
}

/// Decodes one value from Battlement's compact MessagePack representation.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut deserializer = Deserializer::new(Cursor::new(bytes));
    deserializer.set_max_depth(128);
    let value = T::deserialize(&mut deserializer)?;
    if deserializer.position() != bytes.len() as u64 {
        return Err(Error::Syntax(
            "trailing bytes after MessagePack value".into(),
        ));
    }

    Ok(value)
}
