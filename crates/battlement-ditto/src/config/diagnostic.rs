use std::{fmt, path::Path};

const KEYS: &[&str] = &[
  "name",
  "default_profile",
  "player",
  "timeouts",
  "defaults",
  "aliases",
  "baseline",
  "profiles",
  "scenarios",
  "unity_project",
  "scene",
  "rust_manifest",
  "run",
  "build",
  "launch",
  "baseline_download",
  "simulator_boot",
  "step_timeout",
  "scenario_timeout",
  "motion",
  "comparison",
  "threshold",
  "anti_alias",
  "max_changed_percent",
  "kind",
  "namespace",
  "root",
  "public_base_url",
  "account_id_env",
  "bucket_env",
  "access_key_id_env",
  "secret_access_key_env",
  "target",
  "display",
  "headless_command",
  "device",
  "orientation",
  "width",
  "height",
  "scale",
  "steps",
  "timeout",
  "click",
  "hover",
  "drag",
  "key",
  "wait",
  "assert",
  "screenshot",
  "video",
  "from",
  "to",
  "action",
  "frames",
  "object",
  "state",
  "max_duration",
];

#[derive(Debug)]
pub(super) struct ConfigError {
  path: String,
  line: usize,
  column: usize,
  key: String,
  message: String,
  suggestion: Option<String>,
}

pub(super) fn parse_error(path: &Path, source: &str, error: toml::de::Error) -> ConfigError {
  let offset = error.span().map_or(0, |span| span.start);
  let (line, column) = line_column(source, offset);
  let message = error.message().to_owned();
  let unknown = quoted_after(&message, "unknown field ");
  ConfigError {
    path: path.display().to_string(),
    line,
    column,
    key: source_key(source, line, unknown.as_deref()),
    suggestion: unknown.and_then(|unknown| nearest(&unknown)),
    message,
  }
}

pub(super) fn invalid(
  path: &Path,
  source: &str,
  key: impl Into<String>,
  message: impl Into<String>,
) -> ConfigError {
  let key = key.into();
  let leaf = key.rsplit('.').next().unwrap_or(&key);
  let offset = source
    .lines()
    .scan(0, |offset, line| {
      let start = *offset;
      *offset += line.len() + 1;
      Some((start, line))
    })
    .find(|(_, line)| line.trim_start().starts_with(&format!("{leaf} ")))
    .map_or(0, |(offset, _)| offset);
  let (line, column) = line_column(source, offset);
  ConfigError {
    path: path.display().to_string(),
    line,
    column,
    key,
    message: message.into(),
    suggestion: None,
  }
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
  let prefix = &source[..offset.min(source.len())];
  let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
  let column = prefix
    .rsplit_once('\n')
    .map_or(prefix.len(), |(_, tail)| tail.len())
    + 1;
  (line, column)
}

fn source_key(source: &str, line_number: usize, unknown: Option<&str>) -> String {
  let lines: Vec<&str> = source.lines().collect();
  let mut table = "suite";
  for line in lines.iter().take(line_number) {
    let trimmed = line.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
      table = trimmed.trim_matches('[').trim_matches(']');
    }
  }
  let leaf = unknown.or_else(|| {
    lines
      .get(line_number.saturating_sub(1))?
      .split_once('=')
      .map(|(key, _)| key.trim())
  });
  leaf.map_or_else(|| table.to_owned(), |leaf| format!("{table}.{leaf}"))
}

fn quoted_after(message: &str, marker: &str) -> Option<String> {
  let tail = message.split_once(marker)?.1;
  let quote = tail.chars().next()?;
  if quote != '`' && quote != '\'' && quote != '"' {
    return None;
  }
  Some(tail[quote.len_utf8()..].split_once(quote)?.0.to_owned())
}

fn nearest(value: &str) -> Option<String> {
  KEYS
    .iter()
    .map(|candidate| (*candidate, distance(value, candidate)))
    .min_by_key(|(_, distance)| *distance)
    .filter(|(_, distance)| *distance <= 3)
    .map(|(candidate, _)| candidate.to_owned())
}

fn distance(left: &str, right: &str) -> usize {
  let mut previous: Vec<usize> = (0..=right.len()).collect();
  for (left_index, left_byte) in left.bytes().enumerate() {
    let mut current = vec![left_index + 1];
    for (right_index, right_byte) in right.bytes().enumerate() {
      current.push(
        (current[right_index] + 1)
          .min(previous[right_index + 1] + 1)
          .min(previous[right_index] + usize::from(left_byte != right_byte)),
      );
    }
    previous = current;
  }
  previous[right.len()]
}

impl fmt::Display for ConfigError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      formatter,
      "{}:{}:{} [{}]: {}",
      self.path, self.line, self.column, self.key, self.message
    )?;
    if let Some(suggestion) = &self.suggestion {
      write!(formatter, "; did you mean `{suggestion}`?")?;
    }
    Ok(())
  }
}

impl std::error::Error for ConfigError {}
