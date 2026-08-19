use std::{
    collections::VecDeque,
    ptr,
    sync::{Arc, Mutex},
    thread,
};

use masonry::{ClientMessage, Command, Connect, CoreErrorCode, Response, SessionId, messagepack};
use masonry_native::{
    ENGINE_ERROR, Engine, EngineError, INVALID_ARGUMENT, MasonryBuffer, MasonryEngine, NO_MESSAGE,
    OK, PANIC, buffer_free, connect, create, destroy, poll, submit,
};

const CONNECT_BYTES: &[u8] =
    include_bytes!("../../../Packages/com.masonry.client/Tests/Fixtures/csharp-connect.msgpack");
const ACTION_BYTES: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-enter.msgpack"
);

#[derive(Default)]
struct State {
    connects: Vec<Connect>,
    submissions: usize,
    fail_submit: bool,
}

struct FakeEngine {
    state: Arc<Mutex<State>>,
    pending: Arc<Mutex<VecDeque<Response<Command>>>>,
    immediate: Response<Command>,
}

impl Engine for FakeEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.pending.lock().unwrap().clear();
        self.state.lock().unwrap().connects.push(message);
        Ok(self.immediate.clone())
    }

    fn submit(
        &mut self,
        _message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let mut state = self.state.lock().unwrap();
        state.submissions += 1;
        if state.fail_submit {
            return Err(EngineError::new("fake submit failed"));
        }
        Ok(self.immediate.clone())
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        Ok(self.pending.lock().unwrap().pop_front())
    }
}

fn fake_engine(
    state: Arc<Mutex<State>>,
    pending: Arc<Mutex<VecDeque<Response<Command>>>>,
    response: Response<Command>,
) -> FakeEngine {
    FakeEngine {
        state,
        pending,
        immediate: response,
    }
}

fn poison_buffer() -> MasonryBuffer {
    MasonryBuffer {
        data: ptr::dangling_mut::<u8>(),
        length: u64::MAX,
    }
}

unsafe fn take_bytes(buffer: MasonryBuffer) -> Vec<u8> {
    assert!(!buffer.data.is_null());
    let bytes = unsafe {
        std::slice::from_raw_parts(buffer.data, usize::try_from(buffer.length).unwrap()).to_vec()
    };
    unsafe { buffer_free(buffer) };
    bytes
}

unsafe fn take_error(buffer: MasonryBuffer) -> String {
    String::from_utf8(unsafe { take_bytes(buffer) }).unwrap()
}

#[test]
fn raw_adapter_contract_covers_lifecycle_calls_and_buffers() {
    let state = Arc::new(Mutex::new(State::default()));
    let pending = Arc::new(Mutex::new(VecDeque::new()));
    let response = Response::new(SessionId::new_v4(), Vec::new());
    let expected_response = messagepack::to_vec(&response).unwrap();
    let mut engine = ptr::null_mut();
    let mut output = poison_buffer();

    let status = unsafe {
        create(
            || {
                Ok(fake_engine(
                    Arc::clone(&state),
                    Arc::clone(&pending),
                    response.clone(),
                ))
            },
            &mut engine,
            &mut output,
        )
    };
    assert_eq!(status, OK);
    assert!(!engine.is_null());
    assert!(output.data.is_null());
    assert_eq!(output.length, 0);

    let status = unsafe {
        connect(
            engine,
            CONNECT_BYTES.as_ptr(),
            CONNECT_BYTES.len() as u64,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    let status = unsafe {
        submit(
            engine,
            ACTION_BYTES.as_ptr(),
            ACTION_BYTES.len() as u64,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    assert_eq!(unsafe { poll(engine, ptr::null_mut()) }, INVALID_ARGUMENT);

    let mut duplicate = ptr::dangling_mut::<MasonryEngine<FakeEngine>>();
    output = poison_buffer();
    let status = unsafe {
        create(
            || {
                Ok(fake_engine(
                    Arc::clone(&state),
                    Arc::clone(&pending),
                    response.clone(),
                ))
            },
            &mut duplicate,
            &mut output,
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    assert!(duplicate.is_null());
    assert!(unsafe { take_error(output) }.contains("already live"));

    output = poison_buffer();
    let status = unsafe { connect(engine, ptr::null(), 1, &mut output) };
    assert_eq!(status, INVALID_ARGUMENT);
    assert!(unsafe { take_error(output) }.contains("input pointer is null"));

    let mut connect_bytes = CONNECT_BYTES.to_vec();
    output = poison_buffer();
    let status = unsafe {
        connect(
            engine,
            connect_bytes.as_ptr(),
            connect_bytes.len() as u64,
            &mut output,
        )
    };
    assert_eq!(status, OK);
    assert_eq!(unsafe { take_bytes(output) }, expected_response);
    connect_bytes.fill(0);
    assert_eq!(state.lock().unwrap().connects[0].platform, "macOS");

    pending.lock().unwrap().push_back(response.clone());
    output = poison_buffer();
    let status = unsafe {
        connect(
            engine,
            CONNECT_BYTES.as_ptr(),
            CONNECT_BYTES.len() as u64,
            &mut output,
        )
    };
    assert_eq!(status, OK);
    unsafe { buffer_free(output) };
    assert_eq!(state.lock().unwrap().connects.len(), 2);
    assert!(pending.lock().unwrap().is_empty());

    output = poison_buffer();
    let status = unsafe {
        submit(
            engine,
            ACTION_BYTES.as_ptr(),
            ACTION_BYTES.len() as u64,
            &mut output,
        )
    };
    assert_eq!(status, OK);
    assert_eq!(unsafe { take_bytes(output) }, expected_response);
    assert_eq!(state.lock().unwrap().submissions, 1);

    output = poison_buffer();
    let status = unsafe { poll(engine, &mut output) };
    assert_eq!(status, NO_MESSAGE);
    assert!(output.data.is_null());
    assert_eq!(output.length, 0);

    let worker_queue = Arc::clone(&pending);
    let worker_response = response.clone();
    thread::spawn(move || worker_queue.lock().unwrap().push_back(worker_response))
        .join()
        .unwrap();
    output = poison_buffer();
    let status = unsafe { poll(engine, &mut output) };
    assert_eq!(status, OK);
    assert_eq!(unsafe { take_bytes(output) }, expected_response);

    state.lock().unwrap().fail_submit = true;
    output = poison_buffer();
    let status = unsafe {
        submit(
            engine,
            ACTION_BYTES.as_ptr(),
            ACTION_BYTES.len() as u64,
            &mut output,
        )
    };
    assert_eq!(status, ENGINE_ERROR);
    assert_eq!(unsafe { take_error(output) }, "fake submit failed");

    output = poison_buffer();
    let status = unsafe { connect(engine, [0xc1].as_ptr(), 1, &mut output) };
    assert_eq!(status, INVALID_ARGUMENT);
    assert!(unsafe { take_error(output) }.contains("invalid connect MessagePack"));

    unsafe { destroy(engine) };
    unsafe { destroy::<FakeEngine>(ptr::null_mut()) };
    unsafe { buffer_free(MasonryBuffer::EMPTY) };

    let mut failed_engine = ptr::dangling_mut::<MasonryEngine<FakeEngine>>();
    output = poison_buffer();
    let status = unsafe {
        create(
            || Err::<FakeEngine, _>(EngineError::new("factory unavailable")),
            &mut failed_engine,
            &mut output,
        )
    };
    assert_eq!(status, ENGINE_ERROR);
    assert!(failed_engine.is_null());
    assert_eq!(unsafe { take_error(output) }, "factory unavailable");

    let mut replacement = ptr::null_mut();
    output = poison_buffer();
    let replacement_response = Response::new(SessionId::new_v4(), Vec::new());
    let status = unsafe {
        create(
            || {
                Ok(fake_engine(
                    Arc::clone(&state),
                    Arc::clone(&pending),
                    replacement_response,
                ))
            },
            &mut replacement,
            &mut output,
        )
    };
    assert_eq!(status, OK);
    assert!(!replacement.is_null());
    assert!(output.data.is_null());
    unsafe { destroy(replacement) };
}

#[test]
fn null_outputs_are_rejected_without_dereferencing_them() {
    assert_eq!(
        [OK, NO_MESSAGE, INVALID_ARGUMENT, ENGINE_ERROR, PANIC],
        [0, 1, 2, 3, 4]
    );

    let mut error = poison_buffer();
    let status = unsafe {
        create(
            || {
                let response = Response::new(SessionId::new_v4(), Vec::new());
                Ok(fake_engine(
                    Arc::new(Mutex::new(State::default())),
                    Arc::new(Mutex::new(VecDeque::new())),
                    response,
                ))
            },
            ptr::null_mut(),
            &mut error,
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    assert!(error.data.is_null());
    assert_eq!(error.length, 0);

    let mut engine = ptr::null_mut();
    let status = unsafe {
        create(
            || {
                let response = Response::new(SessionId::new_v4(), Vec::new());
                Ok(fake_engine(
                    Arc::new(Mutex::new(State::default())),
                    Arc::new(Mutex::new(VecDeque::new())),
                    response,
                ))
            },
            &mut engine,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    assert!(engine.is_null());

    let mut output = poison_buffer();
    let status = unsafe {
        connect::<FakeEngine>(
            ptr::null_mut(),
            CONNECT_BYTES.as_ptr(),
            CONNECT_BYTES.len() as u64,
            &mut output,
        )
    };
    assert_eq!(status, INVALID_ARGUMENT);
    assert_eq!(unsafe { take_error(output) }, "engine pointer is null");
}
