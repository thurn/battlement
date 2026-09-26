//! Build choices that distinguish physical iOS artifacts from simulators.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use crate::build_identity::BuildTarget;

/// Explicit signing inputs supplied by the caller; provisioning is never automatic.
#[derive(Clone, Debug)]
pub struct IosSigning {
  pub team: String,
  pub identity: String,
  pub profile_uuid: String,
  pub profile_fingerprint: String,
}

/// An arm64 simulator or physical-device build, with explicit signing policy.
#[derive(Clone, Debug)]
pub enum IosTarget {
  Simulator,
  Device { signing: Option<IosSigning> },
}

impl IosTarget {
  pub fn platform(&self) -> &'static str {
    match self {
      Self::Simulator => "ios-simulator",
      Self::Device { .. } => "ios-device",
    }
  }

  pub fn build_target(&self) -> BuildTarget {
    match self {
      Self::Simulator => BuildTarget::IosSimulator,
      Self::Device { .. } => BuildTarget::IosDevice,
    }
  }

  pub fn sdk(&self) -> &'static str {
    match self {
      Self::Simulator => "iphonesimulator",
      Self::Device { .. } => "iphoneos",
    }
  }

  pub fn rust_target(&self) -> &'static str {
    match self {
      Self::Simulator => "aarch64-apple-ios-sim",
      Self::Device { .. } => "aarch64-apple-ios",
    }
  }

  pub fn rust_flags_variable(&self) -> &'static str {
    match self {
      Self::Simulator => "CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUSTFLAGS",
      Self::Device { .. } => "CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS",
    }
  }

  pub fn editor_method(&self) -> &'static str {
    match self {
      Self::Simulator => "Battlement.Editor.BattlementDittoBuild.BuildIosSimulator",
      Self::Device { .. } => "Battlement.Editor.BattlementDittoBuild.BuildIosDevice",
    }
  }

  pub fn signing_arguments(&self) -> Vec<String> {
    match self {
      Self::Device {
        signing: Some(signing),
      } => vec![
        "CODE_SIGNING_ALLOWED=YES".to_owned(),
        "CODE_SIGN_STYLE=Manual".to_owned(),
        format!("DEVELOPMENT_TEAM={}", signing.team),
        format!("CODE_SIGN_IDENTITY={}", signing.identity),
      ],
      _ => vec!["CODE_SIGNING_ALLOWED=NO".to_owned()],
    }
  }

  pub fn profile_uuid(&self) -> &str {
    match self {
      Self::Device {
        signing: Some(signing),
      } => &signing.profile_uuid,
      _ => "",
    }
  }

  pub fn retain_signing(&self, options: &mut BTreeMap<String, String>) -> Result<()> {
    if let Self::Device {
      signing: Some(signing),
    } = self
    {
      for (key, value) in [
        ("signing-team", &signing.team),
        ("signing-identity", &signing.identity),
        ("signing-profile", &signing.profile_uuid),
        ("signing-profile-fingerprint", &signing.profile_fingerprint),
      ] {
        ensure!(!value.trim().is_empty(), "{key} must not be empty");
        options.insert(key.to_owned(), value.clone());
      }
    }
    options.insert(
      "signing".to_owned(),
      if self.is_signed() {
        "manual"
      } else {
        "unsigned"
      }
      .to_owned(),
    );
    Ok(())
  }

  pub fn is_signed(&self) -> bool {
    matches!(self, Self::Device { signing: Some(_) })
  }
}
