use proc_macro2::{Span, TokenStream};

use crate::{DiagnosticCategory, canonical};

use super::{Value, ValueError};

#[derive(Clone, Copy)]
enum Token {
  Command(u8),
  Number(f64),
}

struct Segment {
  command: u8,
  values: Vec<f64>,
}

pub(super) fn parse(arguments: TokenStream) -> Result<Vec<u8>, ValueError> {
  let Value::String(source) = super::parse_stream(arguments)? else {
    return Err(self::invalid());
  };
  let segments = self::segments(&self::tokens(&source)?)?;
  let mut canonical = vec![45];
  canonical.extend(
    u32::try_from(segments.len())
      .expect("clip path segment count overflow")
      .to_be_bytes(),
  );
  for segment in segments {
    canonical.push(segment.command);
    for value in segment.values {
      canonical::number(&mut canonical, value);
    }
  }
  Ok(canonical)
}

fn tokens(source: &str) -> Result<Vec<Token>, ValueError> {
  let bytes = source.as_bytes();
  let mut tokens = Vec::new();
  let mut index = 0;
  let mut comma = false;
  while index < bytes.len() {
    if bytes[index].is_ascii_whitespace() {
      index += 1;
      continue;
    }
    if bytes[index] == b',' {
      if comma || !matches!(tokens.last(), Some(Token::Number(_))) {
        return Err(self::invalid());
      }
      comma = true;
      index += 1;
      continue;
    }
    if bytes[index].is_ascii_alphabetic() {
      if comma {
        return Err(self::invalid());
      }
      tokens.push(Token::Command(self::command(bytes[index])?));
      index += 1;
      continue;
    }
    if !matches!(bytes[index], b'+' | b'-' | b'.' | b'0'..=b'9') {
      return Err(self::invalid());
    }
    let (value, next) = self::number(source, index)?;
    tokens.push(Token::Number(value));
    comma = false;
    index = next;
  }
  if comma || tokens.is_empty() {
    Err(self::invalid())
  } else {
    Ok(tokens)
  }
}

fn command(value: u8) -> Result<u8, ValueError> {
  match value {
    b'M' => Ok(1),
    b'L' => Ok(2),
    b'H' => Ok(3),
    b'V' => Ok(4),
    b'C' => Ok(5),
    b'Q' => Ok(6),
    b'A' => Ok(7),
    b'Z' => Ok(8),
    _ => Err(self::invalid()),
  }
}

fn number(source: &str, start: usize) -> Result<(f64, usize), ValueError> {
  let bytes = source.as_bytes();
  let mut index = start;
  if matches!(bytes[index], b'+' | b'-') {
    index += 1;
  }
  let integer_start = index;
  while index < bytes.len() && bytes[index].is_ascii_digit() {
    index += 1;
  }
  let integer_digits = index - integer_start;
  let mut fraction_digits = 0;
  if bytes.get(index) == Some(&b'.') {
    index += 1;
    let fraction_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
      index += 1;
    }
    fraction_digits = index - fraction_start;
  }
  if integer_digits == 0 && fraction_digits == 0 {
    return Err(self::invalid());
  }
  if matches!(bytes.get(index), Some(b'e' | b'E')) {
    index += 1;
    if matches!(bytes.get(index), Some(b'+' | b'-')) {
      index += 1;
    }
    let exponent_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
      index += 1;
    }
    if index == exponent_start {
      return Err(self::invalid());
    }
  }
  let value = source[start..index]
    .parse::<f64>()
    .ok()
    .filter(|value| value.is_finite())
    .ok_or_else(self::invalid)?;
  Ok((if value == 0.0 { 0.0 } else { value }, index))
}

fn segments(tokens: &[Token]) -> Result<Vec<Segment>, ValueError> {
  if !matches!(tokens.first(), Some(Token::Command(1))) {
    return Err(self::invalid());
  }
  let mut segments = Vec::new();
  let mut drawing = false;
  let mut index = 0;
  while index < tokens.len() {
    let Token::Command(command) = tokens[index] else {
      return Err(self::invalid());
    };
    index += 1;
    let start = index;
    while index < tokens.len() && matches!(tokens[index], Token::Number(_)) {
      index += 1;
    }
    let values = &tokens[start..index];
    if command == 8 {
      if !values.is_empty() {
        return Err(self::invalid());
      }
      segments.push(Segment {
        command,
        values: Vec::new(),
      });
      continue;
    }
    let arity = self::arity(command);
    if values.is_empty() || values.len() % arity != 0 {
      return Err(self::invalid());
    }
    for (group_index, group) in values.chunks(arity).enumerate() {
      let command = if command == 1 && group_index > 0 {
        2
      } else {
        command
      };
      let values = group
        .iter()
        .map(|token| match token {
          Token::Number(value) => *value,
          Token::Command(_) => unreachable!("path value group contains only numbers"),
        })
        .collect::<Vec<_>>();
      self::validate(command, &values)?;
      drawing |= command != 1;
      segments.push(Segment { command, values });
    }
  }
  if drawing {
    Ok(segments)
  } else {
    Err(self::invalid())
  }
}

fn arity(command: u8) -> usize {
  match command {
    1 | 2 => 2,
    3 | 4 => 1,
    5 => 6,
    6 => 4,
    7 => 7,
    _ => unreachable!("drawing path command"),
  }
}

fn validate(command: u8, values: &[f64]) -> Result<(), ValueError> {
  if command != 7 {
    return Ok(());
  }
  let invalid_radius = values[0] < 0.0 || values[1] < 0.0;
  let invalid_flags = !matches!(values[3], 0.0 | 1.0) || !matches!(values[4], 0.0 | 1.0);
  if invalid_radius || invalid_flags {
    Err(self::invalid())
  } else {
    Ok(())
  }
}

fn invalid() -> ValueError {
  ValueError {
    category: DiagnosticCategory::InvalidValue,
    span: Span::call_site().into(),
  }
}
