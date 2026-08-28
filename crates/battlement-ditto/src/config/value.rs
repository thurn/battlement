use std::{fmt, ops::RangeInclusive, str::FromStr, time::Duration};

use anyhow::{Result, bail};

/// A positive whole-millisecond duration used by authoring configuration.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DurationValue(Duration);

/// A normalized unsigned base-10 decimal without binary floating-point loss.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExactDecimal(String);

impl DurationValue {
  /// Parses an integer duration ending in `ms`, `s`, or `m`.
  pub fn parse(value: &str) -> Result<Self> {
    let (digits, multiplier) = if let Some(digits) = value.strip_suffix("ms") {
      (digits, 1)
    } else if let Some(digits) = value.strip_suffix('s') {
      (digits, 1_000)
    } else if let Some(digits) = value.strip_suffix('m') {
      (digits, 60_000)
    } else {
      bail!("duration must be an integer followed by ms, s, or m");
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
      bail!("duration must be an integer followed by ms, s, or m");
    }
    let milliseconds = digits
      .parse::<u64>()?
      .checked_mul(multiplier)
      .ok_or_else(|| anyhow::anyhow!("duration is too large"))?;
    if milliseconds == 0 {
      bail!("duration must be positive");
    }
    Ok(Self(Duration::from_millis(milliseconds)))
  }

  /// Creates a duration from milliseconds known to be positive.
  pub const fn from_millis(milliseconds: u64) -> Self {
    Self(Duration::from_millis(milliseconds))
  }

  /// Returns the whole-millisecond value.
  pub fn as_millis(self) -> u64 {
    self.0.as_millis() as u64
  }
}

impl ExactDecimal {
  pub(crate) fn parse(value: &str, range: RangeInclusive<&str>) -> Result<Self> {
    if value.is_empty() || value.starts_with(['+', '-']) || value.contains(['e', 'E']) {
      bail!("decimal must be unsigned base-10 without an exponent");
    }
    let mut parts = value.split('.');
    let integer = parts.next().expect("split always has one part");
    let fraction = parts.next();
    let invalid_integer = integer.is_empty() || !integer.bytes().all(|byte| byte.is_ascii_digit());
    let invalid_fraction =
      fraction.is_some_and(|digits| !digits.bytes().all(|byte| byte.is_ascii_digit()));
    if parts.next().is_some() || invalid_integer || invalid_fraction {
      bail!("decimal must contain digits with at most one decimal point");
    }
    if integer.len() > 1 && integer.starts_with('0') {
      bail!("decimal must not contain a redundant leading zero");
    }
    let fraction = fraction.unwrap_or_default().trim_end_matches('0');
    let normalized = if fraction.is_empty() {
      integer.to_owned()
    } else {
      format!("{integer}.{fraction}")
    };
    if decimal_cmp(&normalized, range.start()).is_lt()
      || decimal_cmp(&normalized, range.end()).is_gt()
    {
      bail!(
        "decimal must be from {} through {}",
        range.start(),
        range.end()
      );
    }
    Ok(Self(normalized))
  }

  /// Returns the normalized decimal string.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Display for DurationValue {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}ms", self.as_millis())
  }
}

impl fmt::Display for ExactDecimal {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl FromStr for DurationValue {
  type Err = anyhow::Error;

  fn from_str(value: &str) -> Result<Self> {
    Self::parse(value)
  }
}

fn decimal_cmp(left: &str, right: &str) -> std::cmp::Ordering {
  let (left_integer, left_fraction) = left.split_once('.').unwrap_or((left, ""));
  let (right_integer, right_fraction) = right.split_once('.').unwrap_or((right, ""));
  left_integer
    .len()
    .cmp(&right_integer.len())
    .then_with(|| left_integer.cmp(right_integer))
    .then_with(|| {
      let length = left_fraction.len().max(right_fraction.len());
      left_fraction
        .bytes()
        .chain(std::iter::repeat(b'0'))
        .take(length)
        .cmp(
          right_fraction
            .bytes()
            .chain(std::iter::repeat(b'0'))
            .take(length),
        )
    })
}
