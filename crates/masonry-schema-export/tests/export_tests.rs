use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
fn exporter_writes_complete_disposable_draft_7_bundle() {
    let output = temporary_directory();
    fs::create_dir(&output).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_masonry-schema-export"))
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let filenames = filenames(&output);
    assert_eq!(filenames.len(), 7);
    for required in [
        "connect.schema.json",
        "response.schema.json",
        "client-message.schema.json",
        "snapshot.schema.json",
        "batch.schema.json",
        "command.schema.json",
        "quicktype-bundle.schema.json",
    ] {
        assert!(filenames.iter().any(|filename| filename == required));
    }

    let connect = schema(&output, "connect.schema.json");
    assert_eq!(
        connect["$schema"],
        "http://json-schema.org/draft-07/schema#"
    );
    assert!(connect.to_string().contains("masonry.connect"));
    assert!(
        connect["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "screen")
    );

    let command = schema(&output, "command.schema.json");
    assert!(command.to_string().contains("masonry.object.create"));
    assert!(command.to_string().contains("masonry.audio.play"));

    let quicktype_bundle = schema(&output, "quicktype-bundle.schema.json");
    assert_eq!(
        quicktype_bundle["$schema"],
        "http://json-schema.org/draft-07/schema#"
    );
    assert!(
        quicktype_bundle
            .to_string()
            .contains("CameraClippingPayload")
    );

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(!repository.join("schema").exists());
    assert!(!repository.join("schemas").exists());

    fs::remove_dir_all(output).unwrap();
}

fn schema(directory: &Path, filename: &str) -> Value {
    serde_json::from_slice(&fs::read(directory.join(filename)).unwrap()).unwrap()
}

fn filenames(directory: &Path) -> Vec<String> {
    let mut filenames: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    filenames.sort();
    filenames
}

fn temporary_directory() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "masonry-schema-export-{}-{unique}",
        std::process::id()
    ))
}
