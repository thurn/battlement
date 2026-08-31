use std::str::FromStr;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::DiagnosticCategory;

use super::{
  Calculation, Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError, encode,
  position,
};

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  if source.trim().eq_ignore_ascii_case("none") {
    return Err(self::redundant());
  }
  let (name, arguments) = self::function(source)?;
  let canonical = match name.as_str() {
    "inset" => self::inset(arguments)?,
    "circle" | "ellipse" => {
      let value = if arguments.is_empty() && name != "polygon" {
        Value::Function(name.clone(), Box::new(Value::Space(Vec::new())))
      } else {
        super::parse(source)?
      };
      let Value::Function(_, arguments) = value else {
        return Err(self::invalid());
      };
      if name == "circle" {
        self::circle(&arguments)?
      } else {
        self::ellipse(&arguments)?
      }
    }
    "polygon" => {
      let value = super::parse(source)?;
      let Value::Function(_, arguments) = &value else {
        return Err(self::invalid());
      };
      self::polygon(arguments)?;
      let mut canonical = Vec::new();
      encode::value(&value, &mut canonical);
      canonical
    }
    "path" => return Err(self::invalid()),
    _ => return Err(self::invalid()),
  };
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: property.to_owned(),
      canonical,
    }],
    dependencies: Vec::new(),
  })
}

fn function(source: &str) -> Result<(String, TokenStream), ValueError> {
  let stream = TokenStream::from_str(source).map_err(|_| self::invalid())?;
  let mut tokens = stream.into_iter();
  let name = match tokens.next() {
    Some(TokenTree::Ident(value)) => value.to_string().to_ascii_lowercase(),
    _ => return Err(self::invalid()),
  };
  let arguments = match tokens.next() {
    Some(TokenTree::Group(value)) if value.delimiter() == Delimiter::Parenthesis => value.stream(),
    _ => return Err(self::invalid()),
  };
  if tokens.next().is_some() {
    Err(self::invalid())
  } else {
    Ok((name, arguments))
  }
}

fn inset(arguments: TokenStream) -> Result<Vec<u8>, ValueError> {
  let (offsets, radii) = self::split_round(arguments)?;
  let offsets = self::values(offsets)?;
  if offsets.len() > 4 || !offsets.iter().all(self::length_percentage) {
    return Err(self::invalid());
  }
  let offsets = self::expand(&offsets);
  let radii = radii.map(self::radii).transpose()?;
  let mut canonical = vec![41];
  self::encode_values(&offsets, &mut canonical);
  if let Some((horizontal, vertical)) = radii {
    canonical.push(1);
    self::encode_values(&horizontal, &mut canonical);
    self::encode_values(&vertical, &mut canonical);
  } else {
    canonical.push(0);
  }
  Ok(canonical)
}

fn radii(stream: TokenStream) -> Result<([Value; 4], [Value; 4]), ValueError> {
  let sides = self::split(stream, '/')?;
  if sides.len() > 2 {
    return Err(self::invalid());
  }
  let horizontal = self::values(sides[0].clone())?;
  let vertical = sides
    .get(1)
    .map(|value| self::values(value.clone()))
    .transpose()?
    .unwrap_or_else(|| horizontal.clone());
  if horizontal.len() > 4 || vertical.len() > 4 {
    return Err(self::invalid());
  }
  for value in horizontal.iter().chain(&vertical) {
    if !self::nonnegative_length_percentage(value) {
      return Err(self::invalid());
    }
  }
  let horizontal = self::expand(&horizontal);
  let vertical = self::expand(&vertical);
  if sides.len() == 2 && horizontal == vertical {
    return Err(self::redundant());
  }
  Ok((horizontal, vertical))
}

fn circle(arguments: &Value) -> Result<Vec<u8>, ValueError> {
  let values = self::atoms(arguments);
  let (radius, position) = self::at(&values)?;
  if radius.len() > 1
    || radius
      .first()
      .is_some_and(|value| !self::shape_radius(value))
  {
    return Err(self::invalid());
  }
  if radius.first().and_then(|value| self::keyword(value)) == Some("closest-side") {
    return Err(self::redundant());
  }
  self::position(position)?;
  let mut canonical = vec![
    43,
    u8::try_from(radius.len()).expect("circle radius count overflow"),
  ];
  for value in radius {
    encode::value(value, &mut canonical);
  }
  position::encode(position, &mut canonical);
  Ok(canonical)
}

fn ellipse(arguments: &Value) -> Result<Vec<u8>, ValueError> {
  let values = self::atoms(arguments);
  let (radii, position) = self::at(&values)?;
  let keyword = radii.len() == 1
    && matches!(
      self::keyword(radii[0]),
      Some("closest-side" | "farthest-side")
    );
  let explicit = radii.len() == 2
    && radii
      .iter()
      .all(|value| self::nonnegative_length_percentage(value));
  if !radii.is_empty() && !keyword && !explicit {
    return Err(self::invalid());
  }
  if radii.first().and_then(|value| self::keyword(value)) == Some("closest-side") {
    return Err(self::redundant());
  }
  self::position(position)?;
  let mut canonical = vec![
    44,
    u8::try_from(radii.len()).expect("ellipse radius count overflow"),
  ];
  for value in radii {
    encode::value(value, &mut canonical);
  }
  position::encode(position, &mut canonical);
  Ok(canonical)
}

fn polygon(arguments: &Value) -> Result<(), ValueError> {
  let items = match arguments {
    Value::Comma(values) => values.as_slice(),
    _ => return Err(self::invalid()),
  };
  let first_point = match items.first().and_then(self::keyword) {
    Some("nonzero") => return Err(self::redundant()),
    Some("evenodd") => 1,
    Some(_) => return Err(self::invalid()),
    None => 0,
  };
  let points = &items[first_point..];
  if points.len() < 3 {
    return Err(self::invalid());
  }
  let points = points
    .iter()
    .map(|point| {
      let values = self::atoms(point);
      if values.len() != 2 || !values.iter().all(|value| self::length_percentage(value)) {
        return Err(self::invalid());
      }
      Ok([values[0], values[1]])
    })
    .collect::<Result<Vec<_>, _>>()?;
  for index in 0..points.len() {
    if self::same_point(points[index], points[(index + 1) % points.len()]) {
      return Err(self::invalid());
    }
  }
  if let Some((area, scale)) = self::area(&points) {
    let tolerance = f64::EPSILON * scale * scale * points.len() as f64 * 16.0;
    if area.abs() <= tolerance {
      return Err(self::invalid());
    }
  }
  Ok(())
}

fn at<'a>(values: &'a [&Value]) -> Result<(&'a [&'a Value], &'a [&'a Value]), ValueError> {
  if let Some(index) = values
    .iter()
    .position(|value| self::keyword(value) == Some("at"))
  {
    if values[index + 1..].is_empty() {
      return Err(self::invalid());
    }
    Ok((&values[..index], &values[index + 1..]))
  } else {
    Ok((values, &[]))
  }
}

fn position(values: &[&Value]) -> Result<(), ValueError> {
  if values.is_empty() {
    return Ok(());
  }
  if !position::valid(values) {
    return Err(self::invalid());
  }
  if position::is_center(values) {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn shape_radius(value: &Value) -> bool {
  matches!(self::keyword(value), Some("closest-side" | "farthest-side"))
    || self::nonnegative_length_percentage(value)
}

fn nonnegative_length_percentage(value: &Value) -> bool {
  self::length_percentage(value) && !self::negative(value)
}

fn length_percentage(value: &Value) -> bool {
  match value {
    Value::Scalar(Scalar { value, unit }) => {
      (*value == 0.0 && *unit == Unit::Number)
        || matches!(
          unit,
          Unit::Percent
            | Unit::Px
            | Unit::Em
            | Unit::Rem
            | Unit::Vw
            | Unit::Vh
            | Unit::Vmin
            | Unit::Vmax
        )
    }
    Value::Calculation(value) => matches!(
      value.dimension,
      Dimension::Length | Dimension::Percentage | Dimension::LengthPercentage
    ),
    _ => false,
  }
}

fn negative(value: &Value) -> bool {
  match value {
    Value::Scalar(value) => value.value < 0.0,
    Value::Calculation(value) => value.constant.is_some_and(|value| value < 0.0),
    _ => false,
  }
}

fn area(points: &[[&Value; 2]]) -> Option<(f64, f64)> {
  let resolved = points
    .iter()
    .map(|point| Some([self::coordinate(point[0])?, self::coordinate(point[1])?]))
    .collect::<Option<Vec<_>>>()?;
  if !self::compatible_axis(&resolved, 0) || !self::compatible_axis(&resolved, 1) {
    return None;
  }
  let area = resolved
    .iter()
    .zip(resolved.iter().cycle().skip(1))
    .take(resolved.len())
    .map(|(left, right)| left[0].0 * right[1].0 - right[0].0 * left[1].0)
    .sum();
  let scale = resolved
    .iter()
    .flat_map(|point| point.iter())
    .map(|coordinate| coordinate.0.abs())
    .fold(0.0, f64::max);
  Some((area, scale))
}

fn compatible_axis(points: &[[(f64, Unit); 2]], axis: usize) -> bool {
  let unit = points
    .iter()
    .find(|point| point[axis].0 != 0.0)
    .map(|point| point[axis].1);
  points
    .iter()
    .all(|point| point[axis].0 == 0.0 || Some(point[axis].1) == unit)
}

fn coordinate(value: &Value) -> Option<(f64, Unit)> {
  let coordinate = match value {
    Value::Scalar(value) => Some((value.value, value.unit)),
    Value::Calculation(Calculation {
      basis: Some(unit),
      constant: Some(value),
      ..
    }) => Some((*value, *unit)),
    _ => None,
  }?;
  Some(if coordinate.0 == 0.0 {
    (0.0, Unit::Number)
  } else {
    coordinate
  })
}

fn same_point(left: [&Value; 2], right: [&Value; 2]) -> bool {
  (0..2).all(|index| {
    match (
      self::coordinate(left[index]),
      self::coordinate(right[index]),
    ) {
      (Some(left), Some(right)) => left == right,
      _ => left[index] == right[index],
    }
  })
}

fn split_round(stream: TokenStream) -> Result<(TokenStream, Option<TokenStream>), ValueError> {
  let mut before = TokenStream::new();
  let mut after = None;
  for token in stream {
    if matches!(&token, TokenTree::Ident(value) if value.to_string().eq_ignore_ascii_case("round"))
    {
      if after.is_some() || before.is_empty() {
        return Err(self::invalid());
      }
      after = Some(TokenStream::new());
    } else if let Some(stream) = &mut after {
      stream.extend([token]);
    } else {
      before.extend([token]);
    }
  }
  if after.as_ref().is_some_and(TokenStream::is_empty) {
    Err(self::invalid())
  } else {
    Ok((before, after))
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

fn values(stream: TokenStream) -> Result<Vec<Value>, ValueError> {
  match super::parse_stream(stream)? {
    Value::Space(values) => Ok(values),
    Value::Comma(_) => Err(self::invalid()),
    value => Ok(vec![value]),
  }
}

fn atoms(value: &Value) -> Vec<&Value> {
  match value {
    Value::Space(values) => values.iter().collect(),
    value => vec![value],
  }
}

fn expand(values: &[Value]) -> [Value; 4] {
  match values {
    [all] => [all.clone(), all.clone(), all.clone(), all.clone()],
    [vertical, horizontal] => [
      vertical.clone(),
      horizontal.clone(),
      vertical.clone(),
      horizontal.clone(),
    ],
    [top, horizontal, bottom] => [
      top.clone(),
      horizontal.clone(),
      bottom.clone(),
      horizontal.clone(),
    ],
    [top, right, bottom, left] => [top.clone(), right.clone(), bottom.clone(), left.clone()],
    _ => unreachable!("validated one-to-four value list"),
  }
}

fn encode_values(values: &[Value], canonical: &mut Vec<u8>) {
  for value in values {
    encode::value(value, canonical);
  }
}

fn keyword(value: &Value) -> Option<&str> {
  if let Value::Keyword(value) = value {
    Some(value)
  } else {
    None
  }
}

fn invalid() -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidValue,
    span: Span::call_site().into(),
  }
}

fn redundant() -> ValueError {
  ValueError {
    category: DiagnosticCategory::RedundantDefault,
    span: Span::call_site().into(),
  }
}
