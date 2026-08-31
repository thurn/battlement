use std::str::FromStr;

use proc_macro2::{Span, TokenStream, TokenTree};

use crate::{DiagnosticCategory, SourceSpan};

use super::{Calculation, Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError};

const EDGES: [&str; 4] = ["top", "right", "bottom", "left"];
const CORNERS: [&str; 4] = ["top-left", "top-right", "bottom-right", "bottom-left"];

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  let stream = TokenStream::from_str(source).map_err(|error| self::error(error.span().into()))?;
  let fields = match property {
    "border" => self::shorthand(stream, &EDGES)?,
    "border-top" => self::shorthand(stream, &["top"])?,
    "border-right" => self::shorthand(stream, &["right"])?,
    "border-bottom" => self::shorthand(stream, &["bottom"])?,
    "border-left" => self::shorthand(stream, &["left"])?,
    "border-width" => self::four_sides(stream, "width", self::width)?,
    "border-style" => self::four_sides(stream, "style", self::style)?,
    "border-color" => self::four_sides(stream, "color", self::color)?,
    "border-radius" => self::radius(stream)?,
    _ => unreachable!("border parser called with unsupported property"),
  };
  Ok(ParsedValue {
    fields,
    dependencies: Vec::new(),
  })
}

fn shorthand(stream: TokenStream, edges: &[&str]) -> Result<Vec<ParsedField>, ValueError> {
  let values = self::atoms(super::parse_stream(stream)?)?;
  if values.len() == 1 && self::keyword(&values[0]) == Some("none") {
    return Ok(
      edges
        .iter()
        .flat_map(|edge| {
          ["width", "style", "color"]
            .map(|component| self::field(&format!("border-{edge}-{component}"), &values[0]))
        })
        .collect(),
    );
  }
  let mut width = None;
  let mut style = None;
  let mut color = None;
  for value in values {
    if self::is_width(&value) {
      self::set_once(&mut width, value)?;
    } else if self::is_style(&value) {
      self::set_once(&mut style, value)?;
    } else if matches!(value, Value::Color(_)) {
      self::set_once(&mut color, value)?;
    } else {
      return Err(self::invalid());
    }
  }
  let components = [
    ("width", width.ok_or_else(self::invalid)?),
    ("style", style.ok_or_else(self::invalid)?),
    ("color", color.ok_or_else(self::invalid)?),
  ];
  Ok(
    edges
      .iter()
      .flat_map(|edge| {
        components
          .iter()
          .map(move |(name, value)| self::field(&format!("border-{edge}-{name}"), value))
      })
      .collect(),
  )
}

fn four_sides(
  stream: TokenStream,
  component: &str,
  validate: fn(&Value) -> Result<(), ValueError>,
) -> Result<Vec<ParsedField>, ValueError> {
  let values = self::atoms(super::parse_stream(stream)?)?;
  if values.len() > 4 {
    return Err(self::invalid());
  }
  for value in &values {
    validate(value)?;
  }
  Ok(
    self::expand(&values)
      .into_iter()
      .zip(EDGES)
      .map(|(value, edge)| self::field(&format!("border-{edge}-{component}"), value))
      .collect(),
  )
}

fn radius(stream: TokenStream) -> Result<Vec<ParsedField>, ValueError> {
  let sides = self::split(stream, '/')?;
  if sides.len() > 2 {
    return Err(self::invalid());
  }
  let horizontal = self::atoms(super::parse_stream(sides[0].clone())?)?;
  let vertical = sides
    .get(1)
    .map(|value| super::parse_stream(value.clone()).and_then(self::atoms))
    .transpose()?
    .unwrap_or_else(|| horizontal.clone());
  if horizontal.len() > 4 || vertical.len() > 4 {
    return Err(self::invalid());
  }
  for value in horizontal.iter().chain(&vertical) {
    self::radius_value(value)?;
  }
  let horizontal = self::expand(&horizontal);
  let vertical = self::expand(&vertical);
  if sides.len() == 2 && horizontal == vertical {
    return Err(self::redundant());
  }
  Ok(
    CORNERS
      .into_iter()
      .zip(horizontal.iter().zip(vertical.iter()))
      .flat_map(|(corner, (x, y))| {
        [
          self::field(&format!("border-{corner}-radius-x"), x),
          self::field(&format!("border-{corner}-radius-y"), y),
        ]
      })
      .collect(),
  )
}

fn width(value: &Value) -> Result<(), ValueError> {
  self::length(value, false)
}

fn radius_value(value: &Value) -> Result<(), ValueError> {
  self::length(value, true)
}

fn length(value: &Value, percentage: bool) -> Result<(), ValueError> {
  let valid = match value {
    Value::Scalar(Scalar { value, unit }) => {
      *value >= 0.0
        && (matches!(
          unit,
          Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax
        ) || (*unit == Unit::Number && *value == 0.0)
          || (percentage && *unit == Unit::Percent))
    }
    Value::Calculation(Calculation {
      dimension,
      constant,
      ..
    }) => {
      (matches!(dimension, Dimension::Length)
        || (percentage
          && matches!(
            dimension,
            Dimension::Percentage | Dimension::LengthPercentage
          )))
        && constant.is_none_or(|value| value >= 0.0)
    }
    _ => false,
  };
  valid.then_some(()).ok_or_else(self::invalid)
}

fn style(value: &Value) -> Result<(), ValueError> {
  self::is_style(value)
    .then_some(())
    .ok_or_else(self::invalid)
}

fn color(value: &Value) -> Result<(), ValueError> {
  matches!(value, Value::Color(_))
    .then_some(())
    .ok_or_else(self::invalid)
}

fn is_width(value: &Value) -> bool {
  self::width(value).is_ok()
}

fn is_style(value: &Value) -> bool {
  matches!(
    self::keyword(value),
    Some("none" | "solid" | "dashed" | "dotted" | "double")
  )
}

fn keyword(value: &Value) -> Option<&str> {
  if let Value::Keyword(value) = value {
    Some(value)
  } else {
    None
  }
}

fn atoms(value: Value) -> Result<Vec<Value>, ValueError> {
  match value {
    Value::Space(values) => Ok(values),
    Value::Comma(_) => Err(self::invalid()),
    value => Ok(vec![value]),
  }
}

fn expand(values: &[Value]) -> [&Value; 4] {
  match values {
    [all] => [all, all, all, all],
    [vertical, horizontal] => [vertical, horizontal, vertical, horizontal],
    [top, horizontal, bottom] => [top, horizontal, bottom, horizontal],
    [top, right, bottom, left] => [top, right, bottom, left],
    _ => unreachable!("validated one-to-four value list"),
  }
}

fn set_once(slot: &mut Option<Value>, value: Value) -> Result<(), ValueError> {
  if slot.replace(value).is_some() {
    Err(self::invalid())
  } else {
    Ok(())
  }
}

fn field(property: &str, value: &Value) -> ParsedField {
  let mut canonical = Vec::new();
  super::encode::value(value, &mut canonical);
  ParsedField {
    property: property.to_owned(),
    canonical,
  }
}

fn split(stream: TokenStream, separator: char) -> Result<Vec<TokenStream>, ValueError> {
  let mut result = vec![TokenStream::new()];
  for token in stream {
    if matches!(&token, TokenTree::Punct(value) if value.as_char() == separator) {
      if result.last().is_some_and(TokenStream::is_empty) {
        return Err(self::invalid());
      }
      result.push(TokenStream::new());
    } else {
      result
        .last_mut()
        .expect("one split segment")
        .extend([token]);
    }
  }
  if result.last().is_some_and(TokenStream::is_empty) {
    Err(self::invalid())
  } else {
    Ok(result)
  }
}

fn invalid() -> ValueError {
  self::error(Span::call_site().into())
}

fn redundant() -> ValueError {
  ValueError {
    category: DiagnosticCategory::RedundantDefault,
    span: Span::call_site().into(),
  }
}

fn error(span: SourceSpan) -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidValue,
    span,
  }
}
