use std::{
  collections::BTreeSet,
  ffi::OsString,
  io,
  path::{Path, PathBuf},
  sync::{Arc, Mutex, atomic::AtomicBool},
};

use anyhow::{Context, Result, ensure};
use battlement_ditto::{config::model::Suite, preparation::PlayerPreparation};

use crate::project::Overrides;

type AssetPreparation = dyn Fn(&Path, &Path) -> Result<()> + Send + Sync;

struct ReactantPreparation {
  prepared: Mutex<BTreeSet<PathBuf>>,
  prepare_assets: Arc<AssetPreparation>,
}

impl ReactantPreparation {
  fn new(prepare_assets: Arc<AssetPreparation>) -> Self {
    Self {
      prepared: Mutex::new(BTreeSet::new()),
      prepare_assets,
    }
  }
}

impl PlayerPreparation for ReactantPreparation {
  fn prepare(&self, suite: &Suite) -> Result<()> {
    if !suite.player.reactant {
      return Ok(());
    }
    let mut prepared = self.prepared.lock().expect("preparation lock is available");
    if prepared.contains(&suite.source) {
      return Ok(());
    }
    let project = crate::project::resolve(
      suite.player.unity_project.clone(),
      Overrides::default(),
      false,
    )?;
    ensure!(
      project.manifest == suite.player.rust_manifest,
      "Ditto rust_manifest {} does not match reactant.toml manifest-path {}",
      suite.player.rust_manifest.display(),
      project.manifest.display()
    );
    (self.prepare_assets)(&project.root, &project.manifest)?;
    prepared.insert(suite.source.clone());
    Ok(())
  }
}

pub(crate) fn run(
  config: Option<PathBuf>,
  arguments: Vec<OsString>,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let mut forwarded = vec![OsString::from("ditto")];
  if let Some(config) = config {
    forwarded.push(OsString::from("--config"));
    forwarded.push(config.into_os_string());
  } else if !arguments.iter().any(|argument| {
    argument == "--help" || argument == "-h" || argument == "--version" || argument == "-V"
  }) {
    anyhow::bail!("rt ditto requires an explicit --config path");
  }
  forwarded.extend(arguments);
  Ok(battlement_ditto::process_from_with_preparation(
    forwarded,
    &mut io::stdout(),
    &mut io::stderr(),
    interrupted,
    Arc::new(ReactantPreparation::new(Arc::new(prepare_assets))),
  ))
}

fn prepare_assets(project: &Path, manifest: &Path) -> Result<()> {
  super::command::prepare_assets(project, manifest)
    .with_context(|| format!("failed to prepare Reactant project {}", project.display()))
}

#[cfg(test)]
mod tests {
  use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
  };

  use super::*;

  #[test]
  fn reactant_preparation_runs_once_and_direct_players_skip_it() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    fs::create_dir_all(root.join("Assets/Scenes"))?;
    fs::create_dir_all(root.join("rules"))?;
    fs::write(root.join("Assets/Scenes/Game.unity"), "")?;
    fs::write(root.join("rules/Cargo.toml"), "")?;
    fs::write(
      root.join("reactant.toml"),
      "[project]\napplication = \"Game\"\nscene = \"Assets/Scenes/Game.unity\"\n",
    )?;
    let status = std::process::Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(root)
      .status()?;
    assert!(status.success());
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let preparation = ReactantPreparation::new(Arc::new(move |_, _| {
      observed.fetch_add(1, Ordering::SeqCst);
      Ok(())
    }));

    fs::write(root.join("ditto.toml"), suite(true))?;
    let reactant = battlement_ditto::config::load(Some(&root.join("ditto.toml")))?;
    preparation.prepare(&reactant)?;
    preparation.prepare(&reactant)?;
    assert_eq!(count.load(Ordering::SeqCst), 1);

    fs::write(root.join("ditto.toml"), suite(false))?;
    let direct = battlement_ditto::config::load(Some(&root.join("ditto.toml")))?;
    preparation.prepare(&direct)?;
    assert_eq!(count.load(Ordering::SeqCst), 1);

    fs::write(
      root.join("reactant.toml"),
      "[project]\napplication = \"Game\"\nscene = \"Assets/Scenes/Game.unity\"\nmanifest-path = \"rules/other.toml\"\n",
    )?;
    let mismatch = ReactantPreparation::new(Arc::new(|_, _| Ok(())));
    let error = mismatch.prepare(&reactant).unwrap_err().to_string();
    assert!(error.contains("does not match reactant.toml manifest-path"));
    Ok(())
  }

  fn suite(reactant: bool) -> String {
    format!(
      r#"name = "fixture"
default_profile = "macos"

[player]
reactant = {reactant}
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[profiles.macos]
target = "macos"
display = {{ width = 1280, height = 720, scale = 1.0 }}

[[scenarios]]
name = "fixture"
motion = "controlled"

[[scenarios.steps]]
advance = {{ frames = 1 }}
"#
    )
  }
}
