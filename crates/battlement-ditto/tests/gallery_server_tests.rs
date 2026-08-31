use std::{
  collections::BTreeMap,
  fs,
  io::{Read, Write},
  net::{Shutdown, TcpListener, TcpStream},
  process::{Command as ProcessCommand, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use battlement_ditto::gallery_server::{GalleryCheckpoint, GalleryDocument, GalleryServer};

#[test]
fn gallery_serves_source_metadata_and_only_allow_listed_images() {
  let temporary = tempfile::tempdir().unwrap();
  let image = temporary.path().join("baseline.png");
  fs::write(&image, b"canonical-png").unwrap();
  let mut images = BTreeMap::new();
  images.insert("/image/abc.png".to_owned(), image);
  let server = GalleryServer::bind(
    GalleryDocument {
      suite: "sample".to_owned(),
      profile: "macos".to_owned(),
      filename: "ditto.toml".to_owned(),
      source: "name = \"sample\"\n".to_owned(),
      checkpoints: vec![GalleryCheckpoint {
        after_line: 1,
        scenario: "opening".to_owned(),
        checkpoint: "board".to_owned(),
        image: Some("/image/abc.png".to_owned()),
        width: Some(1280),
        height: Some(720),
      }],
    },
    images,
    None,
  )
  .unwrap();
  let base = server.url();
  let interrupted = Arc::new(AtomicBool::new(false));
  let worker_interrupt = Arc::clone(&interrupted);
  let worker = thread::spawn(move || server.serve(&worker_interrupt).unwrap());

  let page = exchange("GET", &base, "/");
  assert_eq!(page.status, 200);
  assert!(page.headers.contains("Content-Security-Policy:"));
  assert!(page.body.contains("Ditto Gallery"));
  let document = exchange("GET", &base, "/api/gallery");
  assert_eq!(document.status, 200);
  assert!(document.body.contains("name = \\\"sample\\\""));
  assert!(document.body.contains("/image/abc.png"));
  assert_eq!(
    exchange("GET", &base, "/image/abc.png").body,
    "canonical-png"
  );
  assert_eq!(exchange("GET", &base, "/image/missing.png").status, 404);
  assert_eq!(exchange("POST", &base, "/api/gallery").status, 405);

  interrupted.store(true, Ordering::Release);
  worker.join().unwrap();
}

#[test]
fn gallery_command_opens_without_a_baseline_store_or_lock() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::write(repository.join("Assets/Scenes/Game.unity"), "").unwrap();
  fs::write(
    repository.join("rules/Cargo.toml"),
    "[package]\nname='gallery-fixture'\nversion='0.1.0'\n",
  )
  .unwrap();
  fs::write(repository.join("ditto.toml"), SUITE_WITHOUT_BASELINES).unwrap();
  assert!(
    ProcessCommand::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let port = listener.local_addr().unwrap().port();
  drop(listener);
  let mut child = ProcessCommand::new(env!("CARGO_BIN_EXE_ditto"))
    .args(["gallery", "--port", &port.to_string(), "--no-open"])
    .current_dir(&repository)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .unwrap();
  let authority = format!("127.0.0.1:{port}");
  let deadline = Instant::now() + Duration::from_secs(5);
  while TcpStream::connect(&authority).is_err() {
    assert!(Instant::now() < deadline, "gallery did not start");
    thread::sleep(Duration::from_millis(25));
  }
  let gallery = exchange("GET", &format!("http://{authority}/"), "/api/gallery");
  assert_eq!(gallery.status, 200);
  assert!(gallery.body.contains(r#""checkpoint":"board""#));
  assert!(gallery.body.contains(r#""image":null"#));
  child.kill().unwrap();
  child.wait().unwrap();
}

struct HttpResponse {
  status: u16,
  headers: String,
  body: String,
}

fn exchange(method: &str, base: &str, path: &str) -> HttpResponse {
  let authority = base.strip_prefix("http://").unwrap().trim_end_matches('/');
  let mut stream = TcpStream::connect(authority).unwrap();
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .unwrap();
  write!(
    stream,
    "{method} {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
  )
  .unwrap();
  stream.shutdown(Shutdown::Write).unwrap();
  let mut bytes = Vec::new();
  stream.read_to_end(&mut bytes).unwrap();
  let response = String::from_utf8(bytes).unwrap();
  let (headers, body) = response.split_once("\r\n\r\n").unwrap();
  let status = headers
    .lines()
    .next()
    .unwrap()
    .split_whitespace()
    .nth(1)
    .unwrap()
    .parse()
    .unwrap();
  HttpResponse {
    status,
    headers: headers.to_owned(),
    body: body.to_owned(),
  }
}

const SUITE_WITHOUT_BASELINES: &str = r#"
name = "gallery"
default_profile = "macos"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[profiles.macos]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "opening"

[[scenarios.steps]]
screenshot = { name = "board" }
"#;
