use std::{
  fs,
  path::PathBuf,
  sync::atomic::AtomicBool,
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail, ensure};
use battlement_tooling::{
  application::Project,
  build_cache::{BuildCache, DEFAULT_BUILD_CACHE_BYTES},
  build_control::BuildControl,
  build_identity::CaptureAdapter,
  developer_tools,
  host::{Host, SystemHost},
  ios_build::{self, IosBuildRequest, IosBuildResult, IosBuildTools},
  ios_device::{self, IosDeviceRequest, IosDeviceTools},
  ios_target::{self, IosSigning, IosTarget},
};
use sha2::{Digest, Sha256};

pub(crate) struct BuildOptions {
  pub bundle_identifier: String,
  pub unsigned: bool,
  pub team: Option<String>,
  pub identity: Option<String>,
  pub profile: Option<PathBuf>,
  pub cache: Option<PathBuf>,
}

pub(crate) fn device(
  cache: PathBuf,
  fingerprint: &str,
  request: IosDeviceRequest,
  interrupted: &AtomicBool,
) -> Result<()> {
  ensure!(
    cfg!(target_os = "macos"),
    "iOS device installation requires macOS and Xcode"
  );
  let cache = BuildCache::open(cache, DEFAULT_BUILD_CACHE_BYTES)?;
  let build = cache.retain_for_replay(
    fingerprint,
    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
  )?;
  let executable = |name| {
    SystemHost
      .find_executable(name)
      .with_context(|| format!("{name} is unavailable"))
  };
  let receipt = ios_device::install(
    &build,
    &IosDeviceTools {
      xcrun: executable("xcrun")?,
      codesign: executable("codesign")?,
      plutil: executable("plutil")?,
    },
    &request,
    BuildControl::new(interrupted),
  )?;
  println!("{}", serde_json::to_string_pretty(&receipt)?);
  Ok(())
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
  ios_target::validate_bundle_identifier(&options.bundle_identifier)?;
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
      &options.bundle_identifier,
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
    target: IosTarget::Device {
      bundle_identifier: options.bundle_identifier,
      signing,
    },
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

fn signing(
  team: String,
  identity: String,
  profile: PathBuf,
  bundle_identifier: &str,
) -> Result<IosSigning> {
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
  let field = |name| {
    SystemHost.command_output(
      &plutil,
      &[
        "-extract",
        name,
        "raw",
        "-o",
        "-",
        plist
          .path()
          .to_str()
          .context("temporary path is not UTF-8")?,
      ],
    )
  };
  let profile_uuid = field("UUID")?;
  ensure!(
    field("TeamIdentifier.0")? == team,
    "provisioning profile belongs to another development team"
  );
  let application = field("Entitlements.application-identifier")?;
  let prefix = field("ApplicationIdentifierPrefix.0")?;
  let expected = format!("{prefix}.{bundle_identifier}");
  let matches = application
    .strip_suffix('*')
    .map_or(application == expected, |prefix| {
      expected.starts_with(prefix)
    });
  ensure!(
    matches,
    "provisioning profile does not authorize bundle identifier {bundle_identifier}; supply a matching installed profile"
  );
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
