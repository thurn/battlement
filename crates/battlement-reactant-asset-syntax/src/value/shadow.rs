use proc_macro2::Span;

use crate::DiagnosticCategory;

use super::{
  Calculation, Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError, encode,
};

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  let value = super::parse(source)?;
  if self::keyword(&value) == Some("none") {
    return Err(self::redundant());
  }
  let layers = match &value {
    Value::Comma(values) => values.as_slice(),
    value => std::slice::from_ref(value),
  };
  let mut canonical = vec![42];
  canonical.extend(
    u32::try_from(layers.len())
      .expect("shadow layer count overflow")
      .to_be_bytes(),
  );
  for layer in layers {
    self::layer(layer, &mut canonical)?;
  }
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: property.to_owned(),
      canonical,
    }],
    dependencies: Vec::new(),
    relation: None,
  })
}

fn layer(value: &Value, canonical: &mut Vec<u8>) -> Result<(), ValueError> {
  let values = self::atoms(value);
  let mut inset = false;
  let mut color = None;
  let mut lengths = Vec::new();
  for value in values {
    if self::keyword(value) == Some("inset") {
      if inset {
        return Err(self::invalid());
      }
      inset = true;
    } else if matches!(value, Value::Color(_)) {
      if color.replace(value).is_some() {
        return Err(self::invalid());
      }
    } else if self::length(value) {
      lengths.push(value);
    } else {
      return Err(self::invalid());
    }
  }
  let Some(color) = color else {
    return Err(self::invalid());
  };
  if !(2..=4).contains(&lengths.len()) {
    return Err(self::invalid());
  }
  if lengths.get(2).is_some_and(|value| self::negative(value)) {
    return Err(self::invalid());
  }
  if lengths.len() == 3 && self::zero(lengths[2]) {
    return Err(self::redundant());
  }
  if lengths.len() == 4 && self::zero(lengths[3]) {
    return Err(self::redundant());
  }
  canonical.push(u8::from(inset));
  for index in 0..4 {
    if let Some(length) = lengths.get(index) {
      self::encode_length(length, canonical);
    } else {
      encode::value(
        &Value::Scalar(Scalar {
          value: 0.0,
          unit: Unit::Number,
        }),
        canonical,
      );
    }
  }
  encode::value(color, canonical);
  Ok(())
}

fn length(value: &Value) -> bool {
  match value {
    Value::Scalar(Scalar { value, unit }) => {
      (*value == 0.0 && *unit == Unit::Number)
        || matches!(
          unit,
          Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax
        )
    }
    Value::Calculation(Calculation { dimension, .. }) => *dimension == Dimension::Length,
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

fn zero(value: &Value) -> bool {
  match value {
    Value::Scalar(value) => value.value == 0.0,
    Value::Calculation(value) => value.constant == Some(0.0),
    _ => false,
  }
}

fn encode_length(value: &Value, canonical: &mut Vec<u8>) {
  if self::zero(value) {
    encode::value(
      &Value::Scalar(Scalar {
        value: 0.0,
        unit: Unit::Number,
      }),
      canonical,
    );
  } else {
    encode::value(value, canonical);
  }
}

fn atoms(value: &Value) -> Vec<&Value> {
  match value {
    Value::Space(values) => values.iter().collect(),
    value => vec![value],
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
