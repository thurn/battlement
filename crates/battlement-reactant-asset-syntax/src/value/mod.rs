use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream};

use crate::{
  Diagnostic, DiagnosticCategory, SourceSpan,
  token::{Cursor, css_name},
};

mod calc;
mod display;
mod encode;

#[derive(Clone, Debug, PartialEq)]
enum Value {
  Scalar(Scalar),
  Color([f32; 4]),
  String(String),
  Keyword(String),
  Function(String, Box<Value>),
  Space(Vec<Value>),
  Comma(Vec<Value>),
  Calculation(Calculation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Unit {
  Number,
  Percent,
  Px,
  Em,
  Rem,
  Vw,
  Vh,
  Vmin,
  Vmax,
  Deg,
  Grad,
  Rad,
  Turn,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Scalar {
  value: f64,
  unit: Unit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dimension {
  Number,
  Length,
  Percentage,
  LengthPercentage,
  Angle,
}

#[derive(Clone, Debug, PartialEq)]
struct Calculation {
  node: CalcNode,
  dimension: Dimension,
  basis: Option<Unit>,
  constant: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
enum CalcNode {
  Scalar(Scalar),
  Add(Box<Calculation>, Box<Calculation>),
  Subtract(Box<Calculation>, Box<Calculation>),
  Multiply(Box<Calculation>, Box<Calculation>),
  Divide(Box<Calculation>, Box<Calculation>),
  Min(Vec<Calculation>),
  Max(Vec<Calculation>),
  Clamp(Vec<Calculation>),
  Group(Box<Calculation>),
}

pub(crate) struct ValueError {
  pub(crate) category: DiagnosticCategory,
  span: SourceSpan,
}

pub(crate) fn canonicalize(source: &str) -> Result<Vec<u8>, ValueError> {
  let value = self::parse(source)?;
  let mut bytes = Vec::new();
  encode::value(&value, &mut bytes);
  Ok(bytes)
}

pub(crate) fn serialize(source: &str) -> Result<String, ValueError> {
  self::parse(source).map(|value| value.to_string())
}

pub(crate) fn standalone_diagnostic(error: ValueError) -> Diagnostic {
  Diagnostic {
    category: error.category,
    symbol: None,
    property: None,
    span: error.span,
  }
}

fn parse(source: &str) -> Result<Value, ValueError> {
  let stream = TokenStream::from_str(source).map_err(|error| ValueError {
    category: DiagnosticCategory::InvalidValue,
    span: error.span().into(),
  })?;
  self::parse_stream(stream)
}

fn parse_stream(stream: TokenStream) -> Result<Value, ValueError> {
  let mut cursor = Cursor::new(stream);
  let value = self::list(&mut cursor)?;
  if cursor.is_empty() {
    Ok(value)
  } else {
    Err(self::invalid(&cursor))
  }
}

fn list(cursor: &mut Cursor) -> Result<Value, ValueError> {
  let mut comma = Vec::new();
  loop {
    let mut space = Vec::new();
    while !cursor.is_empty() && !cursor.peek_punct(',') {
      space.push(self::atom(cursor)?);
    }
    if space.is_empty() {
      return Err(self::invalid(cursor));
    }
    comma.push(if space.len() == 1 {
      space.pop().expect("one space-list item")
    } else {
      Value::Space(space)
    });
    if !cursor.punct(',') {
      break;
    }
  }
  Ok(if comma.len() == 1 {
    comma.pop().expect("one comma-list item")
  } else {
    Value::Comma(comma)
  })
}

fn atom(cursor: &mut Cursor) -> Result<Value, ValueError> {
  if cursor.punct('#') {
    return self::hex_color(cursor);
  }
  if let Some(value) = cursor.string() {
    return Ok(Value::String(value));
  }
  if let Some(scalar) = self::scalar(cursor)? {
    return Ok(Value::Scalar(scalar));
  }
  let span = cursor.span();
  let name = css_name(cursor)
    .map(|name| name.to_ascii_lowercase())
    .ok_or(ValueError {
      category: DiagnosticCategory::InvalidValue,
      span,
    })?;
  if let Some((arguments, _)) = cursor.group(Delimiter::Parenthesis) {
    if matches!(name.as_str(), "rgb" | "rgba" | "hsl" | "hsla") {
      let authored = format!("{name}({})", arguments.to_string().replace(" %", "%"));
      return self::color(&authored, span).map(Value::Color);
    }
    if matches!(name.as_str(), "calc" | "min" | "max" | "clamp") {
      return calc::parse(&name, arguments, span).map(Value::Calculation);
    }
    return self::parse_stream(arguments).map(|value| Value::Function(name, Box::new(value)));
  }
  if name == "transparent" || !name.chars().all(|character| character.is_ascii_hexdigit()) {
    if let Ok(color) = csscolorparser::parse(&name) {
      return Ok(Value::Color(color.to_array()));
    }
  }
  Ok(Value::Keyword(name))
}

fn hex_color(cursor: &mut Cursor) -> Result<Value, ValueError> {
  let span = cursor.span();
  let value = cursor
    .literal()
    .or_else(|| cursor.ident().map(|value| value.0));
  let value = value.ok_or(ValueError {
    category: DiagnosticCategory::InvalidValue,
    span,
  })?;
  self::color(&format!("#{value}"), span).map(Value::Color)
}

fn color(source: &str, span: SourceSpan) -> Result<[f32; 4], ValueError> {
  let color = csscolorparser::parse(source).map_err(|_| ValueError {
    category: DiagnosticCategory::InvalidValue,
    span,
  })?;
  let channels = color
    .to_array()
    .map(|channel| if channel == 0.0 { 0.0 } else { channel });
  channels
    .iter()
    .all(|channel| channel.is_finite())
    .then_some(channels)
    .ok_or(ValueError {
      category: DiagnosticCategory::InvalidValue,
      span,
    })
}

fn scalar(cursor: &mut Cursor) -> Result<Option<Scalar>, ValueError> {
  let span = cursor.span();
  let negative = cursor.punct('-');
  let positive = !negative && cursor.punct('+');
  let Some(literal) = cursor.literal() else {
    if negative || positive {
      return Err(self::invalid_at(span));
    }
    return Ok(None);
  };
  let (mut value, suffix) =
    self::number_and_suffix(&literal).ok_or_else(|| self::invalid_at(span))?;
  if negative {
    value = -value;
  }
  if !value.is_finite() {
    return Err(self::invalid_at(span));
  }
  let unit = if suffix.is_empty() && cursor.punct('%') {
    Unit::Percent
  } else {
    self::unit(&suffix).ok_or_else(|| self::invalid_at(span))?
  };
  Ok(Some(Scalar {
    value: if value == 0.0 { 0.0 } else { value },
    unit,
  }))
}

fn number_and_suffix(value: &str) -> Option<(f64, String)> {
  if value.contains('_') {
    return None;
  }
  value
    .char_indices()
    .map(|(index, _)| index)
    .chain([value.len()])
    .skip(1)
    .filter_map(|index| value[..index].parse().ok().map(|number| (number, index)))
    .last()
    .map(|(number, index)| (number, value[index..].to_ascii_lowercase()))
}

fn unit(value: &str) -> Option<Unit> {
  match value {
    "" => Some(Unit::Number),
    "px" => Some(Unit::Px),
    "em" => Some(Unit::Em),
    "rem" => Some(Unit::Rem),
    "vw" => Some(Unit::Vw),
    "vh" => Some(Unit::Vh),
    "vmin" => Some(Unit::Vmin),
    "vmax" => Some(Unit::Vmax),
    "deg" => Some(Unit::Deg),
    "grad" => Some(Unit::Grad),
    "rad" => Some(Unit::Rad),
    "turn" => Some(Unit::Turn),
    _ => None,
  }
}

fn invalid(cursor: &Cursor) -> ValueError {
  self::invalid_at(cursor.span())
}

fn invalid_at(span: SourceSpan) -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidValue,
    span,
  }
}
