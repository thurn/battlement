use std::fmt;

use super::{CalcNode, Calculation, Unit, Value};

impl fmt::Display for Value {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Scalar(value) => write!(
        formatter,
        "{}{unit}",
        self::decimal(value.value),
        unit = self::unit_name(value.unit)
      ),
      Self::Color([red, green, blue, alpha]) => {
        write!(formatter, "rgba({red} {green} {blue} / {alpha})")
      }
      Self::String(value) => write!(formatter, "{value:?}"),
      Self::Keyword(value) => formatter.write_str(value),
      Self::Function(name, value) => write!(formatter, "{name}({value})"),
      Self::Space(values) => self.values(formatter, values, " "),
      Self::Comma(values) => self.values(formatter, values, ", "),
      Self::Calculation(value) => write!(formatter, "calc({})", CalcDisplay(&value.node)),
    }
  }
}

struct CalcDisplay<'a>(&'a CalcNode);

impl fmt::Display for CalcDisplay<'_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.0 {
      CalcNode::Scalar(value) => write!(
        formatter,
        "{}{unit}",
        self::decimal(value.value),
        unit = self::unit_name(value.unit)
      ),
      CalcNode::Add(left, right) => self.binary(formatter, left, "+", right),
      CalcNode::Subtract(left, right) => self.binary(formatter, left, "-", right),
      CalcNode::Multiply(left, right) => self.binary(formatter, left, "*", right),
      CalcNode::Divide(left, right) => self.binary(formatter, left, "/", right),
      CalcNode::Min(values) => self.calculations(formatter, "min", values),
      CalcNode::Max(values) => self.calculations(formatter, "max", values),
      CalcNode::Clamp(values) => self.calculations(formatter, "clamp", values),
      CalcNode::Group(value) => write!(formatter, "({})", CalcDisplay(&value.node)),
    }
  }
}

fn decimal(value: f64) -> String {
  if value == 0.0 {
    "0".to_owned()
  } else {
    value.to_string()
  }
}

fn unit_name(unit: Unit) -> &'static str {
  match unit {
    Unit::Number => "",
    Unit::Percent => "%",
    Unit::Px => "px",
    Unit::Em => "em",
    Unit::Rem => "rem",
    Unit::Vw => "vw",
    Unit::Vh => "vh",
    Unit::Vmin => "vmin",
    Unit::Vmax => "vmax",
    Unit::Deg => "deg",
    Unit::Grad => "grad",
    Unit::Rad => "rad",
    Unit::Turn => "turn",
  }
}

trait FormatValues {
  fn values(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    values: &[Value],
    separator: &str,
  ) -> fmt::Result;
  fn binary(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    left: &Calculation,
    operator: &str,
    right: &Calculation,
  ) -> fmt::Result;
  fn calculations(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    name: &str,
    values: &[Calculation],
  ) -> fmt::Result;
}

impl<T> FormatValues for T {
  fn values(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    values: &[Value],
    separator: &str,
  ) -> fmt::Result {
    for (index, value) in values.iter().enumerate() {
      if index > 0 {
        formatter.write_str(separator)?;
      }
      write!(formatter, "{value}")?;
    }
    Ok(())
  }

  fn binary(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    left: &Calculation,
    operator: &str,
    right: &Calculation,
  ) -> fmt::Result {
    write!(
      formatter,
      "{} {operator} {}",
      CalcDisplay(&left.node),
      CalcDisplay(&right.node)
    )
  }

  fn calculations(
    &self,
    formatter: &mut fmt::Formatter<'_>,
    name: &str,
    values: &[Calculation],
  ) -> fmt::Result {
    write!(formatter, "{name}(")?;
    for (index, value) in values.iter().enumerate() {
      if index > 0 {
        formatter.write_str(", ")?;
      }
      write!(formatter, "{}", CalcDisplay(&value.node))?;
    }
    formatter.write_str(")")
  }
}
