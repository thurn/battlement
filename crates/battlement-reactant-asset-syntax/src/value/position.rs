use super::{Calculation, Dimension, Scalar, Unit, Value, encode};

enum Component<'a> {
  Center,
  Value(&'a Value),
  Edge(&'a Value, Option<&'a Value>),
}

pub(super) fn valid(values: &[&Value]) -> bool {
  match values {
    [value] => self::component(value),
    [first, second] => self::pair(first, second),
    [first, second, third] => self::three(first, second, third),
    [first, first_offset, second, second_offset] => {
      self::opposite_edges(first, second)
        && self::length_percentage(first_offset)
        && self::length_percentage(second_offset)
    }
    _ => false,
  }
}

pub(super) fn is_center(values: &[&Value]) -> bool {
  !values.is_empty()
    && values
      .iter()
      .all(|value| self::keyword(value) == Some("center"))
}

pub(super) fn encode(values: &[&Value], bytes: &mut Vec<u8>) {
  let (horizontal, vertical) = self::components(values);
  self::encode_component(horizontal, bytes);
  self::encode_component(vertical, bytes);
}

fn components<'a>(values: &[&'a Value]) -> (Component<'a>, Component<'a>) {
  match values {
    [] => (Component::Center, Component::Center),
    [value] if self::horizontal(value) => (Component::Edge(value, None), Component::Center),
    [value] if self::vertical(value) => (Component::Center, Component::Edge(value, None)),
    [value] if self::keyword(value) == Some("center") => (Component::Center, Component::Center),
    [value] => (Component::Value(value), Component::Center),
    [first, second] if self::keyword(first).is_some() && self::keyword(second).is_some() => {
      self::keyword_pair(first, second)
    }
    [horizontal, vertical] => (
      self::canonical_component(horizontal),
      self::canonical_component(vertical),
    ),
    [first, offset, second] if self::length_percentage(offset) => {
      self::edge_pair(first, Some(offset), second, None)
    }
    [first, second, offset] => self::edge_pair(first, None, second, Some(offset)),
    [first, first_offset, second, second_offset] => {
      self::edge_pair(first, Some(first_offset), second, Some(second_offset))
    }
    _ => unreachable!("validated CSS position"),
  }
}

fn keyword_pair<'a>(first: &'a Value, second: &'a Value) -> (Component<'a>, Component<'a>) {
  let horizontal = [first, second]
    .into_iter()
    .find(|value| self::horizontal(value));
  let vertical = [first, second]
    .into_iter()
    .find(|value| self::vertical(value));
  (
    horizontal.map_or(Component::Center, |value| Component::Edge(value, None)),
    vertical.map_or(Component::Center, |value| Component::Edge(value, None)),
  )
}

fn edge_pair<'a>(
  first: &'a Value,
  first_offset: Option<&'a Value>,
  second: &'a Value,
  second_offset: Option<&'a Value>,
) -> (Component<'a>, Component<'a>) {
  if self::horizontal(first) {
    (
      Component::Edge(first, first_offset),
      Component::Edge(second, second_offset),
    )
  } else {
    (
      Component::Edge(second, second_offset),
      Component::Edge(first, first_offset),
    )
  }
}

fn canonical_component(value: &Value) -> Component<'_> {
  if self::keyword(value) == Some("center") {
    Component::Center
  } else if self::keyword(value).is_some() {
    Component::Edge(value, None)
  } else {
    Component::Value(value)
  }
}

fn encode_component(component: Component<'_>, bytes: &mut Vec<u8>) {
  match component {
    Component::Center => bytes.push(0),
    Component::Value(value) => {
      bytes.push(1);
      encode::value(value, bytes);
    }
    Component::Edge(edge, offset) => {
      bytes.push(if offset.is_some() { 3 } else { 2 });
      encode::value(edge, bytes);
      if let Some(offset) = offset {
        encode::value(offset, bytes);
      }
    }
  }
}

fn pair(first: &Value, second: &Value) -> bool {
  let first_keyword = self::keyword(first);
  let second_keyword = self::keyword(second);
  if first_keyword.is_some() && second_keyword.is_some() {
    return first_keyword == Some("center")
      || second_keyword == Some("center")
      || self::opposite_edges(first, second);
  }
  (self::horizontal(first) || self::length_percentage(first))
    && (self::vertical(second) || self::length_percentage(second))
}

fn three(first: &Value, second: &Value, third: &Value) -> bool {
  if self::length_percentage(second) {
    return self::opposite_edges(first, third);
  }
  self::opposite_edges(first, second) && self::length_percentage(third)
}

fn opposite_edges(first: &Value, second: &Value) -> bool {
  if self::horizontal(first) {
    self::vertical(second)
  } else {
    self::vertical(first) && self::horizontal(second)
  }
}

fn component(value: &Value) -> bool {
  self::length_percentage(value)
    || matches!(
      self::keyword(value),
      Some("left" | "right" | "top" | "bottom" | "center")
    )
}

fn horizontal(value: &Value) -> bool {
  matches!(self::keyword(value), Some("left" | "right"))
}

fn vertical(value: &Value) -> bool {
  matches!(self::keyword(value), Some("top" | "bottom"))
}

fn length_percentage(value: &Value) -> bool {
  match value {
    Value::Scalar(Scalar { value: 0.0, .. }) => true,
    Value::Scalar(value) => matches!(
      value.unit,
      Unit::Percent
        | Unit::Px
        | Unit::Em
        | Unit::Rem
        | Unit::Vw
        | Unit::Vh
        | Unit::Vmin
        | Unit::Vmax
    ),
    Value::Calculation(Calculation { dimension, .. }) => matches!(
      dimension,
      Dimension::Length | Dimension::Percentage | Dimension::LengthPercentage
    ),
    _ => false,
  }
}

fn keyword(value: &Value) -> Option<&str> {
  match value {
    Value::Keyword(value) => Some(value),
    _ => None,
  }
}
