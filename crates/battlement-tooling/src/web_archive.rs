//! Preserve static registrations when Unity links a rules archive.

use std::{
  fs,
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{Context, Result, ensure};

/// Combines archive members into one relocatable object so constructors are retained.
/// Unity can then extract the rules entry points without omitting registration-only members.
pub fn retain_constructors(archive: &Path, llvm: &Path) -> Result<PathBuf> {
  let directory = archive
    .parent()
    .context("rules archive needs a directory")?
    .join("unity");
  fs::create_dir_all(&directory)?;
  let object = directory.join("battlement_rules.o");
  let output = directory.join("libbattlement_rules.a");
  let linked = Command::new(self::executable(llvm, "wasm-ld"))
    .args(["-r", "--whole-archive"])
    .arg(archive)
    .args(["--no-whole-archive", "-o"])
    .arg(&object)
    .output()
    .context("failed to link the WebAssembly rules archive")?;
  ensure!(
    linked.status.success(),
    "WebAssembly rules archive link failed: {}",
    String::from_utf8_lossy(&linked.stderr)
  );
  if output.exists() {
    fs::remove_file(&output)?;
  }
  let packed = Command::new(self::executable(llvm, "llvm-ar"))
    .arg("crs")
    .arg(&output)
    .arg(&object)
    .output()
    .context("failed to pack the WebAssembly rules archive")?;
  ensure!(
    packed.status.success(),
    "WebAssembly rules archive packing failed: {}",
    String::from_utf8_lossy(&packed.stderr)
  );
  Ok(output)
}

fn executable(directory: &Path, name: &str) -> PathBuf {
  directory.join(if cfg!(windows) {
    format!("{name}.exe")
  } else {
    name.to_owned()
  })
}
