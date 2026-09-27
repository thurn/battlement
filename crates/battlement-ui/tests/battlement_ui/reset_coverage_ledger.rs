use std::{env, fs, path::Path};

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResettableField {
  fake_route: bool,
  owner: String,
  name: String,
}

#[test]
fn every_resettable_field_has_fake_and_unity_routes() {
  let root =
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
      .join("../..");
  let fields = resettable_fields(&root.join("crates/battlement-ui/src/elements"));
  let unity_runtime = compact(
    &sources(
      &root.join("Packages/com.battlement.client/Runtime/UI"),
      "cs",
    )
    .join("\n"),
  );

  assert!(!fields.is_empty(), "resettable property ledger is empty");
  let mut missing = Vec::new();
  for field in fields {
    if !field.fake_route {
      missing.push(format!(
        "{}.{} has no fake reset route",
        field.owner, field.name
      ));
    }
    if !has_unity_route(&unity_runtime, &field) {
      missing.push(format!(
        "{}.{} has no Unity reset route",
        field.owner, field.name
      ));
    }
  }
  assert!(missing.is_empty(), "{}", missing.join("\n"));
}

fn resettable_fields(directory: &Path) -> Vec<ResettableField> {
  let mut result = Vec::new();
  for source in sources(directory, "rs") {
    let mut owner = None;
    for line in source.lines() {
      let trimmed = line.trim();
      if let Some(name) = trimmed
        .strip_prefix("pub struct ")
        .and_then(|value| value.split_whitespace().next())
      {
        owner = Some(name.trim_end_matches('{').to_owned());
      }
      let Some(field) = trimmed.strip_prefix("pub ") else {
        continue;
      };
      let Some((name, value_type)) = field.split_once(':') else {
        continue;
      };
      if !value_type.trim_start().starts_with("Prop<") {
        continue;
      }
      result.push(ResettableField {
        fake_route: if owner.as_deref() == Some("Style") {
          compact(&source).contains(&format!("{name},"))
        } else {
          compact(&source).contains(&format!("value.{name}"))
        },
        owner: owner.clone().expect("public property belongs to a struct"),
        name: name.to_owned(),
      });
    }
  }
  result.sort();
  result.dedup();
  result
}

fn has_unity_route(source: &str, field: &ResettableField) -> bool {
  let property = pascal_case(&field.name);
  source.contains(&format!(".{property}"))
}

fn sources(directory: &Path, extension: &str) -> Vec<String> {
  let mut result = Vec::new();
  collect_sources(directory, extension, &mut result);
  result
}

fn collect_sources(directory: &Path, extension: &str, result: &mut Vec<String>) {
  for entry in fs::read_dir(directory).expect("source directory exists") {
    let path = entry.expect("source entry is readable").path();
    if path.is_dir() {
      collect_sources(&path, extension, result);
    } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
      result.push(fs::read_to_string(path).expect("source file is readable"));
    }
  }
}

fn compact(value: &str) -> String {
  value
    .chars()
    .filter(|character| !character.is_whitespace())
    .collect()
}

fn pascal_case(value: &str) -> String {
  value
    .split('_')
    .map(|part| {
      let mut characters = part.chars();
      characters
        .next()
        .map(char::to_uppercase)
        .into_iter()
        .flatten()
        .chain(characters)
        .collect::<String>()
    })
    .collect()
}
