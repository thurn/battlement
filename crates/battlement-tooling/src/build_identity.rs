//! Immutable player build identities and no-build explanations.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A player build target whose output bytes are not interchangeable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildTarget {
  Macos,
  Webgl,
  IosSimulator,
}

/// Rust compiler, Cargo, and compilation-target identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustToolchain {
  pub rustc_version: String,
  pub cargo_version: String,
  pub target: String,
}

/// Applicable Xcode and SDK identity for an Apple player target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppleToolchain {
  pub xcode_version: String,
  pub sdk_version: String,
}

/// A versioned capture adapter compiled into the player.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureAdapter {
  pub name: String,
  pub version: String,
}

/// One content-addressed native plugin or binding input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeInput {
  pub name: String,
  pub sha256: String,
}

/// Every byte-affecting input used to derive one player build identity.
///
/// Profile names, displays, devices, orientations, headless commands,
/// scenarios, aliases, baselines, seeds, saves, and motion are runtime inputs
/// and are deliberately absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildIdentityRequest {
  pub source_fingerprint: String,
  pub target: BuildTarget,
  pub unity_version: String,
  pub rust: RustToolchain,
  pub apple: Option<AppleToolchain>,
  pub diagnostics: bool,
  pub capture_adapter: CaptureAdapter,
  pub native_inputs: Vec<NativeInput>,
  pub options: BTreeMap<String, String>,
}

/// One canonical input retained to explain build selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildInput {
  pub name: String,
  pub value: String,
}

/// A stable fingerprint and its sorted diagnostic explanation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
  pub fingerprint: String,
  pub source_fingerprint: String,
  pub inputs: Vec<BuildInput>,
}

/// Whether an exact immutable build satisfies a `--no-build` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NoBuildDecision {
  Reuse {
    fingerprint: String,
  },
  Required {
    expected: String,
    available: Option<String>,
    changed_inputs: Vec<String>,
  },
}

/// Stable baseline coordinates, deliberately independent of build hashes.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineIdentity {
  pub namespace: String,
  pub profile: String,
  pub scenario: String,
  pub checkpoint: String,
}

impl BuildIdentity {
  /// Derives an immutable identity from only byte-affecting build inputs.
  pub fn derive(request: &BuildIdentityRequest) -> Result<Self> {
    self::validate_request(request)?;
    let mut inputs = BTreeMap::from([
      (
        "capture-adapter.name".to_owned(),
        request.capture_adapter.name.clone(),
      ),
      (
        "capture-adapter.version".to_owned(),
        request.capture_adapter.version.clone(),
      ),
      (
        "diagnostics".to_owned(),
        if request.diagnostics {
          "enabled"
        } else {
          "disabled"
        }
        .to_owned(),
      ),
      ("rust.cargo".to_owned(), request.rust.cargo_version.clone()),
      ("rust.rustc".to_owned(), request.rust.rustc_version.clone()),
      ("rust.target".to_owned(), request.rust.target.clone()),
      ("source".to_owned(), request.source_fingerprint.clone()),
      (
        "target".to_owned(),
        self::target_name(request.target).to_owned(),
      ),
      ("unity".to_owned(), request.unity_version.clone()),
    ]);
    if let Some(apple) = &request.apple {
      inputs.insert("apple.sdk".to_owned(), apple.sdk_version.clone());
      inputs.insert("apple.xcode".to_owned(), apple.xcode_version.clone());
    }
    for input in &request.native_inputs {
      self::insert(
        &mut inputs,
        format!("native.{}", input.name),
        input.sha256.clone(),
      )?;
    }
    for (name, value) in &request.options {
      self::insert(&mut inputs, format!("option.{name}"), value.clone())?;
    }
    let mut digest = Sha256::new();
    digest.update(b"battlement-build-v1\0");
    let inputs = inputs
      .into_iter()
      .map(|(name, value)| {
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
        BuildInput { name, value }
      })
      .collect();
    Ok(Self {
      fingerprint: self::hexadecimal(&digest.finalize()),
      source_fingerprint: request.source_fingerprint.clone(),
      inputs,
    })
  }

  /// Explains exact reuse or why a no-build request requires compilation.
  pub fn no_build_decision(&self, available: Option<&Self>) -> NoBuildDecision {
    let Some(available) = available else {
      return NoBuildDecision::Required {
        expected: self.fingerprint.clone(),
        available: None,
        changed_inputs: self.inputs.iter().map(|input| input.name.clone()).collect(),
      };
    };
    if available.fingerprint == self.fingerprint {
      return NoBuildDecision::Reuse {
        fingerprint: self.fingerprint.clone(),
      };
    }
    NoBuildDecision::Required {
      expected: self.fingerprint.clone(),
      available: Some(available.fingerprint.clone()),
      changed_inputs: self::changed_inputs(&self.inputs, &available.inputs),
    }
  }
}

impl BaselineIdentity {
  /// Creates baseline coordinates without source or build fingerprints.
  pub fn new(namespace: &str, profile: &str, scenario: &str, checkpoint: &str) -> Result<Self> {
    for (name, value) in [
      ("namespace", namespace),
      ("profile", profile),
      ("scenario", scenario),
      ("checkpoint", checkpoint),
    ] {
      ensure!(!value.is_empty(), "baseline {name} is empty");
    }
    Ok(Self {
      namespace: namespace.to_owned(),
      profile: profile.to_owned(),
      scenario: scenario.to_owned(),
      checkpoint: checkpoint.to_owned(),
    })
  }
}

fn validate_request(request: &BuildIdentityRequest) -> Result<()> {
  ensure!(
    self::valid_sha256(&request.source_fingerprint),
    "invalid source fingerprint"
  );
  for (name, value) in [
    ("Unity version", request.unity_version.as_str()),
    ("rustc version", request.rust.rustc_version.as_str()),
    ("Cargo version", request.rust.cargo_version.as_str()),
    ("Rust target", request.rust.target.as_str()),
    ("capture adapter", request.capture_adapter.name.as_str()),
    (
      "capture adapter version",
      request.capture_adapter.version.as_str(),
    ),
  ] {
    ensure!(!value.is_empty(), "{name} is empty");
  }
  ensure!(
    request.apple.is_some() == (request.target != BuildTarget::Webgl),
    "Apple toolchain applicability does not match target"
  );
  if let Some(apple) = &request.apple {
    ensure!(!apple.xcode_version.is_empty(), "Xcode version is empty");
    ensure!(!apple.sdk_version.is_empty(), "Apple SDK version is empty");
  }
  let mut native_names = BTreeSet::new();
  for input in &request.native_inputs {
    ensure!(!input.name.is_empty(), "native input name is empty");
    ensure!(native_names.insert(&input.name), "duplicate native input");
    ensure!(
      self::valid_sha256(&input.sha256),
      "invalid native input digest"
    );
  }
  for (name, value) in &request.options {
    ensure!(!name.is_empty(), "build option name is empty");
    ensure!(!value.is_empty(), "build option value is empty");
  }
  Ok(())
}

fn insert(inputs: &mut BTreeMap<String, String>, name: String, value: String) -> Result<()> {
  ensure!(
    inputs.insert(name, value).is_none(),
    "build input name collision"
  );
  Ok(())
}

fn changed_inputs(expected: &[BuildInput], available: &[BuildInput]) -> Vec<String> {
  let expected = expected
    .iter()
    .map(|input| (input.name.as_str(), input.value.as_str()))
    .collect::<BTreeMap<_, _>>();
  let available = available
    .iter()
    .map(|input| (input.name.as_str(), input.value.as_str()))
    .collect::<BTreeMap<_, _>>();
  expected
    .keys()
    .chain(available.keys())
    .filter(|name| expected.get(**name) != available.get(**name))
    .map(|name| (*name).to_owned())
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn target_name(target: BuildTarget) -> &'static str {
  match target {
    BuildTarget::Macos => "macos",
    BuildTarget::Webgl => "webgl",
    BuildTarget::IosSimulator => "ios-simulator",
  }
}

fn valid_sha256(value: &str) -> bool {
  value.len() == 64
    && value
      .bytes()
      .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hexadecimal(bytes: &[u8]) -> String {
  bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
