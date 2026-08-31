use proc_macro2::Span;

use crate::{
  DiagnosticCategory, SourceSpan,
  value::{
    self, Calculation, Dimension, ParsedField, ParsedValue, Scalar, Unit, Value, ValueError,
    encode, position,
  },
};

pub(super) fn parse(property: &str, source: &str) -> Result<ParsedValue, ValueError> {
  let canonical = match property {
    "filter" => self::functions(source, 50, self::filter)?,
    "transform" => self::functions(source, 51, self::transform)?,
    "transform-origin" => self::origin(source)?,
    _ => unreachable!("effect property routing"),
  };
  Ok(ParsedValue {
    fields: vec![ParsedField {
      property: property.to_owned(),
      canonical,
    }],
    dependencies: Vec::new(),
    relation: None,
  })
}

fn functions(
  source: &str,
  tag: u8,
  validate: fn(&str, &Value) -> Result<(), ValueError>,
) -> Result<Vec<u8>, ValueError> {
  let value = value::parse(source)?;
  if self::keyword(&value) == Some("none") {
    return Err(self::redundant());
  }
  let functions = match &value {
    Value::Space(values) => values.as_slice(),
    value => std::slice::from_ref(value),
  };
  for function in functions {
    let Value::Function(name, arguments) = function else {
      return Err(self::invalid());
    };
    validate(name, arguments)?;
  }
  let mut canonical = vec![tag];
  encode::value(&value, &mut canonical);
  Ok(canonical)
}

fn filter(name: &str, arguments: &Value) -> Result<(), ValueError> {
  match name {
    "blur" => self::one(arguments, self::nonnegative_length, self::zero_default),
    "brightness" | "contrast" | "opacity" | "saturate" => {
      self::one(arguments, self::nonnegative_factor, self::one_default)
    }
    "grayscale" | "invert" | "sepia" => self::one(arguments, self::unit_factor, self::zero_default),
    "hue-rotate" => self::one(arguments, self::angle, self::zero_default),
    "drop-shadow" => self::drop_shadow(arguments),
    _ => Err(self::invalid()),
  }
}

fn transform(name: &str, arguments: &Value) -> Result<(), ValueError> {
  let values = self::arguments(arguments);
  match name {
    "translate" => {
      if !(1..=2).contains(&values.len())
        || !values.iter().all(|value| self::length_percentage(value))
      {
        return Err(self::invalid());
      }
      self::reject_all_default(&values, self::zero_default)
    }
    "rotate" => self::one(arguments, self::angle, self::zero_default),
    "scale" => {
      if !(1..=2).contains(&values.len())
        || !values.iter().all(|value| self::number(value).is_some())
      {
        return Err(self::invalid());
      }
      self::reject_all_default(&values, self::one_default)
    }
    "skew" => {
      if !(1..=2).contains(&values.len()) || !values.iter().all(|value| self::angle(value)) {
        return Err(self::invalid());
      }
      self::reject_all_default(&values, self::zero_default)
    }
    "skewx" | "skewy" => self::one(arguments, self::angle, self::zero_default),
    "matrix" => {
      if values.len() != 6 || !values.iter().all(|value| self::number(value).is_some()) {
        return Err(self::invalid());
      }
      let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
      if values
        .iter()
        .zip(identity)
        .all(|(value, expected)| self::number(value) == Some(expected))
      {
        Err(self::redundant())
      } else {
        Ok(())
      }
    }
    _ => Err(self::invalid()),
  }
}

fn origin(source: &str) -> Result<Vec<u8>, ValueError> {
  let value = value::parse(source)?;
  let values = self::arguments(&value);
  let references = values.to_vec();
  if !position::valid(&references) {
    return Err(self::invalid());
  }
  if position::is_center(&references) {
    return Err(self::redundant());
  }
  let mut canonical = vec![52];
  position::encode(&references, &mut canonical);
  Ok(canonical)
}

fn drop_shadow(arguments: &Value) -> Result<(), ValueError> {
  let values = self::arguments(arguments);
  let mut lengths = Vec::new();
  let mut color = false;
  for value in values {
    if matches!(value, Value::Color(_)) {
      if color {
        return Err(self::invalid());
      }
      color = true;
    } else if self::length(value) {
      lengths.push(value);
    } else {
      return Err(self::invalid());
    }
  }
  if !color || !(2..=3).contains(&lengths.len()) {
    return Err(self::invalid());
  }
  if lengths.get(2).is_some_and(|value| self::negative(value)) {
    Err(self::invalid())
  } else {
    Ok(())
  }
}

fn one(
  arguments: &Value,
  valid: fn(&Value) -> bool,
  default: fn(&Value) -> bool,
) -> Result<(), ValueError> {
  let values = self::arguments(arguments);
  if values.len() != 1 || !valid(values[0]) {
    return Err(self::invalid());
  }
  if default(values[0]) {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn reject_all_default(values: &[&Value], default: fn(&Value) -> bool) -> Result<(), ValueError> {
  if values.iter().all(|value| default(value)) {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn arguments(value: &Value) -> Vec<&Value> {
  match value {
    Value::Space(values) | Value::Comma(values) => values.iter().collect(),
    value => vec![value],
  }
}

fn nonnegative_length(value: &Value) -> bool {
  self::length(value) && !self::negative(value)
}

fn length(value: &Value) -> bool {
  match value {
    Value::Scalar(Scalar { value: 0.0, .. }) => true,
    Value::Scalar(value) => matches!(
      value.unit,
      Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax
    ),
    Value::Calculation(value) => value.dimension == Dimension::Length,
    _ => false,
  }
}

fn length_percentage(value: &Value) -> bool {
  self::length(value)
    || matches!(
      value,
      Value::Scalar(Scalar {
        unit: Unit::Percent,
        ..
      })
    )
    || matches!(
      value,
      Value::Calculation(Calculation {
        dimension: Dimension::Percentage | Dimension::LengthPercentage,
        ..
      })
    )
}

fn angle(value: &Value) -> bool {
  matches!(
    value,
    Value::Scalar(Scalar {
      unit: Unit::Deg | Unit::Grad | Unit::Rad | Unit::Turn,
      ..
    })
  ) || matches!(
    value,
    Value::Scalar(Scalar {
      value: 0.0,
      unit: Unit::Number
    })
  ) || matches!(
    value,
    Value::Calculation(Calculation {
      dimension: Dimension::Angle,
      ..
    })
  )
}

fn nonnegative_factor(value: &Value) -> bool {
  self::factor(value).is_some_and(|value| value >= 0.0)
}

fn unit_factor(value: &Value) -> bool {
  self::factor(value).is_some_and(|value| (0.0..=1.0).contains(&value))
}

fn factor(value: &Value) -> Option<f64> {
  match value {
    Value::Scalar(Scalar {
      value,
      unit: Unit::Number,
    }) => Some(*value),
    Value::Scalar(Scalar {
      value,
      unit: Unit::Percent,
    }) => Some(*value / 100.0),
    Value::Calculation(value) if value.dimension == Dimension::Number => value.constant,
    Value::Calculation(value) if value.dimension == Dimension::Percentage => {
      value.constant.map(|value| value / 100.0)
    }
    _ => None,
  }
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

fn negative(value: &Value) -> bool {
  match value {
    Value::Scalar(value) => value.value < 0.0,
    Value::Calculation(value) => value.constant.is_some_and(|value| value < 0.0),
    _ => false,
  }
}

fn zero_default(value: &Value) -> bool {
  self::number(value) == Some(0.0)
    || self::factor(value) == Some(0.0)
    || matches!(value, Value::Scalar(Scalar { value: 0.0, .. }))
    || matches!(
      value,
      Value::Calculation(Calculation {
        constant: Some(0.0),
        ..
      })
    )
}

fn one_default(value: &Value) -> bool {
  self::factor(value) == Some(1.0)
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
