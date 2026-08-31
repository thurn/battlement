use anyhow::Result;
use battlement_reactant_assets::{AssetCommand, CommandOptions, FeatureSelection};

use crate::config::model::Suite;

pub(crate) fn generate(suite: &Suite) -> Result<()> {
  if !suite
    .player
    .unity_project
    .join("Packages/manifest.json")
    .is_file()
  {
    return Ok(());
  }
  battlement_reactant_assets::run_quiet(
    AssetCommand::Generate,
    &CommandOptions {
      project: Some(suite.player.unity_project.clone()),
      manifest_path: Some(suite.player.rust_manifest.clone()),
      feature_selection: FeatureSelection::default(),
      browser: None,
      work_report: None,
    },
  )
}
