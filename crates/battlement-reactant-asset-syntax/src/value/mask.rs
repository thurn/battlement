use std::str::FromStr;

use proc_macro2::{Span, TokenStream, TokenTree};

use crate::{
  DiagnosticCategory, SourceSpan, canonical,
  value::{
    self, Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError, encode, gradient,
    position,
  },
};

pub(super) fn parse(source: &str) -> Result<ParsedValue, ValueError> {
  let stream = TokenStream::from_str(source).map_err(|error| self::error(error.span().into()))?;
  let layers = self::split(stream, ',')?;
  let mut canonical = vec![46];
  canonical.extend(
    u32::try_from(layers.len())
      .expect("mask layer count overflow")
      .to_be_bytes(),
  );
  let mut dependencies = Vec::new();
  for layer in layers {
    self::layer(layer, &mut canonical, &mut dependencies)?;
  }
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: "mask".to_owned(),
      canonical,
    }],
    dependencies,
    relation: None,
  })
}

fn layer(
  stream: TokenStream,
  canonical: &mut Vec<u8>,
  dependencies: &mut Vec<String>,
) -> Result<(), ValueError> {
  let sides = self::split(stream, '/')?;
  if sides.len() > 2 {
    return Err(self::invalid());
  }
  let before = self::atoms(value::parse_stream(sides[0].clone())?);
  if before.len() == 1 && self::keyword(&before[0]) == Some("none") {
    return Err(self::redundant());
  }
  let after = sides
    .get(1)
    .map(|stream| value::parse_stream(stream.clone()).map(self::atoms))
    .transpose()?
    .unwrap_or_default();
  let mut parts = Parts::default();
  for value in before {
    self::classify(value, true, &mut parts)?;
  }
  for value in after {
    self::classify(value, false, &mut parts)?;
  }
  if parts.source.is_none() {
    return Err(self::invalid());
  }
  self::position(&parts.position)?;
  self::size(&parts.size, sides.len() == 2, !parts.position.is_empty())?;
  let repeat = self::repeat(&parts.repeat)?;
  let boxes = self::boxes(&parts.boxes)?;

  canonical.push(1);
  encode::value(
    parts.source.as_ref().expect("validated mask source"),
    canonical,
  );
  if parts.position.is_empty() {
    canonical.push(0);
  } else {
    canonical.push(1);
    position::encode(&parts.position.iter().collect::<Vec<_>>(), canonical);
  }
  self::values(&parts.size, canonical);
  canonical.extend(repeat);
  canonical.extend(boxes);
  self::optional_keyword(&parts.mode, canonical);
  self::optional_keyword(&parts.composition, canonical);
  if let Some(path) = parts.dependency {
    dependencies.push(path);
  }
  Ok(())
}

#[derive(Default)]
struct Parts {
  source: Option<Value>,
  dependency: Option<String>,
  position: Vec<Value>,
  size: Vec<Value>,
  repeat: Vec<Value>,
  boxes: Vec<Value>,
  mode: Option<String>,
  composition: Option<String>,
}

fn classify(value: Value, before_slash: bool, parts: &mut Parts) -> Result<(), ValueError> {
  if let Value::Function(name, arguments) = &value {
    if !before_slash || parts.source.is_some() {
      return Err(self::invalid());
    }
    parts.dependency = self::image(name, arguments)?;
    parts.source = Some(value);
    return Ok(());
  }
  let keyword = self::keyword(&value);
  match keyword {
    Some("alpha" | "luminance") => self::set_keyword(&mut parts.mode, keyword.expect("keyword"))?,
    Some("add" | "subtract" | "intersect" | "exclude") => {
      self::set_keyword(&mut parts.composition, keyword.expect("keyword"))?
    }
    Some("no-repeat" | "repeat-x" | "repeat-y" | "round" | "space" | "repeat") => {
      parts.repeat.push(value)
    }
    Some("border-box" | "padding-box" | "content-box" | "no-clip") => parts.boxes.push(value),
    _ if before_slash => parts.position.push(value),
    _ => parts.size.push(value),
  }
  Ok(())
}

fn image(name: &str, arguments: &Value) -> Result<Option<String>, ValueError> {
  match name {
    "unity-url" => {
      let Value::String(path) = arguments else {
        return Err(self::invalid());
      };
      value::local_path(path, &["png"])
        .map(Some)
        .ok_or_else(self::invalid)
    }
    "linear-gradient" | "repeating-linear-gradient" => {
      gradient::validate(arguments, gradient::Gradient::Linear)?;
      Ok(None)
    }
    "radial-gradient" | "repeating-radial-gradient" => {
      gradient::validate(arguments, gradient::Gradient::Radial)?;
      Ok(None)
    }
    "conic-gradient" | "repeating-conic-gradient" => {
      gradient::validate(arguments, gradient::Gradient::Conic)?;
      Ok(None)
    }
    _ => Err(self::invalid()),
  }
}

fn position(values: &[Value]) -> Result<(), ValueError> {
  let references = values.iter().collect::<Vec<_>>();
  if !references.is_empty() && !position::valid(&references) {
    return Err(self::invalid());
  }
  let top_left = values.len() == 2
    && self::keyword(&values[0]) == Some("left")
    && self::keyword(&values[1]) == Some("top");
  let zero = values.len() == 2 && self::zero(&values[0]) && self::zero(&values[1]);
  if top_left || zero {
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

fn repeat(values: &[Value]) -> Result<[u8; 2], ValueError> {
  let keywords = values
    .iter()
    .map(self::keyword)
    .collect::<Option<Vec<_>>>()
    .ok_or_else(self::invalid)?;
  match keywords.as_slice() {
    [] => Ok([0, 0]),
    ["repeat"] | ["repeat", "repeat"] => Err(self::redundant()),
    ["repeat-x"] => Ok([1, 2]),
    ["repeat-y"] => Ok([2, 1]),
    [value] => self::repeat_tag(value)
      .map(|tag| [tag, tag])
      .ok_or_else(self::invalid),
    [horizontal, vertical] => Ok([
      self::repeat_tag(horizontal).ok_or_else(self::invalid)?,
      self::repeat_tag(vertical).ok_or_else(self::invalid)?,
    ]),
    _ => Err(self::invalid()),
  }
}

fn boxes(values: &[Value]) -> Result<[u8; 2], ValueError> {
  let keywords = values
    .iter()
    .map(self::keyword)
    .collect::<Option<Vec<_>>>()
    .ok_or_else(self::invalid)?;
  let (origin, clip) = match keywords.as_slice() {
    [] => return Ok([0, 0]),
    ["border-box"] => return Err(self::redundant()),
    ["no-clip"] => ("border-box", "no-clip"),
    [box_value @ ("padding-box" | "content-box")] => (*box_value, *box_value),
    [
      origin @ ("border-box" | "padding-box" | "content-box"),
      clip,
    ] => (*origin, *clip),
    _ => return Err(self::invalid()),
  };
  let origin_tag = self::box_tag(origin).ok_or_else(self::invalid)?;
  let clip_tag = self::clip_tag(clip).ok_or_else(self::invalid)?;
  if origin == "border-box" || clip == "border-box" {
    return Err(self::redundant());
  }
  Ok([origin_tag, clip_tag])
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

fn set_keyword(destination: &mut Option<String>, value: &str) -> Result<(), ValueError> {
  if destination.replace(value.to_owned()).is_some() {
    Err(self::invalid())
  } else {
    Ok(())
  }
}

fn optional_keyword(value: &Option<String>, bytes: &mut Vec<u8>) {
  if let Some(value) = value {
    bytes.push(1);
    canonical::string(bytes, value);
  } else {
    bytes.push(0);
  }
}

fn values(values: &[Value], bytes: &mut Vec<u8>) {
  bytes.extend(
    u32::try_from(values.len())
      .expect("mask value count overflow")
      .to_be_bytes(),
  );
  for value in values {
    encode::value(value, bytes);
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

fn repeat_tag(value: &str) -> Option<u8> {
  match value {
    "repeat" => Some(1),
    "no-repeat" => Some(2),
    "round" => Some(3),
    "space" => Some(4),
    _ => None,
  }
}

fn box_tag(value: &str) -> Option<u8> {
  match value {
    "border-box" => Some(1),
    "padding-box" => Some(2),
    "content-box" => Some(3),
    _ => None,
  }
}

fn clip_tag(value: &str) -> Option<u8> {
  self::box_tag(value).or((value == "no-clip").then_some(4))
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
