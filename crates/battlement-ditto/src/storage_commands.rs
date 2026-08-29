use std::{
  collections::BTreeSet,
  io::Write,
  path::{Path, PathBuf},
  thread,
  time::Duration,
};

use anyhow::{Context, Result, ensure};
use time::OffsetDateTime;

use crate::{
  baseline_manifest::{BaselineManifest, ManifestSnapshot},
  baseline_publication::{self, ConditionalObjectStore, R2CredentialNames, R2Credentials},
  baseline_store::{BaselineStore, FilesystemBaselineStore},
  cli::{FetchOptions, SelectionOptions, StorageCommand},
  config::{
    self,
    model::{Baseline, Suite},
  },
  filesystem_publication_store::FilesystemPublicationStore,
  maintenance_commands,
  r2_baseline_store::{self, FetchSelection, R2BaselineStore},
  r2_publication_store::R2PublicationStore,
  selection,
};

pub(crate) fn fetch(
  config_path: Option<&Path>,
  options: FetchOptions,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let (manifest, _) = manifest(&suite)?;
  let store = read_store(&suite)?;
  let selection = if options.all {
    FetchSelection::All
  } else {
    let selected = selection::resolve(&suite, &selection_options(options.selection))?;
    let names = selected
      .scenarios
      .iter()
      .map(|scenario| scenario.scenario.name.clone())
      .collect::<BTreeSet<_>>();
    let fetched = r2_baseline_store::fetch(
      store.as_ref(),
      &manifest,
      &maintenance_commands::cache_roots(&suite)?.baselines,
      FetchSelection::Selected {
        profile: &selected.profile_name,
        scenarios: &names,
      },
      parallelism(),
    )?;
    return print_paths(fetched, stdout);
  };
  print_paths(
    r2_baseline_store::fetch(
      store.as_ref(),
      &manifest,
      &maintenance_commands::cache_roots(&suite)?.baselines,
      selection,
      parallelism(),
    )?,
    stdout,
  )
}

pub(crate) fn storage(
  config_path: Option<&Path>,
  command: StorageCommand,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let (manifest, lock_sha256) = manifest(&suite)?;
  let store = publication_store(&suite)?;
  match command {
    StorageCommand::Publish => {
      let result = baseline_publication::publish(
        store.as_ref(),
        &manifest,
        &lock_sha256,
        OffsetDateTime::now_utc(),
      )?;
      writeln!(stdout, "published generation {}", result.state.generation)?;
      for warning in result.warnings {
        writeln!(stdout, "warning: {warning}")?;
      }
    }
  }
  Ok(0)
}

pub(crate) fn clean_storage(
  config_path: Option<&Path>,
  apply: bool,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let baseline = suite
    .baseline
    .as_ref()
    .context("suite has no baseline store")?;
  let namespace = namespace(baseline);
  let store = publication_store(&suite)?;
  let now = OffsetDateTime::now_utc();
  let plan = baseline_publication::clean_storage(store.as_ref(), namespace, now, false)?;
  let bytes = plan.eligible_sha256.iter().try_fold(0_u64, |total, hash| {
    Ok::<_, anyhow::Error>(
      total.saturating_add(
        store
          .get(&object_key(namespace, hash))?
          .map_or(0, |object| object.bytes.len() as u64),
      ),
    )
  })?;
  writeln!(
    stdout,
    "clean storage: {} objects, {bytes} bytes{}",
    plan.eligible_sha256.len(),
    if apply { " (apply)" } else { " (dry run)" }
  )?;
  for hash in &plan.eligible_sha256 {
    writeln!(stdout, "  {hash}")?;
  }
  stdout.flush()?;
  if apply {
    let result = baseline_publication::apply_cleanup_plan(store.as_ref(), namespace, now, &plan)?;
    ensure!(result.applied, "storage cleanup was not applied");
  }
  Ok(0)
}

pub(crate) fn manifest(suite: &Suite) -> Result<(BaselineManifest, String)> {
  let baseline = suite
    .baseline
    .as_ref()
    .context("suite has no baseline store")?;
  let snapshot = ManifestSnapshot::read(&lock_path(suite))?;
  let manifest = snapshot.manifest.context("ditto.lock is missing")?;
  ensure!(
    manifest.suite == suite.name,
    "ditto.lock suite does not match"
  );
  ensure!(
    manifest.namespace == namespace(baseline),
    "ditto.lock namespace does not match"
  );
  Ok((
    manifest,
    snapshot.sha256.expect("present manifest has a digest"),
  ))
}

pub(crate) fn read_store(suite: &Suite) -> Result<Box<dyn BaselineStore>> {
  Ok(
    match suite
      .baseline
      .as_ref()
      .context("suite has no baseline store")?
    {
      Baseline::Filesystem { root, .. } => Box::new(FilesystemBaselineStore::new(root.clone())),
      Baseline::R2 {
        public_base_url, ..
      } => Box::new(R2BaselineStore::new(
        public_base_url.clone(),
        Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      )),
    },
  )
}

pub(crate) fn write_store(suite: &Suite) -> Result<Box<dyn BaselineStore>> {
  Ok(
    match suite
      .baseline
      .as_ref()
      .context("suite has no baseline store")?
    {
      Baseline::Filesystem { root, .. } => Box::new(FilesystemBaselineStore::new(root.clone())),
      Baseline::R2 {
        public_base_url,
        account_id_env,
        bucket_env,
        access_key_id_env,
        secret_access_key_env,
        ..
      } => Box::new(R2PublicationStore::new(
        R2Credentials::from_environment(R2CredentialNames {
          account_id: account_id_env,
          bucket: bucket_env,
          access_key_id: access_key_id_env,
          secret_access_key: secret_access_key_env,
        })?,
        public_base_url.clone(),
        Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      )?),
    },
  )
}

fn publication_store(suite: &Suite) -> Result<Box<dyn ConditionalObjectStore>> {
  Ok(
    match suite
      .baseline
      .as_ref()
      .context("suite has no baseline store")?
    {
      Baseline::Filesystem { root, .. } => Box::new(FilesystemPublicationStore::new(root.clone())),
      Baseline::R2 {
        public_base_url,
        account_id_env,
        bucket_env,
        access_key_id_env,
        secret_access_key_env,
        ..
      } => Box::new(R2PublicationStore::new(
        R2Credentials::from_environment(R2CredentialNames {
          account_id: account_id_env,
          bucket: bucket_env,
          access_key_id: access_key_id_env,
          secret_access_key: secret_access_key_env,
        })?,
        public_base_url.clone(),
        Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      )?),
    },
  )
}

fn namespace(baseline: &Baseline) -> &str {
  match baseline {
    Baseline::Filesystem { namespace, .. } | Baseline::R2 { namespace, .. } => namespace,
  }
}

fn lock_path(suite: &Suite) -> PathBuf {
  suite
    .source
    .parent()
    .expect("suite source has a parent")
    .join("ditto.lock")
}

fn selection_options(options: SelectionOptions) -> selection::Options {
  selection::Options {
    profile: options.profile,
    includes: options.includes,
    excludes: options.excludes,
    allow_empty: options.allow_empty,
  }
}

fn parallelism() -> usize {
  thread::available_parallelism()
    .map(usize::from)
    .unwrap_or(1)
    .min(8)
}

fn print_paths(paths: Vec<PathBuf>, stdout: &mut dyn Write) -> Result<u8> {
  for path in paths {
    writeln!(stdout, "{}", path.display())?;
  }
  Ok(0)
}

fn object_key(namespace: &str, sha256: &str) -> String {
  format!("{namespace}/objects/{}/{sha256}.png", &sha256[..2])
}
