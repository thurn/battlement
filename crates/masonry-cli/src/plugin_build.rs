use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

const PLUGIN_NAME: &str = "libmasonry_rules.dylib";

pub(crate) fn rules_plugin(
    package: &str,
    architectures: &[String],
    release: bool,
    manifest_path: Option<&Path>,
) -> Result<PathBuf> {
    let target_directory = self::target_directory(package)?;
    let profile = if release { "release" } else { "debug" };
    let mut libraries = Vec::with_capacity(architectures.len());
    for architecture in architectures {
        let target = rust_target(architecture)?;
        build_slice(package, target, release, manifest_path, &target_directory)?;
        let library = target_directory
            .join(target)
            .join(profile)
            .join(PLUGIN_NAME);
        if !library.is_file() {
            bail!(
                "package {package} did not produce {}; its cdylib target must be named masonry_rules",
                library.display()
            );
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

fn target_directory(package: &str) -> Result<PathBuf> {
    Ok(env::current_dir()
        .context("failed to locate the current Cargo workspace")?
        .join("target/masonry-plugin")
        .join(package))
}

fn build_slice(
    package: &str,
    target: &str,
    release: bool,
    manifest_path: Option<&Path>,
    target_directory: &Path,
) -> Result<()> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .arg("build")
        .arg("--package")
        .arg(package)
        .arg("--target")
        .arg(target)
        .arg("--target-dir")
        .arg(target_directory);
    if release {
        command.arg("--release");
    }
    if let Some(manifest_path) = manifest_path {
        command.arg("--manifest-path").arg(manifest_path);
    }
    let status = command.status().context("failed to run cargo build")?;
    if !status.success() {
        bail!("cargo build for {target} exited with status {status}");
    }
    Ok(())
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
    match architecture {
        "arm64" => Ok("aarch64-apple-darwin"),
        "x86_64" => Ok("x86_64-apple-darwin"),
        _ => bail!("unsupported macOS architecture reported by Unity: {architecture}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_architectures_map_to_rust_targets() {
        assert_eq!(rust_target("arm64").unwrap(), "aarch64-apple-darwin");
        assert_eq!(rust_target("x86_64").unwrap(), "x86_64-apple-darwin");
        assert!(rust_target("ppc64").is_err());
    }

    #[test]
    fn rules_packages_have_isolated_target_directories() {
        assert_ne!(
            target_directory("masonry-basic-rules").unwrap(),
            target_directory("masonry-tictactoe-rules").unwrap()
        );
    }
}
