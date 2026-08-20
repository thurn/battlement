use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

pub(crate) fn architectures(path: &Path) -> Result<Vec<String>> {
    let output = output("lipo", [OsStr::new("-archs"), path.as_os_str()])?;
    let architectures: Vec<String> = output.split_whitespace().map(str::to_owned).collect();
    if architectures.is_empty() {
        bail!("lipo reported no architectures for {}", path.display());
    }
    Ok(architectures)
}

pub(crate) fn exported_symbols(path: &Path) -> Result<Vec<String>> {
    output("nm", [OsStr::new("-gjU"), path.as_os_str()]).map(|output| {
        output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.strip_prefix('_').unwrap_or(line).to_owned())
            .collect()
    })
}

pub(crate) fn sign(path: &Path, identity: &str) -> Result<()> {
    status(
        "codesign",
        [
            OsStr::new("--force"),
            OsStr::new("--sign"),
            OsStr::new(identity),
            path.as_os_str(),
        ],
    )
}

pub(crate) fn signature_is_valid(path: &Path) -> bool {
    Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn output<I, S>(program: &str, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {program}"))?;
    if !output.status.success() {
        let diagnostic = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!("{program} failed: {diagnostic}");
    }
    String::from_utf8(output.stdout).with_context(|| format!("{program} returned non-UTF-8 output"))
}

fn status<I, S>(program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if !status.success() {
        bail!("{program} exited with status {status}");
    }
    Ok(())
}
