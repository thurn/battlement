use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    ptr,
};

use libloading::{Library, Symbol};
use masonry::{Connect, messagepack};
use masonry_native::{ENGINE_ERROR, INVALID_ARGUMENT, MasonryBuffer, NO_MESSAGE, OK, PANIC};

const ACTION_BYTES: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-enter.msgpack"
);

type Create = unsafe extern "C" fn(*mut *mut c_void, *mut MasonryBuffer) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Request = unsafe extern "C" fn(*mut c_void, *const u8, u64, *mut MasonryBuffer) -> i32;
type Poll = unsafe extern "C" fn(*mut c_void, *mut MasonryBuffer) -> i32;
type BufferFree = unsafe extern "C" fn(MasonryBuffer);
type Count = unsafe extern "C" fn() -> usize;
type AbiMarker = unsafe extern "C" fn();

fn fixture_library_path() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = workspace.join("target/export-fixture-tests");
    let status = ProcessCommand::new(env!("CARGO"))
        .args([
            "build",
            "--quiet",
            "-p",
            "masonry-native-export-fixture",
            "--target-dir",
        ])
        .arg(&target)
        .current_dir(workspace)
        .status()
        .expect("fixture cdylib should compile");
    assert!(status.success(), "fixture cdylib build failed");

    target.join("debug").join(format!(
        "{}masonry_rules{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

fn connect_bytes(platform: &str) -> Vec<u8> {
    let fixture = include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-connect.msgpack"
    );
    let mut connect: Connect = messagepack::from_slice(fixture).unwrap();
    connect.platform = platform.to_owned();
    messagepack::to_vec(&connect).unwrap()
}

fn poison_buffer() -> MasonryBuffer {
    MasonryBuffer {
        data: ptr::dangling_mut::<u8>(),
        length: u64::MAX,
    }
}

unsafe fn take_buffer(buffer: MasonryBuffer, free: &Symbol<'_, BufferFree>) -> Vec<u8> {
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
    output: &mut MasonryBuffer,
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
        let abi: Symbol<'_, AbiMarker> = library.get(b"masonry_abi_v1").unwrap();
        let create: Symbol<'_, Create> = library.get(b"masonry_engine_create").unwrap();
        let destroy: Symbol<'_, Destroy> = library.get(b"masonry_engine_destroy").unwrap();
        let connect: Symbol<'_, Request> = library.get(b"masonry_connect").unwrap();
        let submit: Symbol<'_, Request> = library.get(b"masonry_submit").unwrap();
        let poll: Symbol<'_, Poll> = library.get(b"masonry_poll").unwrap();
        let free: Symbol<'_, BufferFree> = library.get(b"masonry_buffer_free").unwrap();
        let outstanding: Symbol<'_, Count> = library.get(b"fixture_outstanding_buffers").unwrap();
        let submit_calls: Symbol<'_, Count> = library.get(b"fixture_submit_calls").unwrap();

        abi();
        free(MasonryBuffer::EMPTY);
        destroy(ptr::null_mut());
        assert_eq!(outstanding(), 0);

        std::env::set_var("MASONRY_EXPORT_FIXTURE_CREATE", "panic");
        let mut engine = ptr::dangling_mut::<c_void>();
        let mut output = poison_buffer();
        assert_eq!(create(&mut engine, &mut output), PANIC);
        assert!(engine.is_null());
        assert_eq!(
            String::from_utf8(take_buffer(output, &free)).unwrap(),
            "Rust panic in masonry_engine_create"
        );
        assert_eq!(outstanding(), 0);

        std::env::set_var("MASONRY_EXPORT_FIXTURE_CREATE", "error");
        output = poison_buffer();
        assert_eq!(create(&mut engine, &mut output), ENGINE_ERROR);
        assert!(engine.is_null());
        assert_eq!(
            String::from_utf8(take_buffer(output, &free)).unwrap(),
            "fixture create error"
        );

        std::env::remove_var("MASONRY_EXPORT_FIXTURE_CREATE");
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
                .contains("invalid connect MessagePack")
        );

        output = poison_buffer();
        assert_eq!(
            call_connect(&connect, engine, "panic-connect", &mut output),
            PANIC
        );
        assert_eq!(
            String::from_utf8(take_buffer(output, &free)).unwrap(),
            "Rust panic in masonry_connect"
        );

        output = poison_buffer();
        assert_eq!(call_connect(&connect, engine, "normal", &mut output), OK);
        take_buffer(output, &free);
        output = poison_buffer();
        assert_eq!(
            submit(
                engine,
                ACTION_BYTES.as_ptr(),
                ACTION_BYTES.len() as u64,
                &mut output
            ),
            OK
        );
        take_buffer(output, &free);
        output = poison_buffer();
        assert_eq!(poll(engine, &mut output), NO_MESSAGE);
        assert!(output.data.is_null());
        assert_eq!(output.length, 0);

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
        assert_eq!(
            String::from_utf8(take_buffer(output, &free)).unwrap(),
            "Rust panic in masonry_submit"
        );

        output = poison_buffer();
        assert_eq!(
            call_connect(&connect, engine, "panic-poll", &mut output),
            OK
        );
        take_buffer(output, &free);
        output = poison_buffer();
        assert_eq!(poll(engine, &mut output), PANIC);
        assert_eq!(
            String::from_utf8(take_buffer(output, &free)).unwrap(),
            "Rust panic in masonry_poll"
        );

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
    }
}
