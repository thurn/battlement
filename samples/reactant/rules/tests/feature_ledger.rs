use std::collections::BTreeSet;

const LEDGER: &str = include_str!("../../../../docs/reactant/feature-ledger.md");
const LIBRARY: &str = include_str!("../../../../crates/battlement-reactant/src/lib.rs");
const PROOF_SOURCES: &[(&str, &str)] = &[
  ("composition.rs", include_str!("composition.rs")),
  (
    "element_refs.rs",
    include_str!("../../../../crates/battlement-reactant/tests/element_refs.rs"),
  ),
  (
    "error_boundaries.rs",
    include_str!("../../../../crates/battlement-reactant/tests/error_boundaries.rs"),
  ),
  (
    "event_catalog.rs",
    include_str!("../../../../crates/battlement-reactant/tests/event_catalog.rs"),
  ),
  (
    "external_portals.rs",
    include_str!("../../../../crates/battlement-reactant/tests/external_portals.rs"),
  ),
  (
    "external_stores.rs",
    include_str!("../../../../crates/battlement-reactant/tests/external_stores.rs"),
  ),
  (
    "focus_protocol_tests.rs",
    include_str!("../../../../crates/battlement-ui/tests/focus_protocol_tests.rs"),
  ),
  (
    "BattlementFocusCoordinatorTests.cs",
    include_str!(
      "../../../../Packages/com.battlement.client/Tests/Editor/BattlementFocusCoordinatorTests.cs"
    ),
  ),
  (
    "geometry.rs",
    include_str!("../../../../crates/battlement-reactant/tests/geometry.rs"),
  ),
  (
    "geometry_effects.rs",
    include_str!("../../../../crates/battlement-reactant/tests/geometry_effects.rs"),
  ),
  (
    "generated_assets.rs",
    include_str!("../../../../crates/battlement-reactant/tests/generated_assets.rs"),
  ),
  (
    "hook_scheduling.rs",
    include_str!("../../../../crates/battlement-reactant/tests/hook_scheduling.rs"),
  ),
  (
    "identity.rs",
    include_str!("../../../../crates/battlement-reactant/tests/identity.rs"),
  ),
  (
    "lifecycle.rs",
    include_str!("../../../../crates/battlement-reactant/tests/lifecycle.rs"),
  ),
  (
    "LayoutProjectionTests.cs",
    include_str!(
      "../../../../Packages/com.battlement.client/Tests/Editor/LayoutProjectionTests.cs"
    ),
  ),
  (
    "moves.rs",
    include_str!("../../../../crates/battlement-reactant/tests/moves.rs"),
  ),
  (
    "motion.rs",
    include_str!("../../../../crates/battlement-reactant/tests/motion.rs"),
  ),
  (
    "portals.rs",
    include_str!("../../../../crates/battlement-reactant/tests/portals.rs"),
  ),
  (
    "presence.rs",
    include_str!("../../../../crates/battlement-reactant/tests/presence.rs"),
  ),
  (
    "primitives.rs",
    include_str!("../../../../crates/battlement-reactant/tests/primitives.rs"),
  ),
  (
    "propagation.rs",
    include_str!("../../../../crates/battlement-reactant/tests/propagation.rs"),
  ),
  (
    "refs_context.rs",
    include_str!("../../../../crates/battlement-reactant/tests/refs_context.rs"),
  ),
  (
    "required_props.rs",
    include_str!("../../../../crates/battlement-reactant/tests/required_props.rs"),
  ),
  (
    "resources.rs",
    include_str!("../../../../crates/battlement-reactant/tests/resources.rs"),
  ),
  (
    "runtime.rs",
    include_str!("../../../../crates/battlement-reactant/tests/runtime.rs"),
  ),
  (
    "state.rs",
    include_str!("../../../../crates/battlement-reactant/tests/state.rs"),
  ),
];

#[test]
fn every_public_reactant_module_has_a_screen_and_black_box_proof() {
  let public_modules = LIBRARY
    .lines()
    .filter_map(|line| line.strip_prefix("pub mod "))
    .map(|module| module.trim_end_matches(';'))
    .collect::<BTreeSet<_>>();
  let rows = table_rows("| Public module |", "The remaining focused screens");
  let mapped_modules = rows
    .iter()
    .map(|row| row[0].trim_matches('`'))
    .collect::<BTreeSet<_>>();

  assert_eq!(mapped_modules, public_modules);
  assert!(
    rows
      .iter()
      .all(|row| { !row[1].is_empty() && row[2].contains(".rs") && row[2].contains("::") })
  );
  for row in &rows {
    for proof in row[2].split(',') {
      let proof = proof.trim().trim_matches('`');
      let (file, test) = proof
        .split_once("::")
        .map_or((proof, None), |(file, test)| (file, Some(test)));
      let source = PROOF_SOURCES
        .iter()
        .find_map(|(candidate, source)| (*candidate == file).then_some(*source))
        .unwrap_or_else(|| panic!("ledger references unknown proof file {file}"));
      if let Some(test) = test {
        assert!(
          source.contains(&format!("fn {test}(")),
          "ledger references missing test {file}::{test}"
        );
      }
    }
  }
  assert_eq!(
    rows.iter().map(|row| row[1]).collect::<BTreeSet<_>>(),
    BTreeSet::from([
      "Assets",
      "Composition",
      "Context & Memo",
      "Effects & Stores",
      "Events & Portals",
      "Gestures & Drag",
      "Layout & Reorder",
      "Layout Gallery",
      "Composed Effects",
      "Presence & Lifecycle",
      "Refs & Geometry",
      "Resources & Boundaries",
      "State & Identity",
      "Targets & Timelines",
      "Values, Time & Controls",
    ])
  );
}

#[test]
fn reserved_react_apis_are_explicitly_unsupported() {
  let rows = table_rows("| Reserved API |", "");
  assert_eq!(
    rows
      .iter()
      .map(|row| row[0].trim_matches('`'))
      .collect::<BTreeSet<_>>(),
    BTreeSet::from([
      "StrictMode",
      "use_id",
      "use_layout_effect",
      "use_sync_external_store",
    ])
  );
  assert!(rows.iter().all(|row| row[1] == "Unsupported"));
}

fn table_rows(start: &str, end: &str) -> Vec<Vec<&'static str>> {
  let table = LEDGER
    .split_once(start)
    .expect("ledger table heading should exist")
    .1;
  let table = if end.is_empty() {
    table
  } else {
    table
      .split_once(end)
      .expect("ledger table terminator should exist")
      .0
  };
  table
    .lines()
    .skip(2)
    .take_while(|line| line.starts_with('|'))
    .map(|line| {
      line
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>()
    })
    .collect()
}
