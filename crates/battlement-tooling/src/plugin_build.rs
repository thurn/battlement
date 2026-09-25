use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
};

use crate::{unity_lease::CompilerCapacityLease, web_archive};
use anyhow::{Context, Result, bail};

#[cfg(target_os = "macos")]
const PLUGIN_NAME: &str = "libbattlement_rules.dylib";
#[cfg(windows)]
const PLUGIN_NAME: &str = "battlement_rules.dll";
const WEB_TARGET: &str = "wasm32-unknown-emscripten";
const RELEASE_DEBUG_CONFIG: &str = "profile.release.debug=\"line-tables-only\"";
const RELEASE_SPLIT_DEBUG_CONFIG: &str = "profile.release.split-debuginfo=\"off\"";
const THREADED_RUSTFLAGS: &str = "-C panic=unwind -C target-feature=+atomics,+bulk-memory,+mutable-globals \
   -C link-arg=-fwasm-exceptions -C link-arg=-pthread";

pub fn rules_plugin(
  package: &str,
  architectures: &[String],
  release: bool,
  manifest_path: Option<&Path>,
) -> Result<PathBuf> {
  let target_directory = self::target_directory(package)?;
  let library_name = self::rules_library_name(manifest_path)?;
  let profile = if release { "release" } else { "debug" };
  let mut libraries = Vec::with_capacity(architectures.len());
  for architecture in architectures {
    let target = rust_target(architecture)?;
    build_slice(package, target, release, manifest_path, &target_directory)?;
    let library = target_directory
      .join(target)
      .join(profile)
      .join(self::native_artifact_name(&library_name));
    if !library.is_file() {
      bail!("package {package} did not produce {}", library.display());
    }
    libraries.push(library);
  }

  if let [library] = libraries.as_slice() {
    return Ok(library.clone());
  }
  universal_library(
    &libraries,
    &target_directory.join("universal").join(profile),
  )
}

pub fn web_rules_plugin(
  package: &str,
  release: bool,
  manifest_path: &Path,
  unity_editor: &Path,
) -> Result<PathBuf> {
  #[cfg(windows)]
  let emscripten = unity_editor
    .parent()
    .context("Unity Editor path has no parent directory")?
    .join("Data/PlaybackEngines/WebGLSupport/BuildTools/Emscripten");
  #[cfg(not(windows))]
  let emscripten = unity_editor
    .ancestors()
    .nth(4)
    .context("Unity Editor path does not have the expected application layout")?
    .join("PlaybackEngines/WebGLSupport/BuildTools/Emscripten");
  #[cfg(windows)]
  let emcc = emscripten.join("emscripten/emcc.bat");
  #[cfg(not(windows))]
  let emcc = emscripten.join("emscripten/emcc");
  let llvm = emscripten.join("llvm");
  let binaryen = emscripten.join("binaryen");
  #[cfg(windows)]
  let node = emscripten.join("node/node.exe");
  #[cfg(not(windows))]
  let node = emscripten.join("node/node");
  #[cfg(windows)]
  let required = [
    &emcc,
    &node,
    &llvm.join("clang.exe"),
    &binaryen.join("bin/wasm-opt.exe"),
  ];
  #[cfg(not(windows))]
  let required = [
    &emcc,
    &node,
    &llvm.join("clang"),
    &binaryen.join("bin/wasm-opt"),
  ];
  for required in required {
    if !required.is_file() {
      bail!(
        "Unity Web Build Support is incomplete; {} was not found",
        required.display()
      );
    }
  }

  let target_directory = self::target_directory(package)?.join("web-threaded");
  let toolchain_directory = target_directory.join("emscripten");
  fs::create_dir_all(&toolchain_directory)
    .with_context(|| format!("failed to create {}", toolchain_directory.display()))?;
  let config = toolchain_directory.join(".emscripten");
  fs::write(
    &config,
    format!(
      "LLVM_ROOT = {}\nBINARYEN_ROOT = {}\nNODE_JS = {}\n",
      self::python_string(&llvm),
      self::python_string(&binaryen),
      self::python_string(&node),
    ),
  )
  .with_context(|| format!("failed to write {}", config.display()))?;

  let mut paths = vec![
    emscripten.join("emscripten"),
    llvm,
    binaryen.join("bin"),
    emscripten.join("node"),
  ];
  if let Some(existing) = env::var_os("PATH") {
    paths.extend(env::split_paths(&existing));
  }
  let mut command = self::web_cargo_command(package, release, manifest_path, &target_directory);
  command
    .env("EM_CONFIG", &config)
    .env("EM_CACHE", toolchain_directory.join("cache"))
    .env("CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER", &emcc)
    .env("PATH", env::join_paths(paths)?);
  command.env("RUSTC_BOOTSTRAP", "1").env(
    "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS",
    THREADED_RUSTFLAGS,
  );
  let capacity = CompilerCapacityLease::acquire(&crate::developer_tools::resource_slots())?;
  let status = command
    .status()
    .context("failed to run the Rust WebAssembly build")?;
  drop(capacity);
  if !status.success() {
    bail!(
      "Rust WebAssembly build exited with status {status}; install its standard library with `rustup target add {WEB_TARGET}`"
    );
  }

  let library_name = self::rules_library_name(Some(manifest_path))?;
  let plugin = self::web_plugin_path(&target_directory, release, &library_name);
  if !plugin.is_file() {
    bail!("Rust WebAssembly build omitted {}", plugin.display());
  }
  web_archive::retain_constructors(&plugin, &emscripten.join("llvm"))
}

fn target_directory(package: &str) -> Result<PathBuf> {
  Ok(
    env::current_dir()
      .context("failed to locate the current Cargo workspace")?
      .join("target/battlement-plugin")
      .join(package),
  )
}

pub(crate) fn rules_library_name(manifest_path: Option<&Path>) -> Result<String> {
  let Some(manifest_path) = manifest_path else {
    return Ok("battlement_rules".to_owned());
  };
  let source = fs::read_to_string(manifest_path)
    .with_context(|| format!("failed to read {}", manifest_path.display()))?;
  let manifest: toml::Value = toml::from_str(&source)
    .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
  if let Some(name) = manifest
    .get("lib")
    .and_then(|lib| lib.get("name"))
    .and_then(toml::Value::as_str)
  {
    return Ok(name.to_owned());
  }
  let package = manifest
    .get("package")
    .and_then(|package| package.get("name"))
    .and_then(toml::Value::as_str)
    .context("rules manifest has no package name")?;
  Ok(package.replace('-', "_"))
}

pub(crate) fn native_artifact_name(library_name: &str) -> String {
  if cfg!(windows) {
    format!("{library_name}.dll")
  } else {
    format!("lib{library_name}.dylib")
  }
}

fn web_cargo_command(
  package: &str,
  release: bool,
  manifest_path: &Path,
  target_directory: &Path,
) -> Command {
  let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
  let mut command = crate::process_priority::command(cargo);
  command
    .arg("rustc")
    .arg("--package")
    .arg(package)
    .args(["--target", WEB_TARGET, "--target-dir"])
    .arg(target_directory)
    .arg("--manifest-path")
    .arg(manifest_path)
    .arg("--lib")
    .args(["--crate-type", "staticlib"]);
  command.args(["-Z", "build-std=std,panic_unwind"]);
  self::configure_release_profile(&mut command, release);
  command
}

fn web_plugin_path(target_directory: &Path, release: bool, library_name: &str) -> PathBuf {
  target_directory
    .join(WEB_TARGET)
    .join(if release { "release" } else { "debug" })
    .join(format!("lib{library_name}.a"))
}

fn build_slice(
  package: &str,
  target: &str,
  release: bool,
  manifest_path: Option<&Path>,
  target_directory: &Path,
) -> Result<()> {
  let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
  let mut command = crate::process_priority::command(cargo);
  command
    .arg("build")
    .arg("--package")
    .arg(package)
    .arg("--target")
    .arg(target)
    .arg("--target-dir")
    .arg(target_directory);
  self::configure_release_profile(&mut command, release);
  if let Some(manifest_path) = manifest_path {
    command.arg("--manifest-path").arg(manifest_path);
  }
  let capacity = CompilerCapacityLease::acquire(&crate::developer_tools::resource_slots())?;
  let status = command.status().context("failed to run cargo build")?;
  drop(capacity);
  if !status.success() {
    bail!("cargo build for {target} exited with status {status}");
  }
  Ok(())
}

fn configure_release_profile(command: &mut Command, release: bool) {
  if !release {
    return;
  }

  command
    .arg("--release")
    .args(["--config", RELEASE_DEBUG_CONFIG])
    .args(["--config", RELEASE_SPLIT_DEBUG_CONFIG]);
}

fn universal_library(libraries: &[PathBuf], directory: &Path) -> Result<PathBuf> {
  fs::create_dir_all(directory)
    .with_context(|| format!("failed to create {}", directory.display()))?;
  let output = directory.join(PLUGIN_NAME);
  let status = Command::new("lipo")
    .arg("-create")
    .args(libraries)
    .arg("-output")
    .arg(&output)
    .status()
    .context("failed to run lipo")?;
  if !status.success() {
    bail!("lipo exited with status {status}");
  }
  Ok(output)
}

fn rust_target(architecture: &str) -> Result<&'static str> {
  #[cfg(windows)]
  return match architecture {
    "x86_64" => Ok("x86_64-pc-windows-msvc"),
    _ => bail!("unsupported Windows architecture: {architecture}"),
  };
  #[cfg(target_os = "macos")]
  match architecture {
    "arm64" => Ok("aarch64-apple-darwin"),
    "x86_64" => Ok("x86_64-apple-darwin"),
    _ => bail!("unsupported macOS architecture reported by Unity: {architecture}"),
  }
}

fn python_string(path: &Path) -> String {
  format!(
    "'{}'",
    path
      .to_string_lossy()
      .replace('\\', "\\\\")
      .replace('\'', "\\'")
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unity_architectures_map_to_rust_targets() {
    #[cfg(target_os = "macos")]
    {
      assert_eq!(rust_target("arm64").unwrap(), "aarch64-apple-darwin");
      assert_eq!(rust_target("x86_64").unwrap(), "x86_64-apple-darwin");
    }
    #[cfg(windows)]
    assert_eq!(rust_target("x86_64").unwrap(), "x86_64-pc-windows-msvc");
    assert!(rust_target("ppc64").is_err());
  }

  #[test]
  fn rules_packages_have_isolated_target_directories() {
    assert_ne!(
      target_directory("battlement-basic-rules").unwrap(),
      target_directory("battlement-tictactoe-rules").unwrap()
    );
  }

  #[test]
  fn web_build_replaces_manifest_crate_types_with_static_library() {
    let command = web_cargo_command(
      "battlement-example-rules",
      true,
      Path::new("rules/Cargo.toml"),
      Path::new("target/web"),
    );
    let arguments: Vec<_> = command
      .get_args()
      .map(|argument| argument.to_string_lossy().into_owned())
      .collect();

    assert!(
      arguments
        .windows(2)
        .any(|pair| pair == ["--crate-type", "staticlib"])
    );
    assert!(!arguments.iter().any(|argument| argument == "--"));
    assert!(arguments.iter().any(|argument| argument == "--release"));
    assert!(
      arguments
        .iter()
        .any(|argument| argument == RELEASE_DEBUG_CONFIG)
    );
    assert!(
      arguments
        .iter()
        .any(|argument| argument == RELEASE_SPLIT_DEBUG_CONFIG)
    );
  }

  #[test]
  fn threaded_web_build_rebuilds_std() {
    let command = web_cargo_command(
      "battlement-example-rules",
      false,
      Path::new("rules/Cargo.toml"),
      Path::new("target/web-threaded"),
    );
    let arguments: Vec<_> = command
      .get_args()
      .map(|argument| argument.to_string_lossy().into_owned())
      .collect();

    assert!(
      arguments
        .windows(2)
        .any(|pair| pair == ["-Z", "build-std=std,panic_unwind"])
    );
  }

  #[test]
  fn threaded_web_build_uses_unwind_and_thread_link_flags() {
    assert!(THREADED_RUSTFLAGS.contains("-C panic=unwind"));
    assert!(THREADED_RUSTFLAGS.contains("-C link-arg=-fwasm-exceptions"));
    assert!(THREADED_RUSTFLAGS.contains("-C link-arg=-pthread"));
  }

  #[test]
  fn web_build_uses_cargo_static_library_artifact() {
    assert_eq!(
      web_plugin_path(Path::new("target/web"), false, "chess_rules"),
      Path::new("target/web/wasm32-unknown-emscripten/debug/libchess_rules.a")
    );
    assert_eq!(
      web_plugin_path(Path::new("target/web"), false, "battlement_rules"),
      Path::new("target/web/wasm32-unknown-emscripten/debug/libbattlement_rules.a")
    );
    assert_eq!(
      web_plugin_path(Path::new("target/web"), true, "battlement_rules"),
      Path::new("target/web/wasm32-unknown-emscripten/release/libbattlement_rules.a")
    );
  }

  #[test]
  fn chess_library_keeps_its_cargo_name() {
    let manifest =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/chess/rules/Cargo.toml");
    let name = rules_library_name(Some(&manifest)).unwrap();
    assert_eq!(name, "chess_rules");
    #[cfg(target_os = "macos")]
    assert_eq!(native_artifact_name(&name), "libchess_rules.dylib");
    #[cfg(windows)]
    assert_eq!(native_artifact_name(&name), "chess_rules.dll");
  }
}
