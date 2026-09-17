use std::{
  env,
  ffi::{CStr, c_char},
  fs,
  path::PathBuf,
  process::Command as ProcessCommand,
  ptr,
};

use battlement_flatbuffers::{
  ConnectInput, ReducedMotionPreference, ResponseView,
  test_support::{navigation_submit_ui_event, pointer_enter_core_action},
  write_connect,
};
use battlement_native::{ENGINE_ERROR, INVALID_ARGUMENT, OK, PANIC};
use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};

type Create = unsafe extern "C" fn(*mut u64, *mut u64) -> i32;
type Destroy = unsafe extern "C" fn(u64, *mut u64) -> i32;
type Request = unsafe extern "C" fn(u64, *const u8, u64, *mut u64) -> i32;
type UiRequest = unsafe extern "C" fn(u64, *const u8, u64, *mut u32, *mut u64) -> i32;
type Poll = unsafe extern "C" fn(u64, *mut u64) -> i32;
type BufferInfo = unsafe extern "C" fn(u64, *mut *const u8, *mut u64, *mut u64) -> i32;
type ReleaseBuffer = unsafe extern "C" fn(u64) -> i32;
type TransportDiagnostics =
  unsafe extern "C" fn(*mut u64, *mut u64, *mut u64, *mut u64, *mut u64, *mut u64) -> i32;
type Count = unsafe extern "C" fn() -> usize;
type DeterminismContract = unsafe extern "C" fn() -> u32;
type DeterminismCapabilities = unsafe extern "C" fn() -> u64;
type VoidAction = unsafe extern "C" fn();
type LogAction = unsafe extern "C" fn(*mut u64) -> i32;
type ContractDigest = unsafe extern "C" fn() -> *const c_char;

fn fixture_library_path() -> PathBuf {
  let workspace = PathBuf::from(
    env::var_os("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"),
  )
  .join("../..");
  let target = workspace.join("target/export-fixture-tests");
  let status = ProcessCommand::new(env::var_os("CARGO").expect("Cargo provides its executable"))
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
  write_connect(&ConnectInput {
    platform,
    unity_version: "6000.5.8f1",
    screen_width: 2560,
    screen_height: 1440,
    focused: true,
    paused: false,
    reduced_motion_preference: ReducedMotionPreference::Unavailable,
    custom_command_types: &["cards.draw", "cards.shuffle"],
    modules: &[],
    persistent_data_path: None,
    streaming_assets_path: None,
  })
  .unwrap()
  .as_bytes()
  .to_vec()
}

fn core_action_bytes() -> Vec<u8> {
  pointer_enter_core_action([1; 16], [2; 16], [3; 16])
}

fn poison_buffer() -> u64 {
  u64::MAX
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

unsafe fn take_buffer(
  buffer: u64,
  info: &Symbol<'_, BufferInfo>,
  release: &Symbol<'_, ReleaseBuffer>,
) -> Vec<u8> {
  assert_ne!(buffer, 0);
  let mut data = ptr::null();
  let mut length = 0;
  let mut allocation_bytes = 0;
  assert_eq!(
    unsafe { info(buffer, &mut data, &mut length, &mut allocation_bytes) },
    OK
  );
  assert!(!data.is_null());
  assert!(allocation_bytes >= length);
  let bytes =
    unsafe { std::slice::from_raw_parts(data, usize::try_from(length).unwrap()).to_vec() };
  assert_eq!(unsafe { release(buffer) }, OK);
  bytes
}

unsafe fn call_connect(
  connect: &Symbol<'_, Request>,
  engine: u64,
  platform: &str,
  output: &mut u64,
) -> i32 {
  let bytes = connect_bytes(platform);
  unsafe { connect(engine, bytes.as_ptr(), bytes.len() as u64, output) }
}

unsafe fn call_destroy(
  destroy: &Symbol<'_, Destroy>,
  engine: u64,
  info: &Symbol<'_, BufferInfo>,
  release: &Symbol<'_, ReleaseBuffer>,
) -> (i32, Vec<u8>) {
  let mut output = poison_buffer();
  let status = unsafe { destroy(engine, &mut output) };
  if output == 0 {
    (status, Vec::new())
  } else {
    (status, unsafe { take_buffer(output, info, release) })
  }
}

#[test]
fn exported_cdylib_contains_the_fixed_panic_safe_abi() {
  let action_bytes = core_action_bytes();
  let path = fixture_library_path();
  // SAFETY: The test controls the fixture library and every loaded signature.
  let library = unsafe { Library::new(path).unwrap() };
  // SAFETY: The fixture macro defines each symbol with the declared C signature.
  unsafe {
    let create: Symbol<'_, Create> = library.get(b"battlement_engine_create").unwrap();
    let destroy: Symbol<'_, Destroy> = library.get(b"battlement_engine_destroy").unwrap();
    let connect: Symbol<'_, Request> = library.get(b"battlement_connect").unwrap();
    let submit: Symbol<'_, Request> = library.get(b"battlement_submit").unwrap();
    let submit_ui_event: Symbol<'_, UiRequest> =
      library.get(b"battlement_submit_ui_event").unwrap();
    let poll: Symbol<'_, Poll> = library.get(b"battlement_poll").unwrap();
    let info: Symbol<'_, BufferInfo> = library.get(b"battlement_buffer_info").unwrap();
    let release: Symbol<'_, ReleaseBuffer> = library.get(b"battlement_release_buffer").unwrap();
    let transport_diagnostics: Symbol<'_, TransportDiagnostics> =
      library.get(b"battlement_transport_diagnostics").unwrap();
    let outstanding: Symbol<'_, Count> = library.get(b"fixture_outstanding_buffers").unwrap();
    let submit_calls: Symbol<'_, Count> = library.get(b"fixture_submit_calls").unwrap();
    let logging_drain: Symbol<'_, LogAction> = library.get(b"battlement_logging_drain").unwrap();
    let trace: Symbol<'_, VoidAction> = library.get(b"fixture_trace").unwrap();
    let determinism_contract: Symbol<'_, DeterminismContract> = library
      .get(b"battlement_ditto_determinism_contract")
      .unwrap();
    let determinism_capabilities: Symbol<'_, DeterminismCapabilities> = library
      .get(b"battlement_ditto_determinism_capabilities")
      .unwrap();
    let native_abi: Symbol<'_, ContractDigest> =
      library.get(b"battlement_native_abi_digest").unwrap();
    let wire_contract: Symbol<'_, ContractDigest> =
      library.get(b"battlement_wire_contract_digest").unwrap();

    assert!(library.get::<VoidAction>(b"battlement_abi_v1").is_err());
    assert_eq!(determinism_contract(), 3);
    assert_eq!(
      CStr::from_ptr(native_abi()).to_str().unwrap(),
      battlement_native::NATIVE_ABI_DIGEST
    );
    assert_eq!(
      CStr::from_ptr(wire_contract()).to_str().unwrap(),
      "108b684df07c19a85bb337a37375d4d6427c4ecdb4038104cebc93283f2170d7"
    );
    assert_eq!(
      determinism_capabilities(),
      battlement_native::DITTO_DETERMINISM_CAPABILITIES_V3
    );
    assert!(
      library
        .get::<LogAction>(b"battlement_logging_initialize")
        .is_err()
    );
    assert_eq!(release(0), INVALID_ARGUMENT);
    let mut created = u64::MAX;
    let mut reused = u64::MAX;
    let mut growths = u64::MAX;
    let mut copied_bytes = u64::MAX;
    let mut idle_bytes = u64::MAX;
    let mut handoff_copies = u64::MAX;
    assert_eq!(
      transport_diagnostics(
        &mut created,
        &mut reused,
        &mut growths,
        &mut copied_bytes,
        &mut idle_bytes,
        &mut handoff_copies,
      ),
      OK
    );
    assert_eq!(handoff_copies, 0);
    assert_eq!(
      transport_diagnostics(
        ptr::null_mut(),
        &mut reused,
        &mut growths,
        &mut copied_bytes,
        &mut idle_bytes,
        &mut handoff_copies,
      ),
      INVALID_ARGUMENT
    );
    assert_eq!(reused, 0);
    assert_eq!(growths, 0);
    assert_eq!(copied_bytes, 0);
    assert_eq!(idle_bytes, 0);
    assert_eq!(handoff_copies, 0);
    assert_eq!(
      call_destroy(&destroy, 0, &info, &release).0,
      INVALID_ARGUMENT
    );
    assert_eq!(outstanding(), 0);

    std::env::set_var("BATTLEMENT_EXPORT_FIXTURE_CREATE", "panic");
    let mut engine = u64::MAX;
    let mut output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), PANIC);
    assert_eq!(engine, 0);
    let diagnostic = String::from_utf8(take_buffer(output, &info, &release)).unwrap();
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

    trace();
    let mut log_output = poison_buffer();
    assert_eq!(logging_drain(&mut log_output), OK);
    let log_text = String::from_utf8(take_buffer(log_output, &info, &release)).unwrap();
    let records = log_text
      .lines()
      .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
      .collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    assert!(records[0].get("source").is_none());
    assert!(records[0].get("sequence").is_none());
    assert_eq!(records[0]["event_name"], "fixture.rust_event");
    assert_eq!(records[0]["fields"]["mode"], "test");

    log_output = poison_buffer();
    assert_eq!(logging_drain(&mut log_output), OK);
    assert_eq!(log_output, 0);

    std::env::set_var("BATTLEMENT_EXPORT_FIXTURE_CREATE", "error");
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), ENGINE_ERROR);
    assert_eq!(engine, 0);
    assert_eq!(
      String::from_utf8(take_buffer(output, &info, &release)).unwrap(),
      "fixture create error"
    );

    std::env::remove_var("BATTLEMENT_EXPORT_FIXTURE_CREATE");
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);
    assert_ne!(engine, 0);
    assert_eq!(output, 0);

    output = poison_buffer();
    assert_eq!(
      connect(engine, [0xc1].as_ptr(), 1, &mut output),
      INVALID_ARGUMENT
    );
    assert!(
      String::from_utf8(take_buffer(output, &info, &release))
        .unwrap()
        .contains("invalid connect")
    );

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-connect", &mut output),
      PANIC
    );
    let diagnostic = String::from_utf8(take_buffer(output, &info, &release)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain.starts_with(
        "Rust panic in battlement_connect\nMessage:  fixture connect panic\nLocation:"
      )
    );

    output = poison_buffer();
    assert_eq!(call_connect(&connect, engine, "normal", &mut output), PANIC);
    assert_eq!(
      String::from_utf8(take_buffer(output, &info, &release)).unwrap(),
      "Rust engine is poisoned after an earlier panic"
    );
    let stale_engine = engine;
    assert_eq!(
      call_destroy(&destroy, engine, &info, &release),
      (OK, Vec::new())
    );
    assert_eq!(
      call_destroy(&destroy, stale_engine, &info, &release).0,
      INVALID_ARGUMENT
    );

    engine = 0;
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-submit", &mut output),
      OK
    );
    let connected_bytes = take_buffer(output, &info, &release);
    let connected = ResponseView::read(&connected_bytes).unwrap();
    let session_id = uuid::Uuid::from_bytes(connected.session_id());
    let event = navigation_submit_ui_event(
      *uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111")
        .unwrap()
        .as_bytes(),
      *session_id.as_bytes(),
      *uuid::Uuid::parse_str("33333333-3333-4333-8333-333333333333")
        .unwrap()
        .as_bytes(),
      true,
      true,
    );
    let mut disposition = u32::MAX;
    output = poison_buffer();
    assert_eq!(
      submit_ui_event(
        engine,
        event.as_ptr(),
        event.len() as u64,
        &mut disposition,
        &mut output,
      ),
      OK
    );
    assert_eq!(disposition, 1);
    let response_bytes = take_buffer(output, &info, &release);
    let response = ResponseView::read(&response_bytes).unwrap();
    assert_eq!(response.session_id(), *session_id.as_bytes());
    let calls_before = submit_calls();
    output = poison_buffer();
    assert_eq!(
      submit(
        engine,
        action_bytes.as_ptr(),
        action_bytes.len() as u64,
        &mut output
      ),
      PANIC
    );
    assert_eq!(submit_calls(), calls_before + 1);
    let diagnostic = String::from_utf8(take_buffer(output, &info, &release)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain
        .starts_with("Rust panic in battlement_submit\nMessage:  fixture submit panic\nLocation:")
    );
    assert_eq!(
      call_destroy(&destroy, engine, &info, &release),
      (OK, Vec::new())
    );

    engine = 0;
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "direct-native-label", &mut output),
      OK
    );
    let connected_bytes = take_buffer(output, &info, &release);
    let connected = ResponseView::read(&connected_bytes).unwrap();
    let direct_session = connected.session_id();
    let direct_action = pointer_enter_core_action([0x31; 16], direct_session, [0x32; 16]);
    output = poison_buffer();
    assert_eq!(
      submit(
        engine,
        direct_action.as_ptr(),
        direct_action.len() as u64,
        &mut output,
      ),
      OK
    );
    let direct_bytes = take_buffer(output, &info, &release);
    let direct_response = ResponseView::read(&direct_bytes).unwrap();
    assert_eq!(direct_response.session_id(), direct_session);
    assert_eq!(direct_response.message_count(), 1);
    assert_eq!(
      call_destroy(&destroy, engine, &info, &release),
      (OK, Vec::new())
    );

    engine = 0;
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-poll", &mut output),
      OK
    );
    take_buffer(output, &info, &release);
    output = poison_buffer();
    assert_eq!(poll(engine, &mut output), PANIC);
    let diagnostic = String::from_utf8(take_buffer(output, &info, &release)).unwrap();
    let plain = self::plain_diagnostic(&diagnostic);
    assert!(
      plain.starts_with("Rust panic in battlement_poll\nMessage:  fixture poll panic\nLocation:")
    );
    assert_eq!(
      call_destroy(&destroy, engine, &info, &release),
      (OK, Vec::new())
    );

    engine = 0;
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-destroy", &mut output),
      OK
    );
    take_buffer(output, &info, &release);
    let (status, diagnostic) = call_destroy(&destroy, engine, &info, &release);
    assert_eq!(status, PANIC);
    let plain = self::plain_diagnostic(&String::from_utf8(diagnostic).unwrap());
    assert!(plain.starts_with(
      "Rust panic in battlement_engine_destroy\nMessage:  fixture destroy panic\nLocation:"
    ));

    engine = 0;
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);
    assert_ne!(engine, 0);
    assert_eq!(
      call_destroy(&destroy, engine, &info, &release),
      (OK, Vec::new())
    );
    assert_eq!(outstanding(), 0);
  }
}

#[test]
fn checked_in_contract_manifests_match_exported_digests() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
  for (path, expected) in [
    (
      "contracts/native-abi.json",
      battlement_native::NATIVE_ABI_DIGEST,
    ),
    (
      "contracts/wire-contract.json",
      battlement_native::WIRE_CONTRACT_DIGEST,
    ),
  ] {
    let actual = format!("{:x}", Sha256::digest(fs::read(root.join(path)).unwrap()));
    assert_eq!(actual, expected, "{path} digest changed");
  }
  let shell = fs::read_to_string(
    root.join("Packages/com.battlement.client/Runtime/Host/Native/BattlementNativeContract.cs"),
  )
  .unwrap();
  assert!(shell.contains(battlement_native::NATIVE_ABI_DIGEST));
  assert!(shell.contains(battlement_native::WIRE_CONTRACT_DIGEST));

  let manifest: serde_json::Value =
    serde_json::from_slice(&fs::read(root.join("contracts/native-abi.json")).unwrap()).unwrap();
  let wire_manifest: serde_json::Value =
    serde_json::from_slice(&fs::read(root.join("contracts/wire-contract.json")).unwrap()).unwrap();
  let schema_closure = wire_manifest["flatbuffers"]["schema_closure"]
    .as_object()
    .expect("wire contract has a schema closure");
  let mut schema_names = fs::read_dir(root.join("schemas/flatbuffers"))
    .unwrap()
    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
    .filter(|name| name.ends_with(".fbs"))
    .collect::<Vec<_>>();
  schema_names.sort();
  let mut declared_names = schema_closure.keys().cloned().collect::<Vec<_>>();
  declared_names.sort();
  assert_eq!(declared_names, schema_names);
  for name in schema_names {
    let actual = format!(
      "{:x}",
      Sha256::digest(fs::read(root.join("schemas/flatbuffers").join(&name)).unwrap())
    );
    assert_eq!(
      schema_closure[&name].as_str(),
      Some(actual.as_str()),
      "{name}"
    );
  }
  for operation in ["connect", "submit", "submit_ui_event", "poll"] {
    assert_eq!(
      wire_manifest["operations"][operation]["response_encoding"].as_str(),
      Some("size-prefixed-flatbuffers")
    );
  }
  let fixture_manifest_path =
    root.join("crates/battlement-native/tests/fixtures/exported-engine/schema/wire-contract.json");
  let fixture_manifest_bytes = fs::read(&fixture_manifest_path).unwrap();
  let fixture_digest = format!("{:x}", Sha256::digest(&fixture_manifest_bytes));
  assert_eq!(
    fixture_digest,
    "108b684df07c19a85bb337a37375d4d6427c4ecdb4038104cebc93283f2170d7"
  );
  let fixture_manifest: serde_json::Value =
    serde_json::from_slice(&fixture_manifest_bytes).unwrap();
  assert_eq!(
    fixture_manifest["base_wire_contract_digest"].as_str(),
    Some(battlement_native::WIRE_CONTRACT_DIGEST)
  );
  let fixture_schema = root
    .join("crates/battlement-native/tests/fixtures/exported-engine/schema/fixture_response.fbs");
  let fixture_schema_digest = format!("{:x}", Sha256::digest(fs::read(fixture_schema).unwrap()));
  assert_eq!(
    fixture_manifest["schema_closure"]["fixture_response.fbs"].as_str(),
    Some(fixture_schema_digest.as_str())
  );
  let fixture_csharp = fs::read_to_string(
    root.join("Assets/BattlementIntegration/FlatBuffers/FixtureFlatBufferResponseSchema.cs"),
  )
  .unwrap();
  assert!(fixture_csharp.contains(&fixture_digest));
  let rust = fs::read_to_string(root.join("crates/battlement-native/src/lib.rs")).unwrap();
  let csharp = fs::read_to_string(
    root.join("Packages/com.battlement.client/Runtime/Host/Native/BattlementNativeMethods.cs"),
  )
  .unwrap();
  for function in manifest["functions"].as_array().unwrap() {
    let name = function["name"].as_str().unwrap();
    let return_type = function["return"].as_str().unwrap();
    let parameters = function["parameters"]
      .as_array()
      .unwrap()
      .iter()
      .map(|value| value.as_str().unwrap())
      .collect::<Vec<_>>();
    assert_signature(&csharp, name, return_type, &parameters, Language::Csharp);
    assert_signature(&rust, name, return_type, &parameters, Language::Rust);
  }
}

#[derive(Clone, Copy)]
enum Language {
  Csharp,
  Rust,
}

fn assert_signature(
  source: &str,
  name: &str,
  return_type: &str,
  parameters: &[&str],
  language: Language,
) {
  let source = source.split_whitespace().collect::<Vec<_>>().join(" ");
  let marker = match language {
    Language::Csharp => format!(" {name}("),
    Language::Rust => format!(" fn {name}("),
  };
  let marker_start = source
    .find(&marker)
    .unwrap_or_else(|| panic!("missing {name}"));
  let parameters_start = marker_start + marker.len();
  let parameters_end = source[parameters_start..]
    .find(')')
    .map(|offset| parameters_start + offset)
    .unwrap();
  let actual_parameters = source[parameters_start..parameters_end]
    .split(',')
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(|value| match language {
      Language::Csharp => value.rsplit_once(' ').unwrap().0,
      Language::Rust => value.split_once(':').unwrap().1.trim(),
    })
    .collect::<Vec<_>>();
  let expected_parameters = parameters
    .iter()
    .map(|value| parameter_type(value, language))
    .collect::<Vec<_>>();
  assert_eq!(actual_parameters, expected_parameters, "{name} parameters");

  let actual_return = match language {
    Language::Csharp => source[..marker_start].rsplit_once(' ').unwrap().1,
    Language::Rust => source[parameters_end + 1..]
      .trim_start()
      .strip_prefix("-> ")
      .and_then(|value| value.split_once(' '))
      .map_or("()", |(value, _)| value),
  };
  assert_eq!(
    actual_return,
    return_type_name(return_type, language),
    "{name} return"
  );
}

fn parameter_type<'a>(value: &str, language: Language) -> &'a str {
  match (language, value) {
    (Language::Csharp, "engine_out") => "out ulong",
    (Language::Csharp, "engine") => "ulong",
    (Language::Csharp, "buffer_out") => "out ulong",
    (Language::Csharp, "bytes") => "IntPtr",
    (Language::Csharp, "u64") => "ulong",
    (Language::Csharp, "u64_out") => "out ulong",
    (Language::Csharp, "u32_out") => "out uint",
    (Language::Csharp, "buffer") => "ulong",
    (Language::Csharp, "const_bytes_out") => "out IntPtr",
    (Language::Rust, "engine_out") => "*mut $crate::EngineHandle",
    (Language::Rust, "engine") => "$crate::EngineHandle",
    (Language::Rust, "buffer_out") => "*mut $crate::BufferHandle",
    (Language::Rust, "bytes") => "*const u8",
    (Language::Rust, "u64") => "u64",
    (Language::Rust, "u64_out") => "*mut u64",
    (Language::Rust, "u32_out") => "*mut u32",
    (Language::Rust, "buffer") => "$crate::BufferHandle",
    (Language::Rust, "const_bytes_out") => "*mut *const u8",
    _ => panic!("unknown ABI parameter type {value}"),
  }
}

fn return_type_name(value: &str, language: Language) -> &str {
  match (language, value) {
    (Language::Csharp, "cstring") => "IntPtr",
    (Language::Csharp, "i32") => "int",
    (Language::Csharp, "void") => "void",
    (Language::Csharp, "u32") => "uint",
    (Language::Csharp, "u64") => "ulong",
    (Language::Rust, "cstring") => "*const",
    (Language::Rust, "i32") => "i32",
    (Language::Rust, "void") => "()",
    (Language::Rust, "u32") => "u32",
    (Language::Rust, "u64") => "u64",
    _ => panic!("unknown ABI return type {value}"),
  }
}
