use std::collections::BTreeMap;

use battlement_tooling::build_identity::{
  AppleToolchain, BaselineIdentity, BuildIdentity, BuildIdentityRequest, BuildTarget,
  CaptureAdapter, NativeInput, NoBuildDecision, RustToolchain,
};

const HASH_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HASH_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

#[test]
fn build_identity_has_stable_bytes_and_sorted_explanation() {
  let identity = BuildIdentity::derive(&request()).unwrap();
  assert_eq!(
    identity.fingerprint,
    "94e964d0fa8f21955ebb45baf9c67a79bb72bb10e3b86e7545b9ff8292f49106"
  );
  assert_eq!(identity.source_fingerprint, HASH_A);
  assert_eq!(
    identity
      .inputs
      .iter()
      .map(|input| input.name.as_str())
      .collect::<Vec<_>>(),
    [
      "apple.sdk",
      "apple.xcode",
      "capture-adapter.name",
      "capture-adapter.version",
      "diagnostics",
      "native.battlement-rules",
      "native.csharp-bridge",
      "option.codegen-units",
      "option.development-build",
      "rust.cargo",
      "rust.rustc",
      "rust.target",
      "source",
      "target",
      "unity",
    ]
  );
  let mut reordered = request();
  reordered.native_inputs.reverse();
  reordered.options = reordered.options.into_iter().rev().collect();
  assert_eq!(BuildIdentity::derive(&reordered).unwrap(), identity);
}

#[test]
fn every_included_input_changes_the_fingerprint() {
  let original = BuildIdentity::derive(&request()).unwrap();
  let mut mutations = Vec::new();
  mutations.push(changed("source", |request| {
    request.source_fingerprint = HASH_B.to_owned()
  }));
  mutations.push(changed("target", |request| {
    request.target = BuildTarget::IosSimulator
  }));
  mutations.push(changed("unity", |request| {
    request.unity_version.push_str("-changed")
  }));
  mutations.push(changed("rustc", |request| {
    request.rust.rustc_version.push_str("-changed")
  }));
  mutations.push(changed("cargo", |request| {
    request.rust.cargo_version.push_str("-changed")
  }));
  mutations.push(changed("rust target", |request| {
    request.rust.target.push_str("-changed")
  }));
  mutations.push(changed("Xcode", |request| {
    request
      .apple
      .as_mut()
      .unwrap()
      .xcode_version
      .push_str("-changed")
  }));
  mutations.push(changed("SDK", |request| {
    request
      .apple
      .as_mut()
      .unwrap()
      .sdk_version
      .push_str("-changed")
  }));
  mutations.push(changed("diagnostics", |request| {
    request.diagnostics = false
  }));
  mutations.push(changed("adapter", |request| {
    request.capture_adapter.name.push_str("-changed")
  }));
  mutations.push(changed("adapter version", |request| {
    request.capture_adapter.version.push_str("-changed")
  }));
  mutations.push(changed("native input", |request| {
    request.native_inputs[0].sha256 = HASH_A.to_owned()
  }));
  mutations.push(changed("build option", |request| {
    request
      .options
      .insert("codegen-units".to_owned(), "8".to_owned());
  }));

  for (name, request) in mutations {
    assert_ne!(
      BuildIdentity::derive(&request).unwrap().fingerprint,
      original.fingerprint,
      "{name} did not affect the build"
    );
  }
}

#[test]
fn runtime_inputs_are_outside_build_identity_and_baselines_ignore_hashes() {
  let build = request();
  let first_runtime = RuntimeInputs::canonical();
  let second_runtime = RuntimeInputs {
    profile_name: "phone".to_owned(),
    display: "390x844@3".to_owned(),
    device: Some("iPhone 18".to_owned()),
    orientation: Some("portrait".to_owned()),
    headless_command: Some("browser --headless {url}".to_owned()),
    scenarios: "changed scenario bytes".to_owned(),
    aliases: "changed aliases".to_owned(),
    baselines: "changed baseline lock".to_owned(),
    seed: 99,
    saves: "changed save".to_owned(),
    motion: "real-time".to_owned(),
  };
  assert_ne!(first_runtime, second_runtime);
  assert_eq!(
    execution_build(&build, &first_runtime),
    execution_build(&build, &second_runtime)
  );

  let baseline = BaselineIdentity::new(
    "battlement/tictactoe",
    "macos-ci",
    "human wins",
    "opening-move",
  )
  .unwrap();
  let mut changed_build = build;
  changed_build.source_fingerprint = HASH_B.to_owned();
  assert_ne!(
    BuildIdentity::derive(&changed_build).unwrap().fingerprint,
    execution_build(&request(), &first_runtime).fingerprint
  );
  assert_eq!(
    baseline,
    BaselineIdentity::new(
      "battlement/tictactoe",
      "macos-ci",
      "human wins",
      "opening-move",
    )
    .unwrap()
  );
}

#[test]
fn no_build_decision_explains_reuse_and_rejection() {
  let cached = BuildIdentity::derive(&request()).unwrap();
  assert_eq!(
    cached.no_build_decision(Some(&cached)),
    NoBuildDecision::Reuse {
      fingerprint: cached.fingerprint.clone()
    }
  );

  let mut changed = request();
  changed.source_fingerprint = HASH_B.to_owned();
  let expected = BuildIdentity::derive(&changed).unwrap();
  assert_eq!(
    expected.no_build_decision(Some(&cached)),
    NoBuildDecision::Required {
      expected: expected.fingerprint.clone(),
      available: Some(cached.fingerprint),
      changed_inputs: vec!["source".to_owned()],
    }
  );
}

#[test]
fn invalid_or_inapplicable_inputs_are_rejected() {
  let mut webgl = request();
  webgl.target = BuildTarget::Webgl;
  assert!(
    BuildIdentity::derive(&webgl)
      .unwrap_err()
      .to_string()
      .contains("Apple")
  );
  webgl.apple = None;
  BuildIdentity::derive(&webgl).unwrap();

  let mut duplicate = request();
  duplicate
    .native_inputs
    .push(duplicate.native_inputs[0].clone());
  assert!(
    BuildIdentity::derive(&duplicate)
      .unwrap_err()
      .to_string()
      .contains("duplicate native input")
  );
  let mut invalid = request();
  invalid.native_inputs[0].sha256 = "A".repeat(64);
  assert!(
    BuildIdentity::derive(&invalid)
      .unwrap_err()
      .to_string()
      .contains("invalid native input digest")
  );
}

fn request() -> BuildIdentityRequest {
  BuildIdentityRequest {
    source_fingerprint: HASH_A.to_owned(),
    target: BuildTarget::Macos,
    unity_version: "6000.5.8f1".to_owned(),
    rust: RustToolchain {
      rustc_version: "rustc 1.91.0 (stable)".to_owned(),
      cargo_version: "cargo 1.91.0".to_owned(),
      target: "aarch64-apple-darwin".to_owned(),
    },
    apple: Some(AppleToolchain {
      xcode_version: "Xcode 26.0 17A324".to_owned(),
      sdk_version: "macosx26.0-25A320".to_owned(),
    }),
    diagnostics: true,
    capture_adapter: CaptureAdapter {
      name: "unity-async-readback-png".to_owned(),
      version: "1".to_owned(),
    },
    native_inputs: vec![
      NativeInput {
        name: "battlement-rules".to_owned(),
        sha256: HASH_B.to_owned(),
      },
      NativeInput {
        name: "csharp-bridge".to_owned(),
        sha256: HASH_A.to_owned(),
      },
    ],
    options: BTreeMap::from([
      ("development-build".to_owned(), "false".to_owned()),
      ("codegen-units".to_owned(), "1".to_owned()),
    ]),
  }
}

fn changed(
  name: &'static str,
  change: impl FnOnce(&mut BuildIdentityRequest),
) -> (&'static str, BuildIdentityRequest) {
  let mut request = request();
  change(&mut request);
  (name, request)
}

fn execution_build(build: &BuildIdentityRequest, runtime: &RuntimeInputs) -> BuildIdentity {
  assert!(!runtime.profile_name.is_empty());
  assert!(!runtime.display.is_empty());
  assert!(!runtime.scenarios.is_empty());
  assert!(!runtime.aliases.is_empty());
  assert!(!runtime.baselines.is_empty());
  assert!(!runtime.saves.is_empty());
  assert!(!runtime.motion.is_empty());
  let _ = (
    &runtime.device,
    &runtime.orientation,
    &runtime.headless_command,
    runtime.seed,
  );
  BuildIdentity::derive(build).unwrap()
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeInputs {
  profile_name: String,
  display: String,
  device: Option<String>,
  orientation: Option<String>,
  headless_command: Option<String>,
  scenarios: String,
  aliases: String,
  baselines: String,
  seed: u64,
  saves: String,
  motion: String,
}

impl RuntimeInputs {
  fn canonical() -> Self {
    Self {
      profile_name: "macos-ci".to_owned(),
      display: "1280x720@1".to_owned(),
      device: None,
      orientation: None,
      headless_command: None,
      scenarios: "scenario bytes".to_owned(),
      aliases: "aliases".to_owned(),
      baselines: "baseline lock".to_owned(),
      seed: 1,
      saves: "save bytes".to_owned(),
      motion: "instant".to_owned(),
    }
  }
}
