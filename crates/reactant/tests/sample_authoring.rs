use std::{fs, path::Path};

const FORBIDDEN: [&str; 9] = [
  "impl Engine",
  "battlement_native",
  "battlement-native",
  "ReactantChessApp",
  "TicTacToeEngine",
  "reactant::app::App",
  "reactant::testing",
  "GameApp",
  "reactant::__native",
];

#[test]
fn ordinary_reactant_samples_do_not_expose_transport_boilerplate() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("Reactant crate lives under the repository crates directory");
  for sample in ["chess", "tictactoe", "reactant", "chess-ui"] {
    check_directory(&root.join("samples").join(sample));
  }
}

fn check_directory(directory: &Path) {
  for entry in fs::read_dir(directory).expect("sample directory is readable") {
    let path = entry.expect("sample entry is readable").path();
    if path.is_dir() {
      check_directory(&path);
      continue;
    }
    if !matches!(
      path.extension().and_then(|extension| extension.to_str()),
      Some("rs" | "toml")
    ) || path.file_name().and_then(|name| name.to_str()) == Some("Cargo.lock")
    {
      continue;
    }
    let source = fs::read_to_string(&path).expect("sample source is UTF-8");
    for forbidden in FORBIDDEN {
      assert!(
        !source.contains(forbidden),
        "{} contains forbidden Reactant authoring surface {forbidden:?}",
        path.display(),
      );
    }
  }
}
