use std::{ptr, sync::Mutex};

use battlement_flatbuffers::{
  ConnectInput, ConnectView, CoreClientMessageView, MessageWriter, NativeBatchStart,
  ReducedMotionPreference, ResponseView, UiEventActionView,
  test_support::{navigation_submit_ui_event, pointer_enter_core_action},
  write_connect, write_empty_response,
};
use battlement_native::{
  BattlementBuffer, Engine, EngineError, EngineResponse, FlatBufferSubmitError, NO_MESSAGE, OK,
  ResponseBudget, UiEventResult, buffer_free, ffi_connect, ffi_create, ffi_destroy, ffi_poll,
  ffi_submit, ffi_submit_ui_event,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct DirectEngine {
  session: [u8; 16],
  budget: Option<ResponseBudget>,
}

impl Engine for DirectEngine {
  const WIRE_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_DIGEST_C;

  fn connect(&mut self, _: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let mut response = EngineResponse::from_core(
      self.session,
      write_empty_response(self.session).map_err(error)?,
    )?;
    if let Some(budget) = &self.budget {
      assert!(budget.admit(&mut response));
    }
    Ok(response)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    CoreClientMessageView::read(bytes)
      .map_err(|failure| FlatBufferSubmitError::invalid_argument(failure.to_string()))?;
    let mut writer = MessageWriter::default();
    let label = writer.label("direct");
    let command = writer
      .update_visual_element([4; 16], true, [5; 16], label)
      .map_err(|failure| FlatBufferSubmitError::engine(error(failure)))?;
    let group = writer
      .parallel_group(&[command])
      .map_err(|failure| FlatBufferSubmitError::engine(error(failure)))?;
    let batch = writer
      .batch([6; 16], self.session, None, NativeBatchStart::Now, &[group])
      .map_err(|failure| FlatBufferSubmitError::engine(error(failure)))?;
    let response = writer
      .finish(self.session, &[batch])
      .map_err(|failure| FlatBufferSubmitError::engine(error(failure)))?;
    EngineResponse::from_core(self.session, response).map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: EngineResponse::from_core(
        self.session,
        write_empty_response(self.session).map_err(error)?,
      )?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    Ok(None)
  }
}

#[test]
fn adapter_accepts_finished_flatbuffers() {
  let _guard = TEST_LOCK.lock().unwrap();
  let session = [9; 16];
  let factory = || {
    Ok(DirectEngine {
      session,
      budget: None,
    })
  };
  let mut engine = ptr::null_mut();
  let mut output = BattlementBuffer::EMPTY;
  assert_eq!(unsafe { ffi_create(factory, &mut engine, &mut output) }, OK);
  assert!(!engine.is_null());

  let connect = connect_bytes();
  assert_eq!(
    unsafe {
      ffi_connect(
        factory,
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

  let action = pointer_enter_core_action([7; 16], session, [8; 16]);
  output = BattlementBuffer::EMPTY;
  assert_eq!(
    unsafe {
      ffi_submit(
        factory,
        engine,
        action.as_ptr(),
        action.len() as u64,
        &mut output,
      )
    },
    OK
  );
  let response = unsafe { take_bytes(output) };
  assert_eq!(ResponseView::read(&response).unwrap().session_id(), session);

  let event = navigation_submit_ui_event([3; 16], session, [4; 16], true, true);
  let mut disposition = u32::MAX;
  output = BattlementBuffer::EMPTY;
  assert_eq!(
    unsafe {
      ffi_submit_ui_event(
        factory,
        engine,
        event.as_ptr(),
        event.len() as u64,
        &mut disposition,
        &mut output,
      )
    },
    OK
  );
  assert_eq!(
    disposition,
    battlement::UiEventDisposition::PreventDefault as u32
  );
  unsafe { buffer_free(output) };

  output = BattlementBuffer::EMPTY;
  assert_eq!(
    unsafe { ffi_poll(factory, engine, &mut output) },
    NO_MESSAGE
  );
  assert_eq!(unsafe { ffi_destroy(factory, engine, &mut output) }, OK);
}

fn connect_bytes() -> Vec<u8> {
  write_connect(&ConnectInput {
    host_settings: None,
    platform: "test",
    unity_version: "test",
    screen_width: 1920,
    screen_height: 1080,
    focused: true,
    paused: false,
    reduced_motion_preference: ReducedMotionPreference::NoPreference,
    custom_command_types: &[],
    modules: &[],
    persistent_data_path: None,
    streaming_assets_path: None,
  })
  .unwrap()
  .as_bytes()
  .to_vec()
}

fn error(failure: impl std::fmt::Display) -> EngineError {
  EngineError::new(failure.to_string())
}

unsafe fn take_bytes(buffer: BattlementBuffer) -> Vec<u8> {
  assert!(!buffer.data.is_null());
  let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length as usize) }.to_vec();
  unsafe { buffer_free(buffer) };
  bytes
}

#[test]
fn native_buffer_release_returns_admission_after_the_last_host_use() {
  let _guard = TEST_LOCK.lock().unwrap();
  let budget = ResponseBudget::new(32 * 1024 * 1024);
  let factory = || {
    Ok(DirectEngine {
      session: [9; 16],
      budget: Some(budget.clone()),
    })
  };
  let mut engine = ptr::null_mut();
  let mut output = BattlementBuffer::EMPTY;
  assert_eq!(unsafe { ffi_create(factory, &mut engine, &mut output) }, OK);
  let connect = connect_bytes();
  assert_eq!(
    unsafe {
      ffi_connect(
        factory,
        engine,
        connect.as_ptr(),
        connect.len() as u64,
        &mut output,
      )
    },
    OK
  );
  assert_eq!(budget.retained_bytes(), output.allocation_bytes as usize);
  assert!(budget.retained_bytes() > 0);
  let mut diagnostic = BattlementBuffer::EMPTY;
  assert_eq!(unsafe { ffi_destroy(factory, engine, &mut diagnostic) }, OK);
  assert!(
    budget.retained_bytes() > 0,
    "destroying an engine cannot release a borrowed response"
  );
  unsafe {
    buffer_free(output);
  }
  assert_eq!(budget.retained_bytes(), 0);
}
