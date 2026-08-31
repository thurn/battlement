use proc_macro2::Span;

use crate::DiagnosticCategory;

use super::{Calculation, Dimension, Scalar, Unit, Value, ValueError};

#[derive(Clone, Copy)]
pub(super) enum Gradient {
  Linear,
  Radial,
  Conic,
}

pub(super) fn validate(arguments: &Value, kind: Gradient) -> Result<(), ValueError> {
  let Value::Comma(items) = arguments else {
    return Err(self::invalid());
  };
  let mut first_stop = 0;
  if !self::color_stop(&items[0], kind) {
    self::prelude(&items[0], kind)?;
    first_stop = 1;
  }
  let stops = &items[first_stop..];
  let mut color_count = 0;
  let mut previous_hint = false;
  for (index, stop) in stops.iter().enumerate() {
    if self::color_stop(stop, kind) {
      color_count += 1;
      previous_hint = false;
      continue;
    }
    let invalid_position = index == 0 || index + 1 == stops.len();
    if invalid_position || previous_hint || !self::hint(stop, kind) {
      return Err(self::invalid());
    }
    previous_hint = true;
  }
  if color_count < 2 {
    Err(self::invalid())
  } else {
    Ok(())
  }
}

fn prelude(value: &Value, kind: Gradient) -> Result<(), ValueError> {
  let values = self::atoms(value);
  match kind {
    Gradient::Linear => self::linear(&values),
    Gradient::Radial => self::radial(&values),
    Gradient::Conic => self::conic(&values),
  }
}

fn linear(values: &[&Value]) -> Result<(), ValueError> {
  if values.len() == 1 && self::angle(values[0]) {
    return if self::angle_degrees(values[0]) == Some(180.0) {
      Err(self::redundant())
    } else {
      Ok(())
    };
  }
  if values.first().and_then(|value| self::keyword(value)) != Some("to") {
    return Err(self::invalid());
  }
  let directions = &values[1..];
  let all_directions = (1..=2).contains(&directions.len())
    && directions.iter().all(|value| {
      matches!(
        self::keyword(value),
        Some("left" | "right" | "top" | "bottom")
      )
    });
  let compatible_pair =
    directions.len() < 2 || self::horizontal(directions[0]) != self::horizontal(directions[1]);
  if !all_directions || !compatible_pair {
    return Err(self::invalid());
  }
  if directions.len() == 1 && self::keyword(directions[0]) == Some("bottom") {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn radial(values: &[&Value]) -> Result<(), ValueError> {
  let at = values
    .iter()
    .position(|value| self::keyword(value) == Some("at"));
  let (shape_size, position) = at.map_or((values, &[][..]), |index| {
    (&values[..index], &values[index + 1..])
  });
  if at.is_some() {
    if !super::position::valid(position) {
      return Err(self::invalid());
    }
    if super::position::is_center(position) {
      return Err(self::redundant());
    }
  }
  let shape = shape_size
    .first()
    .and_then(|value| self::keyword(value))
    .filter(|value| matches!(*value, "circle" | "ellipse"));
  let sizes = if matches!(shape, Some("circle" | "ellipse")) {
    &shape_size[1..]
  } else {
    shape_size
  };
  if sizes.len() == 1 && self::keyword(sizes[0]) == Some("farthest-corner") {
    return Err(self::redundant());
  }
  let keyword_size = sizes.len() == 1
    && matches!(
      self::keyword(sizes[0]),
      Some("closest-side" | "closest-corner" | "farthest-side" | "farthest-corner")
    );
  let circle_size = sizes.len() == 1 && self::length(sizes[0]);
  let ellipse_size = sizes.len() == 2 && sizes.iter().all(|value| self::length_percentage(value));
  let compatible = match shape {
    Some("circle") => sizes.is_empty() || keyword_size || circle_size,
    Some("ellipse") => sizes.is_empty() || keyword_size || ellipse_size,
    None => shape_size.is_empty() || [keyword_size, circle_size, ellipse_size].contains(&true),
    _ => false,
  };
  if !compatible {
    return Err(self::invalid());
  }
  if shape == Some("ellipse") && sizes.is_empty() && at.is_none() {
    Err(self::redundant())
  } else {
    Ok(())
  }
}

fn conic(values: &[&Value]) -> Result<(), ValueError> {
  let mut index = 0;
  if values.first().and_then(|value| self::keyword(value)) == Some("from") {
    let Some(angle) = values.get(1).filter(|value| self::angle(value)) else {
      return Err(self::invalid());
    };
    if self::angle_degrees(angle) == Some(0.0) {
      return Err(self::redundant());
    }
    index = 2;
  }
  if values.get(index).and_then(|value| self::keyword(value)) == Some("at") {
    let position = &values[index + 1..];
    if !super::position::valid(position) {
      return Err(self::invalid());
    }
    if super::position::is_center(position) {
      return Err(self::redundant());
    }
    index = values.len();
  }
  if index == values.len() {
    Ok(())
  } else {
    Err(self::invalid())
  }
}

fn color_stop(value: &Value, kind: Gradient) -> bool {
  let values = self::atoms(value);
  matches!(values.first(), Some(Value::Color(_)))
    && values.len() <= 3
    && values[1..]
      .iter()
      .all(|value| self::stop_position(value, kind))
}

fn hint(value: &Value, kind: Gradient) -> bool {
  let values = self::atoms(value);
  values.len() == 1 && self::stop_position(values[0], kind)
}

fn stop_position(value: &Value, kind: Gradient) -> bool {
  match kind {
    Gradient::Conic => self::angle(value) || self::percentage(value),
    Gradient::Linear | Gradient::Radial => self::length_percentage(value),
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

fn length(value: &Value) -> bool {
  matches!(value, Value::Scalar(Scalar { value: 0.0, .. }))
    || matches!(
      value,
      Value::Scalar(Scalar {
        unit: Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax,
        ..
      })
    )
    || matches!(
      value,
      Value::Calculation(Calculation {
        dimension: Dimension::Length,
        ..
      })
    )
}

fn percentage(value: &Value) -> bool {
  matches!(
    value,
    Value::Scalar(Scalar {
      unit: Unit::Percent,
      ..
    })
  ) || matches!(
    value,
    Value::Calculation(Calculation {
      dimension: Dimension::Percentage,
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

fn angle_degrees(value: &Value) -> Option<f64> {
  let scalar = match value {
    Value::Scalar(value) => *value,
    Value::Calculation(Calculation {
      basis: Some(unit),
      constant: Some(value),
      ..
    }) => Scalar {
      value: *value,
      unit: *unit,
    },
    _ => return None,
  };
  Some(match scalar.unit {
    Unit::Number | Unit::Deg => scalar.value,
    Unit::Grad => scalar.value * 0.9,
    Unit::Rad => scalar.value.to_degrees(),
    Unit::Turn => scalar.value * 360.0,
    _ => return None,
  })
}

fn horizontal(value: &Value) -> bool {
  matches!(self::keyword(value), Some("left" | "right"))
}

fn keyword(value: &Value) -> Option<&str> {
  match value {
    Value::Keyword(value) => Some(value),
    _ => None,
  }
}

fn atoms(value: &Value) -> Vec<&Value> {
  match value {
    Value::Space(values) => values.iter().collect(),
    value => vec![value],
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
