use std::str::FromStr;

use proc_macro2::{Span, TokenStream, TokenTree};

use crate::{DiagnosticCategory, SourceSpan};

use super::{Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError};

pub(super) fn parse(source: &str) -> Result<ParsedValue, ValueError> {
  let stream = TokenStream::from_str(source).map_err(|error| self::error(error.span().into()))?;
  let layers = self::split(stream, ',')?;
  let layer_count = layers.len();
  let mut canonical = vec![32];
  canonical.extend(
    u32::try_from(layers.len())
      .expect("background layer count overflow")
      .to_be_bytes(),
  );
  let mut dependencies = Vec::new();
  for (index, layer) in layers.into_iter().enumerate() {
    self::layer(
      layer,
      index + 1 == layer_count,
      &mut canonical,
      &mut dependencies,
    )?;
  }
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: "background".to_owned(),
      canonical,
    }],
    dependencies,
  })
}

fn layer(
  stream: TokenStream,
  final_layer: bool,
  canonical: &mut Vec<u8>,
  dependencies: &mut Vec<String>,
) -> Result<(), ValueError> {
  let sides = self::split(stream, '/')?;
  if sides.len() > 2 {
    return Err(self::invalid());
  }
  let before = self::atoms(super::parse_stream(sides[0].clone())?);
  let after = sides
    .get(1)
    .map(|stream| super::parse_stream(stream.clone()).map(self::atoms))
    .transpose()?
    .unwrap_or_default();
  let mut parts = Parts::default();
  for value in before {
    self::classify(value, true, &mut parts)?;
  }
  for value in after {
    self::classify(value, false, &mut parts)?;
  }
  if parts.color.is_some() && !final_layer {
    return Err(self::invalid());
  }
  if parts.source.is_none() {
    if parts.color.is_none() || parts.has_components() {
      return Err(self::invalid());
    }
  } else {
    self::position(&parts.position)?;
    self::size(&parts.size, sides.len() == 2, !parts.position.is_empty())?;
    self::repeat(&parts.repeat)?;
    self::boxes(&parts.boxes)?;
  }
  canonical.push(1);
  self::optional_value(&parts.source, canonical);
  self::values(&parts.position, canonical);
  self::values(&parts.size, canonical);
  self::values(&parts.repeat, canonical);
  self::values(&parts.boxes, canonical);
  self::optional_value(&parts.color, canonical);
  if let Some(path) = parts.dependency {
    dependencies.push(path);
  }
  Ok(())
}

#[derive(Default)]
struct Parts {
  source: Option<Value>,
  color: Option<Value>,
  dependency: Option<String>,
  position: Vec<Value>,
  size: Vec<Value>,
  repeat: Vec<Value>,
  boxes: Vec<Value>,
}

impl Parts {
  fn has_components(&self) -> bool {
    !self.position.is_empty()
      || !self.size.is_empty()
      || !self.repeat.is_empty()
      || !self.boxes.is_empty()
  }
}

fn classify(value: Value, before_slash: bool, parts: &mut Parts) -> Result<(), ValueError> {
  if matches!(value, Value::Color([_, _, _, 0.0])) {
    return Err(self::redundant());
  }
  if matches!(value, Value::Color(_)) {
    if parts.color.replace(value).is_some() {
      return Err(self::invalid());
    }
    return Ok(());
  }
  if let Value::Function(name, arguments) = &value {
    if !before_slash || parts.source.is_some() {
      return Err(self::invalid());
    }
    let dependency = self::image(name, arguments)?;
    parts.source = Some(value);
    parts.dependency = dependency;
    return Ok(());
  }
  let destination = match self::keyword(&value) {
    Some("no-repeat" | "repeat-x" | "repeat-y" | "round" | "space" | "repeat") => &mut parts.repeat,
    Some("border-box" | "padding-box" | "content-box") => &mut parts.boxes,
    _ if before_slash => &mut parts.position,
    _ => &mut parts.size,
  };
  destination.push(value);
  Ok(())
}

fn image(name: &str, arguments: &Value) -> Result<Option<String>, ValueError> {
  match name {
    "unity-url" => {
      let Value::String(path) = arguments else {
        return Err(self::invalid());
      };
      super::local_path(path, &["png"])
        .map(Some)
        .ok_or_else(self::invalid)
    }
    "linear-gradient" | "repeating-linear-gradient" => {
      super::gradient::validate(arguments, super::gradient::Gradient::Linear)?;
      Ok(None)
    }
    "radial-gradient" | "repeating-radial-gradient" => {
      super::gradient::validate(arguments, super::gradient::Gradient::Radial)?;
      Ok(None)
    }
    "conic-gradient" | "repeating-conic-gradient" => {
      super::gradient::validate(arguments, super::gradient::Gradient::Conic)?;
      Ok(None)
    }
    _ => Err(self::invalid()),
  }
}

fn position(values: &[Value]) -> Result<(), ValueError> {
  let references = values.iter().collect::<Vec<_>>();
  if !references.is_empty() && !super::position::valid(&references) {
    return Err(self::invalid());
  }
  if values.len() == 2
    && ((self::keyword(&values[0]) == Some("left") && self::keyword(&values[1]) == Some("top"))
      || (self::zero(&values[0]) && self::zero(&values[1])))
  {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn size(values: &[Value], slash: bool, positioned: bool) -> Result<(), ValueError> {
  if !slash {
    return Ok(());
  }
  let valid_keyword =
    values.len() == 1 && matches!(self::keyword(&values[0]), Some("cover" | "contain"));
  let valid_dimensions = (1..=2).contains(&values.len())
    && values
      .iter()
      .all(|value| self::length_percentage(value) || self::keyword(value) == Some("auto"));
  if valid_dimensions
    && values
      .iter()
      .all(|value| self::keyword(value) == Some("auto"))
  {
    return Err(self::redundant());
  }
  if positioned && (valid_keyword || valid_dimensions) {
    Ok(())
  } else {
    Err(self::invalid())
  }
}

fn repeat(values: &[Value]) -> Result<(), ValueError> {
  let valid = values.len() <= 2
    && values.iter().all(|value| {
      matches!(
        self::keyword(value),
        Some("no-repeat" | "repeat-x" | "repeat-y" | "round" | "space")
      )
    });
  if valid {
    Ok(())
  } else if values
    .iter()
    .any(|value| self::keyword(value) == Some("repeat"))
  {
    Err(self::redundant())
  } else {
    Err(self::invalid())
  }
}

fn boxes(values: &[Value]) -> Result<(), ValueError> {
  let valid = values.len() <= 2
    && values.iter().all(|value| {
      matches!(
        self::keyword(value),
        Some("border-box" | "padding-box" | "content-box")
      )
    });
  if !valid {
    return Err(self::invalid());
  }
  if (values.len() == 1 && self::keyword(&values[0]) != Some("content-box"))
    || values.first().and_then(self::keyword) == Some("padding-box")
    || values.get(1).and_then(self::keyword) == Some("border-box")
  {
    Err(self::redundant())
  } else {
    Ok(())
  }
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
    Value::Calculation(value) => matches!(
      value.dimension,
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

fn zero(value: &Value) -> bool {
  matches!(value, Value::Scalar(Scalar { value: 0.0, .. }))
}

fn atoms(value: Value) -> Vec<Value> {
  match value {
    Value::Space(values) => values,
    value => vec![value],
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

fn optional_value(value: &Option<Value>, bytes: &mut Vec<u8>) {
  if let Some(value) = value {
    bytes.push(1);
    super::encode::value(value, bytes);
  } else {
    bytes.push(0);
  }
}

fn values(values: &[Value], bytes: &mut Vec<u8>) {
  bytes.extend(
    u32::try_from(values.len())
      .expect("background value count overflow")
      .to_be_bytes(),
  );
  for value in values {
    super::encode::value(value, bytes);
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
