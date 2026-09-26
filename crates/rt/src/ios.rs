use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

use anyhow::{Context, Result, bail, ensure};
use battlement_tooling::{
  application::Project,
  build_cache::{BuildCache, DEFAULT_BUILD_CACHE_BYTES},
  build_control::BuildControl,
  build_identity::CaptureAdapter,
  developer_tools,
  host::{Host, SystemHost},
  ios_build::{self, IosBuildRequest, IosBuildResult, IosBuildTools},
  ios_target::{IosSigning, IosTarget},
};
use sha2::{Digest, Sha256};

pub(crate) struct BuildOptions {
  pub unsigned: bool,
  pub team: Option<String>,
  pub identity: Option<String>,
  pub profile: Option<PathBuf>,
  pub cache: Option<PathBuf>,
}

/// Builds a device artifact with explicit unsigned or caller-supplied signing inputs.
pub(crate) fn build(
  project: &Project,
  options: BuildOptions,
  interrupted: &AtomicBool,
) -> Result<()> {
  ensure!(
    cfg!(target_os = "macos"),
    "iOS device builds require macOS and Xcode"
  );
  let signing = if options.unsigned {
    None
  } else {
    Some(self::signing(
      options
        .team
        .context("supply --signing-team or --unsigned")?,
      options.identity.context("supply --signing-identity")?,
      options
        .profile
        .context("supply an installed --provisioning-profile")?,
    )?)
  };
  let cargo = SystemHost
    .find_executable("cargo")
    .context("Cargo is unavailable")?;
  let rustc = SystemHost
    .find_executable("rustc")
    .context("rustc is unavailable")?;
  let xcrun = SystemHost
    .find_executable("xcrun")
    .context("Xcode is unavailable")?;
  let xcodebuild = PathBuf::from(SystemHost.command_output(&xcrun, &["--find", "xcodebuild"])?);
  let unity_version = fs::read_to_string(project.root.join("ProjectSettings/ProjectVersion.txt"))?
    .lines()
    .find_map(|line| line.strip_prefix("m_EditorVersion: "))
    .context("Unity editor version is missing")?
    .to_owned();
  let git = SystemHost
    .find_executable("git")
    .context("Git is unavailable")?;
  let repository = PathBuf::from(SystemHost.command_output(
    &git,
    &[
      "-C",
      project.root.to_str().context("project path is not UTF-8")?,
      "rev-parse",
      "--show-toplevel",
    ],
  )?);
  let request = IosBuildRequest {
    target: IosTarget::Device { signing },
    repository,
    unity_project: project.root.clone(),
    rust_manifest: project.manifest.clone(),
    scene: project.scene.clone(),
    suite: format!("{}-device", project.application),
    diagnostics: false,
    generated_inputs: Vec::new(),
    native_inputs: Vec::new(),
    capture_adapter: CaptureAdapter {
      name: "native-screen-capture".to_owned(),
      version: "1".to_owned(),
    },
    tools: IosBuildTools {
      unity_editor: developer_tools::unity_editor(&project.root)?,
      unity_version,
      cargo_version: SystemHost.command_output(&cargo, &["--version"])?,
      rustc_version: SystemHost.command_output(&rustc, &["--version"])?,
      cargo,
      architecture: "arm64".to_owned(),
      xcode_version: SystemHost.command_output(&xcodebuild, &["-version"])?,
      xcodebuild,
      sdk_version: SystemHost
        .command_output(&xcrun, &["--sdk", "iphoneos", "--show-sdk-version"])?,
    },
    resource_slots: developer_tools::resource_slots(),
    cache: BuildCache::open(
      options
        .cache
        .unwrap_or_else(|| project.root.join("Build/ios-cache")),
      DEFAULT_BUILD_CACHE_BYTES,
    )?,
  };
  match ios_build::select_ios_player(&request, true, BuildControl::new(interrupted))? {
    IosBuildResult::Ready { build, outcome } => {
      println!(
        "Device artifact: {}",
        ios_build::player_app(&build)?.display()
      );
      println!(
        "Build: {} ({outcome:?}); signing: {}",
        build.metadata().identity.fingerprint,
        if request.target.is_signed() {
          "manual"
        } else {
          "unsigned; cannot install on a physical device"
        }
      );
      println!(
        "Identity: {}",
        build
          .path()
          .join(ios_build::STARTUP_IDENTITY_FILE)
          .display()
      );
      Ok(())
    }
    IosBuildResult::Failed(failure) => bail!(
      "iOS {} build failed: {}; log: {}",
      failure.phase,
      failure.message,
      failure.log_path.display()
    ),
    IosBuildResult::Required { .. } => unreachable!("builds are allowed"),
  }
}

fn signing(team: String, identity: String, profile: PathBuf) -> Result<IosSigning> {
  let contents = fs::read(&profile).context("read supplied installed provisioning profile")?;
  let security = SystemHost
    .find_executable("security")
    .context("security is unavailable")?;
  let decoded = SystemHost.command_output(
    &security,
    &[
      "cms",
      "-D",
      "-i",
      profile.to_str().context("profile path is not UTF-8")?,
    ],
  )?;
  let plist = tempfile::NamedTempFile::new()?;
  fs::write(plist.path(), decoded)?;
  let plutil = SystemHost
    .find_executable("plutil")
    .context("plutil is unavailable")?;
  let profile_uuid = SystemHost.command_output(
    &plutil,
    &[
      "-extract",
      "UUID",
      "raw",
      "-o",
      "-",
      plist
        .path()
        .to_str()
        .context("temporary path is not UTF-8")?,
    ],
  )?;
  Ok(IosSigning {
    team,
    identity,
    profile_uuid,
    profile_fingerprint: Sha256::digest(contents)
      .iter()
      .map(|byte| format!("{byte:02x}"))
      .collect(),
  })
}
