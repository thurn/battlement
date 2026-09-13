use battlement_flatbuffers::{ConnectInput, ConnectView, ReducedMotionPreference, write_connect};

fn input<'a>(custom_command_types: &'a [&'a str], modules: &'a [&'a str]) -> ConnectInput<'a> {
  ConnectInput {
    platform: "macOS 日本語 🚀",
    unity_version: "6000.5.8f1",
    screen_width: 2560,
    screen_height: 1440,
    focused: true,
    paused: false,
    reduced_motion_preference: ReducedMotionPreference::Reduce,
    custom_command_types,
    modules,
    persistent_data_path: Some("/tmp/保存"),
    streaming_assets_path: None,
  }
}

#[test]
fn direct_connect_round_trip_preserves_borrowed_values_and_allocation() {
  let finished = write_connect(&input(&["cards.draw", "cards.shuffle"], &["core", "ui"])).unwrap();
  assert!(finished.allocation_bytes() >= finished.as_bytes().len());
  let view = ConnectView::read(finished.as_bytes()).unwrap();
  assert_eq!(view.platform(), "macOS 日本語 🚀");
  assert_eq!(view.unity_version(), "6000.5.8f1");
  assert_eq!((view.screen_width(), view.screen_height()), (2560, 1440));
  assert!(view.focused());
  assert!(!view.paused());
  assert_eq!(
    view.reduced_motion_preference(),
    ReducedMotionPreference::Reduce
  );
  assert_eq!(view.custom_command_type(1), Some("cards.shuffle"));
  assert_eq!(view.module(0), Some("core"));
  assert_eq!(view.persistent_data_path(), Some("/tmp/保存"));
  assert_eq!(view.streaming_assets_path(), None);
}

#[test]
fn verifier_rejects_truncation_and_wrong_identifier() {
  let finished = write_connect(&input(&[], &[])).unwrap();
  for length in 0..finished.as_bytes().len() {
    assert!(
      ConnectView::read(&finished.as_bytes()[..length]).is_err(),
      "accepted prefix length {length} of {}",
      finished.as_bytes().len()
    );
  }
  let mut wrong_identifier = finished.as_bytes().to_vec();
  wrong_identifier[4..8].copy_from_slice(b"NOPE");
  assert!(ConnectView::read(&wrong_identifier).is_err());
}

#[test]
fn semantic_validation_rejects_noncanonical_collections() {
  assert!(write_connect(&input(&["cards.shuffle", "cards.draw"], &[])).is_err());
  assert!(write_connect(&input(&["cards.draw", "cards.draw"], &[])).is_err());
  assert!(write_connect(&input(&[], &["ui", "ui"])).is_err());
}
