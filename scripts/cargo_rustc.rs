use std::env;
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
#[cfg(not(unix))]
use std::process;
use std::process::Command;

fn main() {
  let mut arguments = env::args_os().skip(1);
  let compiler = arguments.next().expect("Cargo supplies the compiler");
  let arguments: Vec<OsString> = arguments.collect();
  let root = PathBuf::from(
    env::var_os("BATTLEMENT_CARGO_REPOSITORY").expect("Cargo supplies its repository"),
  );
  let local = arguments.iter().any(|value| {
    let path = Path::new(value);
    if path.extension().is_none_or(|extension| extension != "rs") {
      return false;
    }
    path
      .canonicalize()
      .is_ok_and(|source| source.starts_with(&root))
  });
  let outer = env::var_os("BATTLEMENT_CARGO_OUTER_WRAPPER");
  let mut command = Command::new(outer.as_ref().unwrap_or(&compiler));
  if outer.is_some() {
    command.arg(compiler);
  }
  if local {
    let mut arguments = arguments.into_iter();
    while let Some(value) = arguments.next() {
      if value == "--cap-lints" {
        arguments.next();
      } else if !value.to_string_lossy().starts_with("--cap-lints=") {
        command.arg(value);
      }
    }
    // Cwd remapping uses the pinned compiler's internal option. Only this
    // invocation opts in; dependency lint caps cannot allow unstable language.
    command.args(["-Zremap-cwd-prefix=.", "-Funstable_features"]);
    command.env("RUSTC_BOOTSTRAP", "1");
  } else {
    command.args(arguments);
  }
  execute(command);
}

#[cfg(unix)]
fn execute(mut command: Command) {
  panic!("Cannot execute compiler: {}", command.exec());
}

#[cfg(not(unix))]
fn execute(mut command: Command) {
  process::exit(
    command
      .status()
      .expect("Cannot execute compiler")
      .code()
      .unwrap_or(1),
  );
}
