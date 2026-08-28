use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A sparse property value that may be omitted, assigned, or reset.
///
/// Omitted properties leave live state unchanged. [`Self::Set`] serializes its
/// value, while [`Self::Reset`] serializes `null` and restores the documented
/// native default. Builders accept ordinary values through [`From<T>`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Prop<T> {
  /// Omits the property from the wire representation.
  #[default]
  Unset,
  /// Assigns a concrete property value.
  Set(T),
  /// Restores the property's documented native default.
  Reset,
}

impl<T> Prop<T> {
  pub(crate) const fn is_unset(&self) -> bool {
    matches!(self, Self::Unset)
  }

  pub(crate) const fn set_value(&self) -> Option<&T> {
    match self {
      Self::Set(value) => Some(value),
      Self::Unset | Self::Reset => None,
    }
  }
}

impl<T> Prop<Vec<T>> {
  pub(crate) fn push(&mut self, value: T) {
    if !matches!(self, Self::Set(_)) {
      *self = Self::Set(Vec::new());
    }
    let Self::Set(values) = self else {
      unreachable!("set property disappeared");
    };
    values.push(value);
  }
}

impl<T> From<T> for Prop<T> {
  fn from(value: T) -> Self {
    Self::Set(value)
  }
}

impl<T> From<Option<T>> for Prop<T> {
  fn from(value: Option<T>) -> Self {
    value.map_or(Self::Unset, Self::Set)
  }
}

impl From<&str> for Prop<String> {
  fn from(value: &str) -> Self {
    Self::Set(value.to_owned())
  }
}

impl From<&String> for Prop<String> {
  fn from(value: &String) -> Self {
    Self::Set(value.clone())
  }
}

impl<T> Serialize for Prop<T>
where
  T: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    match self {
      Self::Set(value) => value.serialize(serializer),
      Self::Unset | Self::Reset => serializer.serialize_none(),
    }
  }
}

impl<'de, T> Deserialize<'de> for Prop<T>
where
  T: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Ok(Option::<T>::deserialize(deserializer)?.map_or(Self::Reset, Self::Set))
  }
}
