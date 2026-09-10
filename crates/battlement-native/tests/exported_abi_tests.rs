use std::{
  env,
  ffi::{CStr, c_char, c_void},
  fs,
  path::PathBuf,
  process::Command as ProcessCommand,
  ptr,
};

use battlement::{Connect, json};
use battlement_native::{BattlementBuffer, ENGINE_ERROR, INVALID_ARGUMENT, OK, PANIC};
use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};

const ACTION_BYTES: &[u8] = br#"{"Action":{"action_id":"11111111-1111-4111-8111-111111111111","session_id":"22222222-2222-4222-8222-222222222222","body":{"PointerEnter":{"object_id":"33333333-3333-4333-8333-333333333333","pointer_id":0,"screen_position":{"x":1.0,"y":2.0},"world_hit":{"x":0.0,"y":0.0,"z":0.0}}}}}"#;

type Create = unsafe extern "C" fn(*mut *mut c_void, *mut BattlementBuffer) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void, *mut BattlementBuffer) -> i32;
type Request = unsafe extern "C" fn(*mut c_void, *const u8, u64, *mut BattlementBuffer) -> i32;
type UiRequest =
  unsafe extern "C" fn(*mut c_void, *const u8, u64, *mut u32, *mut BattlementBuffer) -> i32;
type Poll = unsafe extern "C" fn(*mut c_void, *mut BattlementBuffer) -> i32;
type BufferFree = unsafe extern "C" fn(BattlementBuffer);
type Count = unsafe extern "C" fn() -> usize;
type DeterminismContract = unsafe extern "C" fn() -> u32;
type DeterminismCapabilities = unsafe extern "C" fn() -> u64;
type VoidAction = unsafe extern "C" fn();
type LogAction = unsafe extern "C" fn(*mut BattlementBuffer) -> i32;
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
  let mut connect: Connect = json::from_slice(br#"{"platform":"macOS","unity_version":"6000.5.8f1","screen":{"width":2560,"height":1440},"application_state":{"focused":true,"paused":false},"custom_command_types":["cards.draw","cards.shuffle"],"persistent_data_path":null,"streaming_assets_path":null}"#).unwrap();
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

unsafe fn call_destroy(
  destroy: &Symbol<'_, Destroy>,
  engine: *mut c_void,
  free: &Symbol<'_, BufferFree>,
) -> (i32, Vec<u8>) {
  let mut output = poison_buffer();
  let status = unsafe { destroy(engine, &mut output) };
  if output.length == 0 {
    assert!(output.data.is_null());
    (status, Vec::new())
  } else {
    (status, unsafe { take_buffer(output, free) })
  }
}

#[test]
fn exported_cdylib_contains_the_fixed_panic_safe_abi() {
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
    let free: Symbol<'_, BufferFree> = library.get(b"battlement_buffer_free").unwrap();
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
      battlement_native::WIRE_CONTRACT_DIGEST
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
    free(BattlementBuffer::EMPTY);
    assert_eq!(
      call_destroy(&destroy, ptr::null_mut(), &free),
      (OK, Vec::new())
    );
    assert_eq!(outstanding(), 0);

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

    trace();
    let mut log_output = poison_buffer();
    assert_eq!(logging_drain(&mut log_output), OK);
    let log_text = String::from_utf8(take_buffer(log_output, &free)).unwrap();
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
    assert_eq!(log_output.length, 0);

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
    assert_eq!(call_destroy(&destroy, engine, &free), (OK, Vec::new()));

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-submit", &mut output),
      OK
    );
    let connected: serde_json::Value = serde_json::from_slice(&take_buffer(output, &free)).unwrap();
    let session_id = connected["session_id"].as_str().unwrap();
    let event = serde_json::to_vec(&serde_json::json!({
      "action_id": "11111111-1111-4111-8111-111111111111",
      "session_id": session_id,
      "event": {
        "target_id": "33333333-3333-4333-8333-333333333333",
        "cancelable": true,
        "default_prevented": true,
        "body": { "Click": "NavigationSubmit" }
      }
    }))
    .unwrap();
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
    let response: serde_json::Value = serde_json::from_slice(&take_buffer(output, &free)).unwrap();
    assert_eq!(response["session_id"], session_id);
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
    assert_eq!(call_destroy(&destroy, engine, &free), (OK, Vec::new()));

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
    assert_eq!(call_destroy(&destroy, engine, &free), (OK, Vec::new()));

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);

    output = poison_buffer();
    assert_eq!(
      call_connect(&connect, engine, "panic-destroy", &mut output),
      OK
    );
    take_buffer(output, &free);
    let (status, diagnostic) = call_destroy(&destroy, engine, &free);
    assert_eq!(status, PANIC);
    let plain = self::plain_diagnostic(&String::from_utf8(diagnostic).unwrap());
    assert!(plain.starts_with(
      "Rust panic in battlement_engine_destroy\nMessage:  fixture destroy panic\nLocation:"
    ));

    engine = ptr::null_mut();
    output = poison_buffer();
    assert_eq!(create(&mut engine, &mut output), OK);
    assert!(!engine.is_null());
    assert_eq!(call_destroy(&destroy, engine, &free), (OK, Vec::new()));
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
  let rust = fs::read_to_string(root.join("crates/battlement-native/src/lib.rs")).unwrap();
  let csharp = fs::read_to_string(
    root.join("Packages/com.battlement.client/Runtime/Host/Native/BattlementNativeMethods.cs"),
  )
  .unwrap();
  assert!(
    rust.contains("#[repr(C)]")
      || fs::read_to_string(root.join("crates/battlement-native/src/adapter.rs"))
        .unwrap()
        .contains("#[repr(C)]")
  );
  assert!(csharp.contains("[StructLayout(LayoutKind.Sequential)]"));
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
    (Language::Csharp, "engine_out") => "out IntPtr",
    (Language::Csharp, "engine") => "IntPtr",
    (Language::Csharp, "buffer_out") => "out BattlementNativeBuffer",
    (Language::Csharp, "bytes") => "[In] byte[]",
    (Language::Csharp, "u64") => "ulong",
    (Language::Csharp, "u32_out") => "out uint",
    (Language::Csharp, "buffer") => "BattlementNativeBuffer",
    (Language::Rust, "engine_out") => "*mut *mut ::core::ffi::c_void",
    (Language::Rust, "engine") => "*mut ::core::ffi::c_void",
    (Language::Rust, "buffer_out") => "*mut $crate::BattlementBuffer",
    (Language::Rust, "bytes") => "*const u8",
    (Language::Rust, "u64") => "u64",
    (Language::Rust, "u32_out") => "*mut u32",
    (Language::Rust, "buffer") => "$crate::BattlementBuffer",
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
