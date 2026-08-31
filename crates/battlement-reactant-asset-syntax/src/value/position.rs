use super::{Calculation, Dimension, Scalar, Unit, Value};

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
