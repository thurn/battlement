use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
};

use anyhow::bail;

use crate::{WorkReport, manifest_validation::GeneratedSet, transaction};

#[test]
fn rejected_final_validation_restores_root_and_metadata() {
  let temporary = tempfile::tempdir().unwrap();
  let project = temporary.path();
  let root = project.join("Assets/Generated/BattlementReactant");
  let metadata = project.join("Assets/Generated/BattlementReactant.meta");
  fs::create_dir_all(&root).unwrap();
  fs::write(root.join("previous"), b"previous root").unwrap();
  fs::write(&metadata, b"previous metadata").unwrap();
  let set = GeneratedSet {
    directories: BTreeSet::from(["Assets/Generated/BattlementReactant".to_owned()]),
    files: BTreeMap::from([
      (
        "Assets/Generated/BattlementReactant.meta".to_owned(),
        b"replacement metadata".to_vec(),
      ),
      (
        "Assets/Generated/BattlementReactant/replacement".to_owned(),
        b"replacement root".to_vec(),
      ),
    ]),
  };

  let result = transaction::install(project, &set, &mut WorkReport::default(), |_| {
    bail!("injected final validation failure")
  });

  assert!(result.is_err());
  assert_eq!(fs::read(root.join("previous")).unwrap(), b"previous root");
  assert_eq!(fs::read(metadata).unwrap(), b"previous metadata");
  assert!(!root.join("replacement").exists());
}
