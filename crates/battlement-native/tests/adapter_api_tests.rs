use std::{
  collections::VecDeque,
  ptr,
  sync::{Arc, Mutex},
  thread,
};

use battlement::{
  ClientMessage, Command, CoreErrorCode, ObjectId, Response, SessionId, UiEventAction,
  UiEventResponse,
};
use battlement_flatbuffers::{
  ConnectInput, ConnectView, CoreClientMessageView, MessageWriter, NativeBatchStart,
  ReducedMotionPreference, ResponseView, UiEventActionView,
  test_support::{navigation_submit_ui_event, pointer_enter_core_action},
  write_connect, write_empty_response,
};
use battlement_native::{
  BattlementBuffer, BattlementEngine, ENGINE_ERROR, Engine, EngineError, FlatBufferSubmitError,
  INVALID_ARGUMENT, MAXIMUM_DIAGNOSTIC_BYTES, NO_MESSAGE, NativeEngine, NativeResponse,
  NativeUiEventResponse, OK, PANIC, buffer_free, connect, create, destroy, ffi_native_connect,
  ffi_native_create, ffi_native_destroy, ffi_native_poll, ffi_native_submit,
  ffi_native_submit_ui_event, poll, submit, submit_ui_event,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct DirectEngine {
  session: [u8; 16],
}

impl NativeEngine for DirectEngine {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect_native(&mut self, _: ConnectView<'_>) -> Result<NativeResponse, EngineError> {
    NativeResponse::from_core(
      self.session,
      write_empty_response(self.session).map_err(|error| EngineError::new(error.to_string()))?,
    )
  }

  fn submit_native(&mut self, bytes: &[u8]) -> Result<NativeResponse, FlatBufferSubmitError> {
    CoreClientMessageView::read(bytes)
      .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
    let mut writer = MessageWriter::default();
    let label = writer.label("direct");
    let command = writer
      .update_visual_element([4; 16], true, [5; 16], label)
      .map_err(|error| FlatBufferSubmitError::engine(EngineError::new(error.to_string())))?;
    let group = writer
      .parallel_group(&[command])
      .map_err(|error| FlatBufferSubmitError::engine(EngineError::new(error.to_string())))?;
    let batch = writer
      .batch([6; 16], self.session, None, NativeBatchStart::Now, &[group])
      .map_err(|error| FlatBufferSubmitError::engine(EngineError::new(error.to_string())))?;
    let response = writer
      .finish(self.session, &[batch])
      .map_err(|error| FlatBufferSubmitError::engine(EngineError::new(error.to_string())))?;
    NativeResponse::from_core(self.session, response).map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event_native(
    &mut self,
    _: battlement_native::UiEventActionView<'_>,
  ) -> Result<NativeUiEventResponse, EngineError> {
    Ok(NativeUiEventResponse {
      disposition: battlement::UiEventDisposition::Continue,
      response: NativeResponse::from_core(
        self.session,
        write_empty_response(self.session).map_err(|error| EngineError::new(error.to_string()))?,
      )?,
    })
  }

  fn poll_native(&mut self) -> Result<Option<NativeResponse>, EngineError> {
    Ok(None)
  }
}

#[derive(Default)]
struct State {
  connects: Vec<String>,
  submissions: usize,
  fail_submit: bool,
  fail_ui_event: bool,
}

struct FakeEngine {
  state: Arc<Mutex<State>>,
  pending: Arc<Mutex<VecDeque<Response<Command>>>>,
  immediate: Response<Command>,
}

#[test]
fn direct_native_adapter_only_accepts_finished_flatbuffers() {
  let _guard = TEST_LOCK.lock().unwrap();
  let session = [9; 16];
  let mut engine = ptr::null_mut();
  let mut output = BattlementBuffer::EMPTY;
  assert_eq!(
    unsafe { ffi_native_create(|| DirectEngine { session }, &mut engine, &mut output,) },
    OK
  );
  assert!(!engine.is_null());

  let connect = connect_bytes();
  assert_eq!(
    unsafe {
      ffi_native_connect(
        || DirectEngine { session },
        engine,
        connect.as_ptr(),
        connect.len() as u64,
        &mut output,
      )
    },
    OK
  );
  let response = unsafe { take_bytes(output) };
  assert_eq!(ResponseView::read(&response).unwrap().session_id(), session);
  output = BattlementBuffer::EMPTY;

  let action = pointer_enter_core_action([7; 16], session, [8; 16]);
  assert_eq!(
    unsafe {
      ffi_native_submit(
        || DirectEngine { session },
        engine,
        action.as_ptr(),
        action.len() as u64,
        &mut output,
      )
    },
    OK
  );
  unsafe { buffer_free(output) };
  output = BattlementBuffer::EMPTY;

  let session_id = SessionId::from_uuid(uuid::Uuid::from_bytes(session)).unwrap();
  let ui_event = ui_event_bytes(session_id, false);
  let mut disposition = u32::MAX;
  assert_eq!(
    unsafe {
      ffi_native_submit_ui_event(
        || DirectEngine { session },
        engine,
        ui_event.as_ptr(),
        ui_event.len() as u64,
        &mut disposition,
        &mut output,
      )
    },
    OK
  );
  assert_eq!(disposition, battlement::UiEventDisposition::Continue as u32);
  unsafe { buffer_free(output) };
  output = BattlementBuffer::EMPTY;

  assert_eq!(
    unsafe { ffi_native_poll(|| DirectEngine { session }, engine, &mut output) },
    NO_MESSAGE
  );
  assert_eq!(
    unsafe { ffi_native_destroy(|| DirectEngine { session }, engine, &mut output) },
    OK
  );
}

impl Engine for FakeEngine {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<Response<Self::Command>, EngineError> {
    self.pending.lock().unwrap().clear();
    self
      .state
      .lock()
      .unwrap()
      .connects
      .push(message.platform().to_owned());
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

  fn submit_core_view(
    &mut self,
    _message: CoreClientMessageView<'_>,
  ) -> Result<Response<Self::Command>, EngineError> {
    let mut state = self.state.lock().unwrap();
    state.submissions += 1;
    if state.fail_submit {
      return Err(EngineError::new("fake submit failed"));
    }
    Ok(self.immediate.clone())
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    if self.state.lock().unwrap().fail_ui_event {
      return Err(EngineError::new("fake UI event failed"));
    }
    Ok(UiEventResponse::from_event(
      &action.event,
      self.immediate.clone(),
    ))
  }

  fn submit_ui_event_view(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    if self.state.lock().unwrap().fail_ui_event {
      return Err(EngineError::new("fake UI event failed"));
    }
    Ok(UiEventResponse::new(
      if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      self.immediate.clone(),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
    Ok(self.pending.lock().unwrap().pop_front())
  }
}

fn ui_event_bytes(session_id: SessionId, default_prevented: bool) -> Vec<u8> {
  navigation_submit_ui_event(
    *battlement::ActionId::new_v4().as_uuid().as_bytes(),
    *session_id.as_uuid().as_bytes(),
    *ObjectId::new_v4().as_uuid().as_bytes(),
    true,
    default_prevented,
  )
}

fn connect_bytes() -> Vec<u8> {
  write_connect(&ConnectInput {
    platform: "macOS",
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

fn poison_buffer() -> BattlementBuffer {
  BattlementBuffer {
    data: ptr::dangling_mut::<u8>(),
    length: u64::MAX,
    allocation_bytes: u64::MAX,
    allocation_data: ptr::dangling_mut::<u8>(),
    allocation_length: u64::MAX,
  }
}

unsafe fn take_bytes(buffer: BattlementBuffer) -> Vec<u8> {
  assert!(!buffer.data.is_null());
  let bytes = unsafe {
    std::slice::from_raw_parts(buffer.data, usize::try_from(buffer.length).unwrap()).to_vec()
  };
  unsafe { buffer_free(buffer) };
  bytes
}

unsafe fn take_error(buffer: BattlementBuffer) -> String {
  String::from_utf8(unsafe { take_bytes(buffer) }).unwrap()
}

fn assert_response(bytes: Vec<u8>, session_id: SessionId) {
  let value = ResponseView::read(&bytes).expect("adapter returned a verified response");
  assert_eq!(value.session_id(), *session_id.as_uuid().as_bytes());
  assert_eq!(value.message_count(), 0);
}

struct ComposedSchemaEngine(Response<Command>);

impl Engine for ComposedSchemaEngine {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _: ConnectView<'_>) -> Result<Response<Command>, EngineError> {
    Ok(self.0.clone())
  }

  fn submit(&mut self, _: ClientMessage<(), ()>) -> Result<Response<Command>, EngineError> {
    unreachable!("native submissions use submit_flatbuffer")
  }

  fn submit_flatbuffer(
    &mut self,
    bytes: &[u8],
  ) -> Result<Response<Command>, FlatBufferSubmitError> {
    if bytes != b"fixture-composed-message" {
      return Err(FlatBufferSubmitError::invalid_argument(
        "invalid fixture composed message",
      ));
    }
    Ok(self.0.clone())
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Command>, EngineError> {
    Ok(UiEventResponse::from_event(&action.event, self.0.clone()))
  }

  fn poll(&mut self) -> Result<Option<Response<Command>>, EngineError> {
    Ok(None)
  }
}

#[test]
fn engine_can_verify_a_build_composed_client_schema() {
  let _guard = TEST_LOCK.lock().unwrap();
  let response = Response::new(SessionId::new_v4(), Vec::new());
  let response_session = response.session_id;
  let mut engine = ptr::null_mut();
  let mut output = BattlementBuffer::EMPTY;
  assert_eq!(
    unsafe {
      create(
        || Ok(ComposedSchemaEngine(response)),
        &mut engine,
        &mut output,
      )
    },
    OK
  );

  let message = b"fixture-composed-message";
  assert_eq!(
    unsafe { submit(engine, message.as_ptr(), message.len() as u64, &mut output) },
    OK
  );
  assert_response(unsafe { take_bytes(output) }, response_session);

  output = BattlementBuffer::EMPTY;
  let invalid = b"not-the-schema";
  assert_eq!(
    unsafe { submit(engine, invalid.as_ptr(), invalid.len() as u64, &mut output) },
    INVALID_ARGUMENT
  );
  assert!(unsafe { take_error(output) }.contains("invalid fixture composed message"));
  unsafe { destroy(engine) };
}

#[test]
fn raw_adapter_contract_covers_lifecycle_calls_and_buffers() {
  let _guard = TEST_LOCK.lock().unwrap();
  let state = Arc::new(Mutex::new(State::default()));
  let pending = Arc::new(Mutex::new(VecDeque::new()));
  let response = Response::new(SessionId::new_v4(), Vec::new());
  let response_session = response.session_id;
  let mut engine = ptr::null_mut();
  let mut output = poison_buffer();
  let action_bytes = core_action_bytes();

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

  let valid_connect = connect_bytes();
  let status = unsafe {
    connect(
      engine,
      valid_connect.as_ptr(),
      valid_connect.len() as u64,
      ptr::null_mut(),
    )
  };
  assert_eq!(status, INVALID_ARGUMENT);
  let status = unsafe {
    submit(
      engine,
      action_bytes.as_ptr(),
      action_bytes.len() as u64,
      ptr::null_mut(),
    )
  };
  assert_eq!(status, INVALID_ARGUMENT);
  assert_eq!(unsafe { poll(engine, ptr::null_mut()) }, INVALID_ARGUMENT);

  let mut duplicate = ptr::dangling_mut::<BattlementEngine<FakeEngine>>();
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

  let mut sent_connect = connect_bytes();
  output = poison_buffer();
  let status = unsafe {
    connect(
      engine,
      sent_connect.as_ptr(),
      sent_connect.len() as u64,
      &mut output,
    )
  };
  assert_eq!(status, OK);
  assert_response(unsafe { take_bytes(output) }, response_session);
  sent_connect.fill(0);
  assert_eq!(state.lock().unwrap().connects[0], "macOS");

  pending.lock().unwrap().push_back(response.clone());
  output = poison_buffer();
  let reconnect_bytes = connect_bytes();
  let status = unsafe {
    connect(
      engine,
      reconnect_bytes.as_ptr(),
      reconnect_bytes.len() as u64,
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
      action_bytes.as_ptr(),
      action_bytes.len() as u64,
      &mut output,
    )
  };
  assert_eq!(status, OK);
  assert_response(unsafe { take_bytes(output) }, response_session);
  assert_eq!(state.lock().unwrap().submissions, 1);

  output = poison_buffer();
  let status = unsafe {
    submit(
      engine,
      ptr::dangling::<u8>(),
      battlement_flatbuffers::MAXIMUM_MESSAGE_BYTES as u64 + 1,
      &mut output,
    )
  };
  assert_eq!(status, INVALID_ARGUMENT);
  assert_eq!(unsafe { take_error(output) }, "input length exceeds 16 MiB");
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
  assert_response(unsafe { take_bytes(output) }, response_session);

  state.lock().unwrap().fail_submit = true;
  output = poison_buffer();
  let status = unsafe {
    submit(
      engine,
      action_bytes.as_ptr(),
      action_bytes.len() as u64,
      &mut output,
    )
  };
  assert_eq!(status, ENGINE_ERROR);
  assert_eq!(unsafe { take_error(output) }, "fake submit failed");

  output = poison_buffer();
  let status = unsafe { connect(engine, [0xc1].as_ptr(), 1, &mut output) };
  assert_eq!(status, INVALID_ARGUMENT);
  assert!(unsafe { take_error(output) }.contains("invalid connect"));

  unsafe { destroy(engine) };
  unsafe { destroy::<FakeEngine>(ptr::null_mut()) };
  unsafe { buffer_free(BattlementBuffer::EMPTY) };

  let mut failed_engine = ptr::dangling_mut::<BattlementEngine<FakeEngine>>();
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

  output = poison_buffer();
  let status = unsafe {
    create(
      || Err::<FakeEngine, _>(EngineError::new("x".repeat(MAXIMUM_DIAGNOSTIC_BYTES + 1))),
      &mut failed_engine,
      &mut output,
    )
  };
  assert_eq!(status, ENGINE_ERROR);
  assert!(failed_engine.is_null());
  let diagnostic = unsafe { take_error(output) };
  assert_eq!(diagnostic.len(), MAXIMUM_DIAGNOSTIC_BYTES);
  assert!(diagnostic.ends_with("\n[diagnostic truncated]"));

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
  let _guard = TEST_LOCK.lock().unwrap();
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
  let connect_bytes = connect_bytes();
  let status = unsafe {
    connect::<FakeEngine>(
      ptr::null_mut(),
      connect_bytes.as_ptr(),
      connect_bytes.len() as u64,
      &mut output,
    )
  };
  assert_eq!(status, INVALID_ARGUMENT);
  assert_eq!(unsafe { take_error(output) }, "engine pointer is null");
}

#[test]
fn ui_event_adapter_returns_only_valid_complete_results() {
  let _guard = TEST_LOCK.lock().unwrap();
  let state = Arc::new(Mutex::new(State::default()));
  let session_id = SessionId::new_v4();
  let response = Response::empty(session_id);
  let mut engine = ptr::null_mut();
  let mut output = poison_buffer();
  assert_eq!(
    unsafe {
      create(
        || {
          Ok(fake_engine(
            Arc::clone(&state),
            Arc::new(Mutex::new(VecDeque::new())),
            response.clone(),
          ))
        },
        &mut engine,
        &mut output,
      )
    },
    OK
  );

  let request = self::ui_event_bytes(session_id, false);
  let mut disposition = u32::MAX;
  output = poison_buffer();
  assert_eq!(
    unsafe {
      submit_ui_event(
        engine,
        request.as_ptr(),
        request.len() as u64,
        &mut disposition,
        &mut output,
      )
    },
    OK
  );
  assert_eq!(disposition, 0);
  assert_response(unsafe { take_bytes(output) }, session_id);

  let prevented = self::ui_event_bytes(session_id, true);
  output = poison_buffer();
  assert_eq!(
    unsafe {
      submit_ui_event(
        engine,
        prevented.as_ptr(),
        prevented.len() as u64,
        &mut disposition,
        &mut output,
      )
    },
    OK
  );
  assert_eq!(disposition, 1);
  unsafe { buffer_free(output) };

  state.lock().unwrap().fail_ui_event = true;
  output = poison_buffer();
  disposition = u32::MAX;
  assert_eq!(
    unsafe {
      submit_ui_event(
        engine,
        request.as_ptr(),
        request.len() as u64,
        &mut disposition,
        &mut output,
      )
    },
    ENGINE_ERROR
  );
  assert_eq!(disposition, 0);
  assert_eq!(unsafe { take_error(output) }, "fake UI event failed");

  disposition = u32::MAX;
  output = poison_buffer();
  assert_eq!(
    unsafe {
      submit_ui_event::<FakeEngine>(
        ptr::null_mut(),
        ptr::null(),
        1,
        &mut disposition,
        &mut output,
      )
    },
    INVALID_ARGUMENT
  );
  assert_eq!(disposition, 0);
  assert_eq!(unsafe { take_error(output) }, "engine pointer is null");

  unsafe { destroy(engine) };
}
