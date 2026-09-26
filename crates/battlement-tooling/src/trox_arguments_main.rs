use std::{
  env, fs,
  path::{Path, PathBuf},
  process::ExitCode,
};

use anyhow::{Context, Result};
use battlement_tooling::trox_arguments;

fn main() -> ExitCode {
  match self::check() {
    Ok(false) => ExitCode::SUCCESS,
    Ok(true) => ExitCode::FAILURE,
    Err(error) => {
      eprintln!("{error:#}");
      ExitCode::FAILURE
    }
  }
}

fn check() -> Result<bool> {
  let mut failed = false;
  let mut paths = Vec::new();
  for root in env::args().skip(1) {
    self::sources(Path::new(&root), &mut paths)?;
  }
  paths.sort();
  for path in paths {
    let path = path.display().to_string();
    let source = fs::read_to_string(&path).with_context(|| format!("read {path}"))?;
    for diagnostic in
      trox_arguments::check_source(&source).with_context(|| format!("parse {path}"))?
    {
      let name = diagnostic.placeholder;
      eprintln!(
        "{path}:{}:{}: opaque argument `{name}` is hidden from Trox extraction; use tx_args![{name} => opaque(localized_value)] with an unqualified inline marker instead of a local wrapper or qualified call",
        diagnostic.line, diagnostic.column,
      );
      failed = true;
    }
  }
  Ok(failed)
}

fn sources(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
  if root.is_dir() {
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
      let entry = entry?;
      if entry.file_type()?.is_symlink() {
        continue;
      }
      self::sources(&entry.path(), paths)?;
    }
  } else if root.extension().is_some_and(|extension| extension == "rs") {
    paths.push(root.to_path_buf());
  }
  Ok(())
}
