use proc_macro2::Span;

use crate::{
  DiagnosticCategory, SourceSpan,
  value::{
    self, Calculation, Dimension, ParsedField, ParsedRelation, ParsedValue, Scalar, Unit, Value,
    ValueError, encode,
  },
};

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  let value = value::parse(source)?;
  let relation = match property {
    "content" => self::content(&value)?,
    "padding" => self::padding(&value)?,
    "font-size" => self::font_size(&value)?,
    "font-style" => self::font_style(&value)?,
    "font-weight" => self::font_weight(&value)?,
    "font-stretch" => self::font_stretch(&value)?,
    "line-height" => self::line_height(&value)?,
    "letter-spacing" | "word-spacing" => self::spacing(&value)?,
    "text-align" => self::keyword_value(
      &value,
      &["center", "end", "justify", "left", "right"],
      "start",
    )?,
    "white-space" => self::keyword_value(&value, &["pre", "pre-wrap", "nowrap"], "normal")?,
    "color" => self::color(&value)?,
    "background-clip" => self::background_clip(&value)?,
    "-webkit-text-stroke" => self::stroke(&value)?,
    _ => unreachable!("text property routing"),
  };
  let mut canonical = vec![self::tag(property)];
  encode::value(&value, &mut canonical);
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: property.to_owned(),
      canonical,
    }],
    dependencies: Vec::new(),
    relation,
  })
}

fn padding(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  let values = self::atoms(value);
  if (1..=4).contains(&values.len()) && values.iter().all(|value| self::nonnegative_length(value)) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn content(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if matches!(value, Value::String(_)) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn font_size(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if self::positive_length(value) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn font_style(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  let values = self::atoms(value);
  let valid = matches!(values.as_slice(), [value] if matches!(self::keyword(value), Some("normal" | "italic")))
    || matches!(values.as_slice(), [value] if self::keyword(value) == Some("oblique"))
    || matches!(values.as_slice(), [first, angle] if self::keyword(first) == Some("oblique") && self::angle(angle));
  if valid {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn font_weight(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if self::number(value)
    .is_some_and(|value| value.fract() == 0.0 && (1.0..=1000.0).contains(&value))
  {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn font_stretch(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if matches!(value, Value::Scalar(Scalar { value, unit: Unit::Percent }) if (50.0..=200.0).contains(value))
  {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn line_height(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if self::keyword(value) == Some("normal") {
    return Err(self::redundant());
  }
  if self::nonnegative_length(value) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn spacing(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if self::keyword(value) == Some("normal") {
    return Err(self::redundant());
  }
  if self::length(value) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn keyword_value(
  value: &Value,
  accepted: &[&str],
  default: &str,
) -> Result<Option<ParsedRelation>, ValueError> {
  if self::keyword(value) == Some(default) {
    return Err(self::redundant());
  }
  if self::keyword(value).is_some_and(|value| accepted.contains(&value)) {
    Ok(None)
  } else {
    Err(self::invalid())
  }
}

fn color(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  let Value::Color(channels) = value else {
    return Err(self::invalid());
  };
  if *channels == [0.0, 0.0, 0.0, 1.0] {
    return Err(self::redundant());
  }
  Ok(Some(ParsedRelation::TextColorTransparent(
    channels[3] == 0.0,
  )))
}

fn background_clip(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  if self::keyword(value) == Some("text") {
    Ok(Some(ParsedRelation::TextClip))
  } else {
    Err(self::invalid())
  }
}

fn stroke(value: &Value) -> Result<Option<ParsedRelation>, ValueError> {
  let values = self::atoms(value);
  if values.len() != 2 {
    return Err(self::invalid());
  }
  let width = values.iter().find(|value| self::nonnegative_length(value));
  let color = values.iter().find(|value| matches!(value, Value::Color(_)));
  if width.is_none() || color.is_none() {
    return Err(self::invalid());
  }
  if width.is_some_and(|value| self::zero(value)) {
    Err(self::redundant())
  } else {
    Ok(None)
  }
}

fn positive_length(value: &Value) -> bool {
  self::length(value) && self::constant(value).is_none_or(|value| value > 0.0)
}

fn nonnegative_length(value: &Value) -> bool {
  self::length(value) && self::constant(value).is_none_or(|value| value >= 0.0)
}

fn length(value: &Value) -> bool {
  matches!(
    value,
    Value::Scalar(Scalar {
      value: 0.0,
      unit: Unit::Number
    })
  ) || matches!(
    value,
    Value::Scalar(Scalar {
      unit: Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax,
      ..
    })
  ) || matches!(
    value,
    Value::Calculation(Calculation {
      dimension: Dimension::Length,
      ..
    })
  )
}

fn angle(value: &Value) -> bool {
  matches!(
    value,
    Value::Scalar(Scalar {
      value: 0.0,
      unit: Unit::Number
    })
  ) || matches!(
    value,
    Value::Scalar(Scalar {
      unit: Unit::Deg | Unit::Grad | Unit::Rad | Unit::Turn,
      ..
    })
  ) || matches!(
    value,
    Value::Calculation(Calculation {
      dimension: Dimension::Angle,
      ..
    })
  )
}

fn number(value: &Value) -> Option<f64> {
  match value {
    Value::Scalar(Scalar {
      value,
      unit: Unit::Number,
    }) => Some(*value),
    Value::Calculation(value) if value.dimension == Dimension::Number => value.constant,
    _ => None,
  }
}

fn constant(value: &Value) -> Option<f64> {
  match value {
    Value::Scalar(value) => Some(value.value),
    Value::Calculation(value) => value.constant,
    _ => None,
  }
}

fn zero(value: &Value) -> bool {
  self::constant(value) == Some(0.0)
}

fn atoms(value: &Value) -> Vec<&Value> {
  match value {
    Value::Space(values) => values.iter().collect(),
    value => vec![value],
  }
}

fn keyword(value: &Value) -> Option<&str> {
  match value {
    Value::Keyword(value) => Some(value),
    _ => None,
  }
}

fn tag(property: &str) -> u8 {
  match property {
    "padding" => 66,
    "content" => 53,
    "font-size" => 54,
    "font-style" => 55,
    "font-weight" => 56,
    "font-stretch" => 57,
    "line-height" => 58,
    "letter-spacing" => 59,
    "word-spacing" => 60,
    "text-align" => 61,
    "white-space" => 62,
    "color" => 63,
    "background-clip" => 64,
    "-webkit-text-stroke" => 65,
    _ => unreachable!("text property tag"),
  }
}

fn invalid() -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidValue,
    span: SourceSpan::from(Span::call_site()),
  }
}

fn redundant() -> ValueError {
  ValueError {
    category: DiagnosticCategory::RedundantDefault,
    span: SourceSpan::from(Span::call_site()),
  }
}
