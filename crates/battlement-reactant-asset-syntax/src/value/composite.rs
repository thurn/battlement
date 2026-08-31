use proc_macro2::Span;

use crate::{
  DiagnosticCategory, SourceSpan,
  value::{
    self, Dimension, ParsedField, ParsedRelation, ParsedValue, Scalar, Unit, Value, ValueError,
    encode,
  },
};

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  match property {
    "background-blend-mode" => self::blend(source),
    "isolation" => self::isolation(source),
    "opacity" => self::opacity(source),
    _ => unreachable!("compositing property routing"),
  }
}

pub(super) fn blend_canonical(modes: &[u8]) -> Vec<u8> {
  let mut canonical = vec![47];
  canonical.extend(
    u32::try_from(modes.len())
      .expect("background blend mode count overflow")
      .to_be_bytes(),
  );
  canonical.extend(modes);
  canonical
}

fn blend(source: &str) -> Result<ParsedValue, ValueError> {
  let values = match value::parse(source)? {
    Value::Comma(values) => values,
    value => vec![value],
  };
  let modes = values
    .iter()
    .map(self::blend_tag)
    .collect::<Option<Vec<_>>>()
    .ok_or_else(self::invalid)?;
  if modes.contains(&0) {
    return Err(self::redundant());
  }
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: "background-blend-mode".to_owned(),
      canonical: self::blend_canonical(&modes),
    }],
    dependencies: Vec::new(),
    relation: Some(ParsedRelation::BlendModes(modes)),
  })
}

fn isolation(source: &str) -> Result<ParsedValue, ValueError> {
  let value = value::parse(source)?;
  let canonical = match self::keyword(&value) {
    Some("isolate") => vec![48, 1],
    Some("auto") => return Err(self::redundant()),
    _ => return Err(self::invalid()),
  };
  self::simple("isolation", canonical)
}

fn opacity(source: &str) -> Result<ParsedValue, ValueError> {
  let value = value::parse(source)?;
  let number = match &value {
    Value::Scalar(Scalar {
      value,
      unit: Unit::Number,
    }) => Some(*value),
    Value::Calculation(calculation) if calculation.dimension == Dimension::Number => {
      calculation.constant
    }
    _ => None,
  }
  .filter(|value| (0.0..=1.0).contains(value))
  .ok_or_else(self::invalid)?;
  if number == 1.0 {
    return Err(self::redundant());
  }
  let mut canonical = vec![49];
  encode::value(&value, &mut canonical);
  self::simple("opacity", canonical)
}

fn simple(property: &str, canonical: Vec<u8>) -> Result<ParsedValue, ValueError> {
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: property.to_owned(),
      canonical,
    }],
    dependencies: Vec::new(),
    relation: None,
  })
}

fn blend_tag(value: &Value) -> Option<u8> {
  Some(match self::keyword(value)? {
    "normal" => 0,
    "multiply" => 1,
    "screen" => 2,
    "overlay" => 3,
    "darken" => 4,
    "lighten" => 5,
    "color-dodge" => 6,
    "color-burn" => 7,
    "hard-light" => 8,
    "soft-light" => 9,
    "difference" => 10,
    "exclusion" => 11,
    "hue" => 12,
    "saturation" => 13,
    "color" => 14,
    "luminosity" => 15,
    _ => return None,
  })
}

fn keyword(value: &Value) -> Option<&str> {
  match value {
    Value::Keyword(value) => Some(value),
    _ => None,
  }
}

fn invalid() -> ValueError {
  self::error(DiagnosticCategory::InvalidValue)
}

fn redundant() -> ValueError {
  self::error(DiagnosticCategory::RedundantDefault)
}

fn error(category: DiagnosticCategory) -> ValueError {
  ValueError {
    category,
    span: SourceSpan::from(Span::call_site()),
  }
}
