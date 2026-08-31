use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Parent/child sequencing selected by a resolved variant target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VariantWhen {
  /// Parent and children begin from the same orchestration origin.
  Together,
  /// Parent completion releases its children.
  BeforeChildren,
  /// Child completion releases the parent.
  AfterChildren,
}

/// Direction used to assign stagger positions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StaggerDirection {
  /// Earlier logical children begin first.
  Forward,
  /// Later logical children begin first.
  Reverse,
}

/// Inspectable facts retained after Rust resolves logical variants.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionVariantResolution {
  /// Ordered names merged into the final target.
  pub names: Vec<String>,
  /// Whether the names came from the logical parent.
  pub inherited: bool,
  /// Snapshot identity of custom data used by computed targets.
  pub custom_snapshot: u64,
  /// Direct logical-child position used for stagger scheduling.
  pub child_index: u32,
  /// Final orchestration delay applied to this host.
  pub delay_micros: u64,
  /// Parent/child sequencing selected by this host.
  pub when: VariantWhen,
  /// Stagger direction selected by this host.
  pub stagger_direction: StaggerDirection,
}

impl MotionVariantResolution {
  pub(crate) fn validate(&self) -> Result<(), String> {
    if self.names.is_empty() {
      return Err("resolved variant names cannot be empty".to_owned());
    }
    let mut names = HashSet::new();
    for name in &self.names {
      if name.trim().is_empty() {
        return Err("resolved variant names cannot be blank".to_owned());
      }
      if !names.insert(name) {
        return Err("resolved variant names cannot repeat".to_owned());
      }
    }
    Ok(())
  }
}
