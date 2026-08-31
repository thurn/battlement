use std::{collections::BTreeMap, fs, io::Write, path::Path, sync::atomic::AtomicBool};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use toml::Spanned;

use crate::{
  baseline_manifest::ManifestSnapshot,
  cli::GalleryOptions,
  config::{self, model::StepKind},
  gallery_server::{GalleryCheckpoint, GalleryDocument, GalleryServer},
  maintenance_commands, review_commands, storage_commands,
};

pub(crate) fn gallery(
  config_path: Option<&Path>,
  options: GalleryOptions,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let profile = options
    .profile
    .unwrap_or_else(|| suite.default_profile.clone());
  ensure!(
    suite.profiles.contains_key(&profile),
    "profile {profile:?} does not exist"
  );
  let source = fs::read_to_string(&suite.source)
    .with_context(|| format!("read {}", suite.source.display()))?;
  let positions = screenshot_positions(&source)?;
  let screenshots = suite.scenarios.iter().flat_map(|scenario| {
    scenario.steps.iter().filter_map(|step| match &step.action {
      StepKind::Screenshot(screenshot) => Some((scenario.name.as_str(), screenshot.name.as_str())),
      _ => None,
    })
  });
  let screenshots = screenshots.collect::<Vec<_>>();
  ensure!(
    positions.len() == screenshots.len(),
    "suite source layout does not match its screenshot steps"
  );

  let manifest = match suite.baseline.as_ref() {
    Some(baseline) => {
      match ManifestSnapshot::read(&storage_commands::lock_path(&suite))?.manifest {
        Some(manifest) => {
          ensure!(
            manifest.suite == suite.name,
            "ditto.lock suite does not match"
          );
          ensure!(
            manifest.namespace == storage_commands::namespace(baseline),
            "ditto.lock namespace does not match"
          );
          Some(manifest)
        }
        None => None,
      }
    }
    None => None,
  };
  let resources = manifest
    .as_ref()
    .map(|_| {
      Ok::<_, anyhow::Error>((
        storage_commands::read_store(&suite)?,
        maintenance_commands::cache_roots(&suite)?.baselines,
      ))
    })
    .transpose()?;
  let mut images = BTreeMap::new();
  let checkpoints = screenshots
    .into_iter()
    .zip(positions)
    .map(|((scenario, checkpoint), after_line)| {
      let entry = manifest
        .as_ref()
        .and_then(|manifest| manifest.find(&profile, scenario, checkpoint));
      let image = entry
        .map(|entry| {
          let (store, cache) = resources
            .as_ref()
            .expect("manifest resources are available");
          let path = store.hydrate(
            &manifest.as_ref().expect("manifest is available").namespace,
            &entry.sha256,
            cache,
          )?;
          let route = format!("/image/{}.png", entry.sha256);
          images.insert(route.clone(), path);
          Ok::<_, anyhow::Error>(route)
        })
        .transpose()?;
      Ok(GalleryCheckpoint {
        after_line,
        scenario: scenario.to_owned(),
        checkpoint: checkpoint.to_owned(),
        image,
        width: entry.map(|entry| entry.width),
        height: entry.map(|entry| entry.height),
      })
    })
    .collect::<Result<Vec<_>>>()?;
  let server = GalleryServer::bind(
    GalleryDocument {
      suite: suite.name,
      profile,
      filename: suite
        .source
        .file_name()
        .expect("suite source has a file name")
        .to_string_lossy()
        .into_owned(),
      source,
      checkpoints,
    },
    images,
    options.port,
  )?;
  let url = server.url();
  writeln!(stderr, "DITTO_GALLERY_URL={url}")?;
  stderr.flush()?;
  if !options.no_open {
    review_commands::open_browser(&url)?;
  }
  server.serve(interrupted)?;
  Ok(0)
}

#[derive(Deserialize)]
struct GallerySource {
  scenarios: Vec<GallerySourceScenario>,
}

#[derive(Deserialize)]
struct GallerySourceScenario {
  steps: Vec<GallerySourceStep>,
}

#[derive(Deserialize)]
struct GallerySourceStep {
  screenshot: Option<Spanned<toml::Value>>,
}

fn screenshot_positions(source: &str) -> Result<Vec<usize>> {
  let parsed: GallerySource = toml::from_str(source).context("parse gallery source locations")?;
  Ok(
    parsed
      .scenarios
      .into_iter()
      .flat_map(|scenario| scenario.steps)
      .filter_map(|step| step.screenshot)
      .map(|screenshot| {
        let span = screenshot.span();
        if source[span.clone()].starts_with('[') {
          table_end_line(source, span.end)
        } else {
          source[..span.end].lines().count()
        }
      })
      .collect(),
  )
}

fn table_end_line(source: &str, value_end: usize) -> usize {
  let start_line = source[..value_end].lines().count();
  source
    .lines()
    .enumerate()
    .skip(start_line)
    .find_map(|(index, line)| {
      let line = line.trim();
      (line.starts_with('[') && line.ends_with(']')).then_some(index)
    })
    .unwrap_or_else(|| source.lines().count())
}

#[cfg(test)]
mod tests {
  use super::screenshot_positions;

  #[test]
  fn locates_complete_screenshot_steps_in_source_order() {
    let source = r#"[[scenarios]]
name = "opening"

[[scenarios.steps]]
screenshot = {
  name = "board"
}

[[scenarios.steps]]
click = { target = "board" }

[[scenarios.steps]]
screenshot = { name = "moved" }
"#;
    assert_eq!(screenshot_positions(source).unwrap(), [7, 13]);
  }

  #[test]
  fn locates_equivalent_toml_screenshot_forms() {
    for (source, expected) in [
      (
        "[[scenarios]]\nname='opening'\n[[scenarios.steps]]\nscreenshot.name='board'\n",
        4,
      ),
      (
        "[[scenarios]]\nname='opening'\n[[scenarios.steps]]\n\"screenshot\"={name='board'}\n",
        4,
      ),
      (
        "[[scenarios]]\nname='opening'\n[[scenarios.steps]]\n[scenarios.steps.screenshot]\nname='board'\n",
        5,
      ),
    ] {
      assert_eq!(screenshot_positions(source).unwrap(), [expected]);
    }
  }
}
