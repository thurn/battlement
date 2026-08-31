use std::fmt;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::LitStr;

use super::{CalcNode, Calculation, Unit, Value};

pub(super) fn css_tokens(stream: TokenStream) -> String {
  let tokens = stream.into_iter().collect::<Vec<_>>();
  let mut output = String::new();
  for (index, token) in tokens.iter().enumerate() {
    match token {
      TokenTree::Ident(value) => {
        self::separate_word(&mut output);
        output.push_str(&value.to_string());
      }
      TokenTree::Literal(value) => {
        self::separate_word(&mut output);
        let source = value.to_string();
        if let Ok(string) = syn::parse_str::<LitStr>(&source) {
          output.push_str(&format!("{:?}", string.value()));
        } else {
          output.push_str(&source);
        }
      }
      TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
        self::trim_space(&mut output);
        output.push('(');
        output.push_str(&self::css_tokens(group.stream()));
        self::trim_space(&mut output);
        output.push(')');
      }
      TokenTree::Group(group) => {
        self::separate_word(&mut output);
        output.push_str(&group.to_string());
      }
      TokenTree::Punct(punctuation) => {
        let character = punctuation.as_char();
        let previous_word = index
          .checked_sub(1)
          .and_then(|previous| tokens.get(previous))
          .is_some_and(|token| matches!(token, TokenTree::Ident(_)));
        let next_word = tokens
          .get(index + 1)
          .is_some_and(|token| matches!(token, TokenTree::Ident(_)));
        let unary = index == 0
          || tokens.get(index - 1).is_some_and(|token| {
            matches!(token, TokenTree::Punct(value) if matches!(value.as_char(), ',' | '/' | '+' | '-' | '*'))
          });
        match character {
          ',' => {
            self::trim_space(&mut output);
            output.push_str(", ");
          }
          '/' => {
            self::trim_space(&mut output);
            output.push_str(" / ");
          }
          '#' => {
            self::trim_space(&mut output);
            output.push('#');
          }
          '-' if previous_word && next_word => {
            self::trim_space(&mut output);
            output.push('-');
          }
          '-' | '+' if unary => {
            self::trim_space(&mut output);
            output.push(character);
          }
          '-' | '+' | '*' => {
            self::trim_space(&mut output);
            output.push(' ');
            output.push(character);
            output.push(' ');
          }
          _ => {
            self::trim_space(&mut output);
            output.push(character);
          }
        }
      }
    }
  }
  output.trim().to_owned()
}

fn separate_word(output: &mut String) {
  if output
    .chars()
    .last()
    .is_some_and(|character| character.is_ascii_alphanumeric() || matches!(character, ')' | '"'))
  {
    output.push(' ');
  }
}

fn trim_space(output: &mut String) {
  while output.ends_with(' ') {
    output.pop();
  }
}

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

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use proc_macro2::TokenStream;

  use super::css_tokens;

  #[test]
  fn css_tokens_restore_functions_hyphens_shorthand_slashes_and_calculations() {
    assert_eq!(
      css_tokens(
        TokenStream::from_str(
          "unity-url(\"Assets/a.png\") center / calc(20px - 2px) 10px no-repeat",
        )
        .unwrap()
      ),
      "unity-url(\"Assets/a.png\") center / calc(20px - 2px) 10px no-repeat"
    );
  }
}
