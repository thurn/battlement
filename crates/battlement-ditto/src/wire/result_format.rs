use anyhow::{Context, Result, ensure};
use serde::Serialize;

pub(super) fn canonical_pretty_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
  let ordered = serde_json::to_value(value).context("serialize canonical JSON value")?;
  let mut bytes = Vec::new();
  serde_json::to_writer_pretty(&mut bytes, &ordered).context("serialize canonical pretty JSON")?;
  bytes.push(b'\n');
  Ok(bytes)
}

pub(super) fn canonical_json_line<T: Serialize>(value: &T) -> Result<Vec<u8>> {
  let ordered = serde_json::to_value(value).context("serialize canonical JSON value")?;
  let mut bytes = serde_json::to_vec(&ordered).context("serialize canonical JSON line")?;
  bytes.push(b'\n');
  Ok(bytes)
}

pub(super) fn timestamp(field: &str, value: &str) -> Result<()> {
  ensure!(
    value.len() == 20,
    "{field} must use whole-second RFC 3339 UTC"
  );
  let bytes = value.as_bytes();
  let punctuation = [
    (4, b'-'),
    (7, b'-'),
    (10, b'T'),
    (13, b':'),
    (16, b':'),
    (19, b'Z'),
  ]
  .iter()
  .all(|(index, expected)| bytes[*index] == *expected);
  ensure!(punctuation, "{field} must use whole-second RFC 3339 UTC");
  for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
    ensure!(
      bytes[index].is_ascii_digit(),
      "{field} contains an invalid digit"
    );
  }
  let year = number(bytes, 0, 4);
  let month = number(bytes, 5, 2);
  let day = number(bytes, 8, 2);
  let hour = number(bytes, 11, 2);
  let minute = number(bytes, 14, 2);
  let second = number(bytes, 17, 2);
  ensure!(year > 0, "{field} year must be positive");
  ensure!((1..=12).contains(&month), "{field} month is invalid");
  ensure!(
    day > 0 && day <= days_in_month(year, month),
    "{field} day is invalid"
  );
  ensure!(hour <= 23 && minute <= 59, "{field} time is invalid");
  ensure!(second <= 59, "{field} leap seconds are not canonical");
  Ok(())
}

pub(super) fn artifact_path(field: &str, value: &str) -> Result<()> {
  ensure!(!value.is_empty(), "{field} must not be empty");
  ensure!(
    value.len() <= 1024,
    "{field} may contain at most 1024 UTF-8 bytes"
  );
  ensure!(!value.starts_with('/'), "{field} must be relative");
  ensure!(!value.contains('\\'), "{field} must use slash separators");
  ensure!(
    value
      .split('/')
      .all(|part| !part.is_empty() && part != "." && part != ".."),
    "{field} must be a normalized relative path"
  );
  Ok(())
}

fn number(bytes: &[u8], start: usize, length: usize) -> u32 {
  bytes[start..start + length]
    .iter()
    .fold(0, |value, byte| value * 10 + u32::from(byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
  match month {
    4 | 6 | 9 | 11 => 30,
    2 if leap_year(year) => 29,
    2 => 28,
    _ => 31,
  }
}

fn leap_year(year: u32) -> bool {
  year % 400 == 0 || year % 4 == 0 && year % 100 != 0
}
