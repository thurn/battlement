use proc_macro2::{Delimiter, Span, TokenStream};

use crate::{
  DiagnosticCategory, SourceSpan,
  token::{Cursor, css_name},
};

use super::{CalcNode, Calculation, Dimension, Unit, ValueError, scalar};

pub(super) fn parse(
  name: &str,
  arguments: TokenStream,
  span: SourceSpan,
) -> Result<Calculation, ValueError> {
  let mut cursor = Cursor::new(arguments);
  let mut values = Vec::new();
  loop {
    values.push(self::sum(&mut cursor)?);
    if !cursor.punct(',') {
      break;
    }
  }
  if !cursor.is_empty() {
    return Err(self::error(cursor.span()));
  }
  match name {
    "calc" if values.len() == 1 => Ok(values.pop().expect("one calculation")),
    "min" | "max" if !values.is_empty() => self::list(name, values),
    "clamp" if values.len() == 3 => self::list(name, values),
    _ => Err(self::error(span)),
  }
}

fn sum(cursor: &mut Cursor) -> Result<Calculation, ValueError> {
  let mut left = self::product(cursor)?;
  loop {
    let operator = if cursor.punct('+') {
      Some(true)
    } else if cursor.punct('-') {
      Some(false)
    } else {
      None
    };
    let Some(add) = operator else { return Ok(left) };
    let right = self::product(cursor)?;
    let dimension = self::sum_dimension(left.dimension, right.dimension)
      .ok_or_else(|| self::error(cursor.span()))?;
    let operation = if add {
      |left, right| left + right
    } else {
      |left, right| left - right
    };
    let (basis, constant) = self::constant_pair(&left, &right, operation)?;
    let node = if add {
      CalcNode::Add(Box::new(left), Box::new(right))
    } else {
      CalcNode::Subtract(Box::new(left), Box::new(right))
    };
    left = Calculation {
      node,
      dimension,
      basis,
      constant,
    };
  }
}

fn product(cursor: &mut Cursor) -> Result<Calculation, ValueError> {
  let mut left = self::primary(cursor)?;
  loop {
    let operator = if cursor.punct('*') {
      Some(true)
    } else if cursor.punct('/') {
      Some(false)
    } else {
      None
    };
    let Some(multiply) = operator else {
      return Ok(left);
    };
    let right = self::primary(cursor)?;
    let dimension = if multiply {
      self::product_dimension(left.dimension, right.dimension)
    } else if right.dimension == Dimension::Number {
      Some(left.dimension)
    } else {
      None
    }
    .ok_or_else(|| self::error(cursor.span()))?;
    if !multiply && right.constant == Some(0.0) {
      return Err(self::error(cursor.span()));
    }
    let (basis, constant) = self::product_constant(&left, &right, multiply)?;
    let node = if multiply {
      CalcNode::Multiply(Box::new(left), Box::new(right))
    } else {
      CalcNode::Divide(Box::new(left), Box::new(right))
    };
    left = Calculation {
      node,
      dimension,
      basis,
      constant,
    };
  }
}

fn primary(cursor: &mut Cursor) -> Result<Calculation, ValueError> {
  if let Some(scalar) = scalar(cursor)? {
    return Ok(Calculation {
      node: CalcNode::Scalar(scalar),
      dimension: self::dimension(scalar.unit),
      basis: Some(scalar.unit),
      constant: Some(scalar.value),
    });
  }
  if let Some((group, span)) = cursor.group(Delimiter::Parenthesis) {
    let calculation = self::parse("calc", group, span)?;
    return Ok(Calculation {
      node: CalcNode::Group(Box::new(calculation.clone())),
      ..calculation
    });
  }
  let span = cursor.span();
  let name = css_name(cursor)
    .map(|name| name.to_ascii_lowercase())
    .ok_or_else(|| self::error(span))?;
  let arguments = cursor
    .group(Delimiter::Parenthesis)
    .ok_or_else(|| self::error(span))?
    .0;
  if matches!(name.as_str(), "calc" | "min" | "max" | "clamp") {
    self::parse(&name, arguments, span)
  } else {
    Err(self::error(span))
  }
}

fn list(name: &str, values: Vec<Calculation>) -> Result<Calculation, ValueError> {
  let dimension = values
    .iter()
    .skip(1)
    .try_fold(values[0].dimension, |dimension, value| {
      self::sum_dimension(dimension, value.dimension)
    })
    .ok_or_else(|| self::error(Span::call_site().into()))?;
  let basis = values
    .iter()
    .map(|value| value.basis)
    .reduce(|left, right| if left == right { left } else { None })
    .flatten();
  let constant = basis.and_then(|_| {
    values
      .iter()
      .map(|value| value.constant)
      .collect::<Option<Vec<_>>>()
      .map(|values| match name {
        "min" => values.into_iter().reduce(f64::min).expect("nonempty min"),
        "max" => values.into_iter().reduce(f64::max).expect("nonempty max"),
        _ => values[1].clamp(values[0], values[2]),
      })
  });
  let node = match name {
    "min" => CalcNode::Min(values),
    "max" => CalcNode::Max(values),
    _ => CalcNode::Clamp(values),
  };
  Ok(Calculation {
    node,
    dimension,
    basis,
    constant,
  })
}

fn sum_dimension(left: Dimension, right: Dimension) -> Option<Dimension> {
  match (left, right) {
    (left, right) if left == right => Some(left),
    (Dimension::Length, Dimension::Percentage)
    | (Dimension::Percentage, Dimension::Length)
    | (Dimension::LengthPercentage, Dimension::Length)
    | (Dimension::Length, Dimension::LengthPercentage)
    | (Dimension::LengthPercentage, Dimension::Percentage)
    | (Dimension::Percentage, Dimension::LengthPercentage) => Some(Dimension::LengthPercentage),
    _ => None,
  }
}

fn product_dimension(left: Dimension, right: Dimension) -> Option<Dimension> {
  match (left, right) {
    (Dimension::Number, right) => Some(right),
    (left, Dimension::Number) => Some(left),
    _ => None,
  }
}

fn dimension(unit: Unit) -> Dimension {
  match unit {
    Unit::Number => Dimension::Number,
    Unit::Percent => Dimension::Percentage,
    Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Vmin | Unit::Vmax => {
      Dimension::Length
    }
    Unit::Deg | Unit::Grad | Unit::Rad | Unit::Turn => Dimension::Angle,
  }
}

fn constant_pair(
  left: &Calculation,
  right: &Calculation,
  operation: fn(f64, f64) -> f64,
) -> Result<(Option<Unit>, Option<f64>), ValueError> {
  if left.basis != right.basis {
    return Ok((None, None));
  }
  self::checked(
    left.basis,
    left
      .constant
      .zip(right.constant)
      .map(|(left, right)| operation(left, right)),
  )
}

fn product_constant(
  left: &Calculation,
  right: &Calculation,
  multiply: bool,
) -> Result<(Option<Unit>, Option<f64>), ValueError> {
  let basis = if left.dimension == Dimension::Number {
    right.basis
  } else {
    left.basis
  };
  let constant = left
    .constant
    .zip(right.constant)
    .map(|(left, right)| if multiply { left * right } else { left / right });
  self::checked(basis, constant)
}

fn checked(
  basis: Option<Unit>,
  constant: Option<f64>,
) -> Result<(Option<Unit>, Option<f64>), ValueError> {
  if constant.is_some_and(|value| !value.is_finite()) {
    Err(self::error(Span::call_site().into()))
  } else {
    Ok((
      basis,
      constant.map(|value| if value == 0.0 { 0.0 } else { value }),
    ))
  }
}

fn error(span: SourceSpan) -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidArithmetic,
    span,
  }
}
