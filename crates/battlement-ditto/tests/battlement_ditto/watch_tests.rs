use std::{
  fs,
  path::PathBuf,
  time::{Duration, Instant},
};

use battlement_ditto::watch::{ChangeSet, CyclePath, FileObserver, PendingState};

#[test]
fn file_bursts_debounce_and_classify_distinct_paths() {
  let temporary = tempfile::tempdir().unwrap();
  fs::create_dir_all(temporary.path().join("src")).unwrap();
  fs::write(temporary.path().join("ditto.toml"), "suite").unwrap();
  fs::write(temporary.path().join("ditto.lock"), "lock").unwrap();
  fs::write(temporary.path().join("src/lib.rs"), "source").unwrap();
  let mut observer = FileObserver::new(
    temporary.path(),
    [PathBuf::from("ditto.toml")],
    PathBuf::from("ditto.lock"),
    Duration::from_millis(20),
  )
  .unwrap();
  let origin = Instant::now();

  fs::write(temporary.path().join("ditto.toml"), "changed suite").unwrap();
  assert_eq!(observer.poll(origin).unwrap(), None);
  fs::write(temporary.path().join("ditto.lock"), "changed lock").unwrap();
  fs::write(temporary.path().join("src/lib.rs"), "changed source").unwrap();
  assert_eq!(
    observer.poll(origin + Duration::from_millis(10)).unwrap(),
    None
  );
  assert_eq!(
    observer.poll(origin + Duration::from_millis(31)).unwrap(),
    Some(ChangeSet {
      scenario: true,
      lock: true,
      source: true,
      retry_broken_build: false,
    })
  );
}

#[test]
fn one_pending_state_preserves_broken_build_rules() {
  let mut state = PendingState::default();
  assert_eq!(
    state
      .begin(ChangeSet {
        source: true,
        ..ChangeSet::default()
      })
      .unwrap(),
    CyclePath::ReplacementBuild
  );
  state.enqueue(ChangeSet {
    scenario: true,
    ..ChangeSet::default()
  });
  let pending = state.finish(Some(false)).unwrap();
  assert!(state.source_is_broken());
  assert!(state.begin(pending).is_err());
  assert_eq!(
    state.begin(state.retry().unwrap()).unwrap(),
    CyclePath::ReplacementBuild
  );
  state.enqueue(ChangeSet {
    lock: true,
    ..ChangeSet::default()
  });
  assert_eq!(
    state.finish(Some(true)).unwrap().path(),
    Some(CyclePath::ComparisonOnly)
  );
  assert!(!state.source_is_broken());
}

#[test]
fn simultaneous_changes_choose_one_replacement_build() {
  assert_eq!(
    ChangeSet {
      scenario: true,
      lock: true,
      source: true,
      retry_broken_build: false,
    }
    .path(),
    Some(CyclePath::ReplacementBuild)
  );
}

#[test]
fn an_external_fragment_is_observed_as_scenario_input() {
  let repository = tempfile::tempdir().unwrap();
  let fragment_root = tempfile::tempdir().unwrap();
  let fragment = fragment_root.path().join("cycle.toml");
  fs::write(&fragment, "first").unwrap();
  let mut observer = FileObserver::new(
    repository.path(),
    [fragment.clone()],
    repository.path().join("ditto.lock"),
    Duration::from_millis(10),
  )
  .unwrap();
  let origin = Instant::now();
  fs::write(fragment, "second").unwrap();
  assert_eq!(observer.poll(origin).unwrap(), None);
  assert_eq!(
    observer.poll(origin + Duration::from_millis(11)).unwrap(),
    Some(ChangeSet {
      scenario: true,
      ..ChangeSet::default()
    })
  );
}
