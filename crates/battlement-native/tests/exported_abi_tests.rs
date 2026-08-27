use std::{
  ffi::c_void,
  fs,
  io::Write as _,
  path::{Path, PathBuf},
  process::Command as ProcessCommand,
  ptr,
};

use battlement::{Connect, json};
use battlement_native::{BattlementBuffer, ENGINE_ERROR, INVALID_ARGUMENT, OK, PANIC};
use libloading::{Library, Symbol};

const ACTION_BYTES: &[u8] = br#"{"Action":{"action_id":"11111111-1111-4111-8111-111111111111","session_id":"22222222-2222-4222-8222-222222222222","body":{"PointerEnter":{"object_id":"33333333-3333-4333-8333-333333333333","pointer_id":0,"screen_position":{"x":1.0,"y":2.0},"world_hit":{"x":0.0,"y":0.0,"z":0.0}}}}}"#;

type Create = unsafe extern "C" fn(*mut *mut c_void, *mut BattlementBuffer) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Request = unsafe extern "C" fn(*mut c_void, *const u8, u64, *mut BattlementBuffer) -> i32;
type Poll = unsafe extern "C" fn(*mut c_void, *mut BattlementBuffer) -> i32;
type BufferFree = unsafe extern "C" fn(BattlementBuffer);
type Count = unsafe extern "C" fn() -> usize;
type VoidAction = unsafe extern "C" fn();
type LogInput = unsafe extern "C" fn(*const u8, u64, *mut BattlementBuffer) -> i32;
type LogRead = unsafe extern "C" fn(u64, u64, *mut BattlementBuffer, *mut u64) -> i32;
type LogAction = unsafe extern "C" fn(*mut BattlementBuffer) -> i32;

fn fixture_library_path() -> PathBuf {
  let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let target = workspace.join("target/export-fixture-tests");
  let status = ProcessCommand::new(env!("CARGO"))
    .args([
      "build",
      "--quiet",
      "-p",
      "battlement-native-export-fixture",
      "--target-dir",
    ])
    .arg(&target)
    .current_dir(workspace)
    .status()
    .expect("fixture cdylib should compile");
  assert!(status.success(), "fixture cdylib build failed");

  target.join("debug").join(format!(
    "{}battlement_rules{}",
    std::env::consts::DLL_PREFIX,
    std::env::consts::DLL_SUFFIX
  ))
}

fn connect_bytes(platform: &str) -> Vec<u8> {
  let mut connect: Connect = json::from_slice(br#"{"platform":"macOS","unity_version":"6000.5.8f1","screen":{"width":2560,"height":1440},"custom_command_types":["cards.draw","cards.shuffle"],"persistent_data_path":null,"streaming_assets_path":null}"#).unwrap();
  connect.platform = platform.to_owned();
  json::to_vec(&connect).unwrap()
}

fn poison_buffer() -> BattlementBuffer {
  BattlementBuffer {
    data: ptr::dangling_mut::<u8>(),
    length: u64::MAX,
  }
}

fn plain_diagnostic(value: &str) -> String {
  let bytes = value.as_bytes();
  let mut plain = Vec::with_capacity(bytes.len());
  let mut index = 0;
  while index < bytes.len() {
    if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'[') {
      index += 2;
      while index < bytes.len() && bytes[index] != b'm' {
        index += 1;
      }
      index += usize::from(index < bytes.len());
      continue;
    }

    plain.push(bytes[index]);
    index += 1;
  }
  String::from_utf8(plain).unwrap()
}

unsafe fn take_buffer(buffer: BattlementBuffer, free: &Symbol<'_, BufferFree>) -> Vec<u8> {
  assert!(!buffer.data.is_null());
  let bytes = unsafe {
    std::slice::from_raw_parts(buffer.data, usize::try_from(buffer.length).unwrap()).to_vec()
  };
  unsafe { free(buffer) };
  bytes
}

unsafe fn call_connect(
  connect: &Symbol<'_, Request>,
  engine: *mut c_void,
  platform: &str,
  output: &mut BattlementBuffer,
) -> i32 {
  let bytes = connect_bytes(platform);
  unsafe { connect(engine, bytes.as_ptr(), bytes.len() as u64, output) }
}

#[test]
fn exported_cdylib_contains_the_fixed_panic_safe_abi() {
  let path = fixture_library_path();
  // SAFETY: The test controls the fixture library and every loaded signature.
  let library = unsafe { Library::new(path).unwrap() };
  // SAFETY: The fixture macro defines each symbol with the declared C signature.
  unsafe {
    let abi: Symbol<'_, VoidAction> = library.get(b"battlement_abi_v1").unwrap();
    let create: Symbol<'_, Create> = library.get(b"battlement_engine_create").unwrap();
    let destroy: Symbol<'_, Destroy> = library.get(b"battlement_engine_destroy").unwrap();
    let connect: Symbol<'_, Request> = library.get(b"battlement_connect").unwrap();
    let submit: Symbol<'_, Request> = library.get(b"battlement_submit").unwrap();
    let poll: Symbol<'_, Poll> = library.get(b"battlement_poll").unwrap();
    let free: Symbol<'_, BufferFree> = library.get(b"battlement_buffer_free").unwrap();
    let outstanding: Symbol<'_, Count> = library.get(b"fixture_outstanding_buffers").unwrap();
    let submit_calls: Symbol<'_, Count> = library.get(b"fixture_submit_calls").unwrap();
    let log_initialize: Symbol<'_, LogInput> = library.get(b"battlement_log_initialize").unwrap();
    let log_write: Symbol<'_, LogInput> = library.get(b"battlement_log_write").unwrap();
    let log_read: Symbol<'_, LogRead> = library.get(b"battlement_log_read").unwrap();
    let log_sync: Symbol<'_, LogAction> = library.get(b"battlement_log_sync").unwrap();
    let log_close: Symbol<'_, LogAction> = library.get(b"battlement_log_close").unwrap();
    let trace: Symbol<'_, VoidAction> = library.get(b"fixture_trace").unwrap();

    abi();
    free(BattlementBuffer::EMPTY);
    destroy(ptr::null_mut());
    assert_eq!(outstanding(), 0);

    let log_directory = std::env::temp_dir().join(format!(
      "battlement-native-log-{}-{}",
      std::process::id(),
      std::thread::current().name().unwrap_or("abi")
    ));
    let _ = fs::remove_dir_all(&log_directory);
    fs::create_dir_all(&log_directory).unwrap();
    fs::write(
      log_directory.join("battlement.jsonl"),
      vec![b'x'; 8 * 1024 * 1024],
    )
    .unwrap();
    let path = log_directory.to_string_lossy();
    let mut log_output = poison_buffer();
    assert_eq!(
      log_initialize(path.as_bytes().as_ptr(), path.len() as u64, &mut log_output),
      OK
    );
    assert_eq!(log_output.length, 0);
    assert!(log_directory.join("battlement.jsonl.1").is_file());
    let unity_record = br#"{"severity":"information","event_name":"fixture.started","message":"ready","fields":{"mode":"test"}}"#;
    log_output = poison_buffer();
    assert_eq!(
      log_write(
        unity_record.as_ptr(),
        unity_record.len() as u64,
        &mut log_output
      ),
      OK
    );
    assert_eq!(log_output.length, 0);
    trace();

    std::env::set_var("BATTLEMENT_EXPORT_FIXTURE_CREATE", "panic");
    let mut engine = ptr::dangling_mut::<c_void>();
    let mut output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), PANIC);
    assert!(engine.is_null());
    let diagnostic = String::from_utf8(take_buffer(output, &free)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(diagnostic.contains('\u{1b}'));
    assert!(plain.starts_with(
      "Rust panic in battlement_engine_create\nMessage:  fixture create panic\nLocation:"
    ));
    assert!(plain.contains(" BACKTRACE "));
    assert!(
      plain
        .lines()
        .any(|line| { line.contains("battlement_rules") && line.ends_with("::create_engine") }),
      "{plain}"
    );
    assert!(!plain.contains("battlement_native::panic_capture"));
    assert!(!plain.contains("core::panicking"));
    assert_eq!(outstanding(), 0);

    let mut next_offset = u64::MAX;
    log_output = poison_buffer();
    assert_eq!(
      log_read(0, 1024 * 1024, &mut log_output, &mut next_offset),
      OK
    );
    let log_text = String::from_utf8(take_buffer(log_output, &free)).unwrap();
    let records = log_text
      .lines()
      .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
      .collect::<Vec<_>>();
    assert_eq!(records[0]["source"], "unity");
    assert_eq!(records[0]["sequence"], 1);
    assert_eq!(records[1]["source"], "rust");
    assert_eq!(records[1]["sequence"], 2);
    assert_eq!(records[1]["event_name"], "fixture.rust_event");
    assert_eq!(records[2]["event_name"], "battlement.rust.panic");
    assert_eq!(records[2]["sequence"], 3);
    assert!(next_offset > 0);

    let mut active = fs::OpenOptions::new()
      .append(true)
      .open(log_directory.join("battlement.jsonl"))
      .unwrap();
    active.write_all(br#"{"incomplete":true}"#).unwrap();
    log_output = poison_buffer();
    let incomplete_offset = next_offset;
    assert_eq!(
      log_read(incomplete_offset, 1024, &mut log_output, &mut next_offset),
      OK
    );
    assert_eq!(log_output.length, 0);
    assert_eq!(next_offset, incomplete_offset);
    active.write_all(b"\n").unwrap();
    log_output = poison_buffer();
    assert_eq!(
      log_read(incomplete_offset, 1024, &mut log_output, &mut next_offset),
      OK
    );
    assert_eq!(
      String::from_utf8(take_buffer(log_output, &free)).unwrap(),
      "{\"incomplete\":true}\n"
    );

    std::env::set_var("BATTLEMENT_EXPORT_FIXTURE_CREATE", "error");
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), ENGINE_ERROR);
    assert!(engine.is_null());
    assert_eq!(
      String::from_utf8(take_buffer(output, &free)).unwrap(),
      "fixture create error"
    );

    std::env::remove_var("BATTLEMENT_EXPORT_FIXTURE_CREATE");
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);
    assert!(!engine.is_null());
    assert!(output.data.is_null());
    assert_eq!(output.length, 0);

    output = poison_buffer();
    assert_eq!(
      connect(engine, [0xc1].as_ptr(), 1, &mut output),
      INVALID_ARGUMENT
    );
    assert!(
      String::from_utf8(take_buffer(output, &free))
        .unwrap()
        .contains("invalid connect JSON")
    );

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-connect", &mut output),
      PANIC
    );
    let diagnostic = String::from_utf8(take_buffer(output, &free)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain.starts_with(
        "Rust panic in battlement_connect\nMessage:  fixture connect panic\nLocation:"
      )
    );

    output = poison_buffer();
    assert_eq!(call_connect(&connect, engine, "normal", &mut output), PANIC);
    assert_eq!(
      String::from_utf8(take_buffer(output, &free)).unwrap(),
      "Rust engine is poisoned after an earlier panic"
    );
    destroy(engine);

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-submit", &mut output),
      OK
    );
    take_buffer(output, &free);
    let calls_before = submit_calls();
    output = poison_buffer();
    assert_eq!(
      submit(
        engine,
        ACTION_BYTES.as_ptr(),
        ACTION_BYTES.len() as u64,
        &mut output
      ),
      PANIC
    );
    assert_eq!(submit_calls(), calls_before + 1);
    let diagnostic = String::from_utf8(take_buffer(output, &free)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain
        .starts_with("Rust panic in battlement_submit\nMessage:  fixture submit panic\nLocation:")
    );
    destroy(engine);

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-poll", &mut output),
      OK
    );
    take_buffer(output, &free);
    output = poison_buffer();
    assert_eq!(poll(engine, &mut output), PANIC);
    let diagnostic = String::from_utf8(take_buffer(output, &free)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain.starts_with("Rust panic in battlement_poll\nMessage:  fixture poll panic\nLocation:")
    );
    destroy(engine);

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-destroy", &mut output),
      OK
    );
    take_buffer(output, &free);
    destroy(engine);

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);
    assert!(!engine.is_null());
    destroy(engine);
    assert_eq!(outstanding(), 0);

    log_output = poison_buffer();
    assert_eq!(log_sync(&mut log_output), OK);
    assert_eq!(log_output.length, 0);
    log_output = poison_buffer();
    assert_eq!(log_close(&mut log_output), OK);
    assert_eq!(log_output.length, 0);
    fs::remove_dir_all(log_directory).unwrap();
  }
}
