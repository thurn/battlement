use battlement::host_settings::HostSettings;
use std::{error::Error, fmt};

use flatbuffers::{FlatBufferBuilder, VerifierOptions, WIPOffset};

use crate::{
  MAXIMUM_APPARENT_BYTES, MAXIMUM_MESSAGE_BYTES, MAXIMUM_TABLE_DEPTH, MAXIMUM_TABLE_VISITS,
  common_generated as wire_common,
  connect_generated::battlement::flat_buffers::generated as wire,
  host_settings::{self, HostSettingsView},
};

/// Host preference for reducing nonessential motion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducedMotionPreference {
  /// The current host cannot report a preference.
  Unavailable,
  /// The host requests reduced motion.
  Reduce,
  /// The host reports no reduced-motion preference.
  NoPreference,
}

impl ReducedMotionPreference {
  /// Returns the stable schema ordinal.
  #[must_use]
  pub const fn ordinal(self) -> u8 {
    match self {
      Self::Unavailable => 0,
      Self::Reduce => 1,
      Self::NoPreference => 2,
    }
  }
}

/// Borrowed values used to construct a connect request directly.
pub struct ConnectInput<'a> {
  /// Initial host capability observation, or unavailable defaults.
  pub host_settings: Option<&'a HostSettings>,
  /// Unity platform name.
  pub platform: &'a str,
  /// Exact Unity editor or player version.
  pub unity_version: &'a str,
  /// Screen width in physical pixels.
  pub screen_width: u32,
  /// Screen height in physical pixels.
  pub screen_height: u32,
  /// Whether the application owns focus.
  pub focused: bool,
  /// Whether Unity has suspended the application.
  pub paused: bool,
  /// Host reduced-motion preference.
  pub reduced_motion_preference: ReducedMotionPreference,
  /// Sorted custom command type names compiled into the build.
  pub custom_command_types: &'a [&'a str],
  /// Selected module identifiers in Inspector order.
  pub modules: &'a [&'a str],
  /// Absolute persistent-data path, when available.
  pub persistent_data_path: Option<&'a str>,
  /// Absolute StreamingAssets path, when available.
  pub streaming_assets_path: Option<&'a str>,
}

/// Constructs a connect request from the public protocol value.
pub fn write_connect_request(
  value: &battlement::Connect,
) -> Result<FinishedMessage, ProtocolError> {
  let custom_command_types = value
    .custom_command_types
    .iter()
    .map(String::as_str)
    .collect::<Vec<_>>();
  let modules = value.modules.iter().map(String::as_str).collect::<Vec<_>>();
  let reduced_motion_preference = match value.reduced_motion_preference {
    battlement::application::ReducedMotionPreference::Unavailable => {
      ReducedMotionPreference::Unavailable
    }
    battlement::application::ReducedMotionPreference::Reduce => ReducedMotionPreference::Reduce,
    battlement::application::ReducedMotionPreference::NoPreference => {
      ReducedMotionPreference::NoPreference
    }
  };
  write_connect(&ConnectInput {
    host_settings: Some(&value.host_settings),
    platform: &value.platform,
    unity_version: &value.unity_version,
    screen_width: value.screen.width,
    screen_height: value.screen.height,
    focused: value.application_state.focused,
    paused: value.application_state.paused,
    reduced_motion_preference,
    custom_command_types: &custom_command_types,
    modules: &modules,
    persistent_data_path: value.persistent_data_path.as_deref(),
    streaming_assets_path: value.streaming_assets_path.as_deref(),
  })
}

/// One finished immutable FlatBuffer and its initialized range.
pub struct FinishedMessage {
  storage: Vec<u8>,
  start: usize,
}

impl FinishedMessage {
  /// Adopts a finished FlatBuffer suffix without copying its payload.
  ///
  /// This is exposed for generated build-composed schemas. Callers must finish,
  /// structurally verify, and semantically validate the message first.
  #[doc(hidden)]
  pub fn from_storage(storage: Vec<u8>, start: usize) -> Self {
    Self { storage, start }
  }

  /// Returns only the initialized finished bytes.
  #[must_use]
  pub fn as_bytes(&self) -> &[u8] {
    &self.storage[self.start..]
  }

  /// Returns the retained allocation size charged to transport accounting.
  #[must_use]
  pub fn allocation_bytes(&self) -> usize {
    self.storage.capacity()
  }

  /// Returns the original allocation and finished-range start without copying.
  #[must_use]
  pub fn into_storage(self) -> (Vec<u8>, usize) {
    (self.storage, self.start)
  }
}

/// A structurally verified and semantically checked borrowed connect request.
#[derive(Clone, Copy)]
pub struct ConnectView<'a> {
  value: wire::ConnectRequest<'a>,
}

impl<'a> ConnectView<'a> {
  /// Verifies and validates one complete connect request.
  pub fn read(bytes: &'a [u8]) -> Result<Self, ProtocolError> {
    if bytes.len() > MAXIMUM_MESSAGE_BYTES {
      return Err(ProtocolError::new("connect request exceeds 16 MiB"));
    }
    if bytes.len() < 4
      || usize::try_from(u32::from_le_bytes(
        bytes[..4].try_into().expect("length checked"),
      ))
      .ok()
        != bytes.len().checked_sub(4)
    {
      return Err(ProtocolError::new(
        "connect request size prefix does not match its finished range",
      ));
    }
    if bytes.len() < 12 || !wire::connect_request_size_prefixed_buffer_has_identifier(bytes) {
      return Err(ProtocolError::new(
        "connect request has the wrong file identifier",
      ));
    }
    let value =
      wire::size_prefixed_root_as_connect_request_with_opts(&verifier_options(), bytes)
        .map_err(|error| ProtocolError::new(format!("invalid connect FlatBuffer: {error}")))?;
    validate(value)?;
    Ok(Self { value })
  }

  /// Returns the validated initial host settings snapshot.
  pub fn host_settings(self) -> HostSettingsView<'a> {
    HostSettingsView::new(self.value.host_settings()).expect("connect host settings were validated")
  }

  /// Returns the borrowed Unity platform name.
  #[must_use]
  pub fn platform(self) -> &'a str {
    self.value.platform()
  }

  /// Returns the borrowed Unity version.
  #[must_use]
  pub fn unity_version(self) -> &'a str {
    self.value.unity_version()
  }

  /// Returns the screen width in physical pixels.
  #[must_use]
  pub fn screen_width(self) -> u32 {
    self.value.screen().width()
  }

  /// Returns the screen height in physical pixels.
  #[must_use]
  pub fn screen_height(self) -> u32 {
    self.value.screen().height()
  }

  /// Returns whether the application owns focus.
  #[must_use]
  pub fn focused(self) -> bool {
    self.value.application_state().focused()
  }

  /// Returns whether Unity has suspended the application.
  #[must_use]
  pub fn paused(self) -> bool {
    self.value.application_state().paused()
  }

  /// Returns the validated reduced-motion preference.
  #[must_use]
  pub fn reduced_motion_preference(self) -> ReducedMotionPreference {
    motion_preference(self.value.reduced_motion_preference())
      .expect("ConnectView stores a semantically validated enum")
  }

  /// Returns the number of custom command type names.
  #[must_use]
  pub fn custom_command_type_count(self) -> usize {
    self.value.custom_command_types().len()
  }

  /// Returns one borrowed custom command type name.
  #[must_use]
  pub fn custom_command_type(self, index: usize) -> Option<&'a str> {
    (index < self.custom_command_type_count()).then(|| self.value.custom_command_types().get(index))
  }

  /// Iterates over borrowed custom command type names.
  pub fn custom_command_types(self) -> impl ExactSizeIterator<Item = &'a str> {
    self.value.custom_command_types().iter()
  }

  /// Returns the number of selected modules.
  #[must_use]
  pub fn module_count(self) -> usize {
    self.value.modules().len()
  }

  /// Returns one borrowed module identifier.
  #[must_use]
  pub fn module(self, index: usize) -> Option<&'a str> {
    (index < self.module_count()).then(|| self.value.modules().get(index))
  }

  /// Iterates over borrowed selected module identifiers.
  pub fn modules(self) -> impl ExactSizeIterator<Item = &'a str> {
    self.value.modules().iter()
  }

  /// Returns the borrowed persistent-data path, when available.
  #[must_use]
  pub fn persistent_data_path(self) -> Option<&'a str> {
    self.value.persistent_data_path()
  }

  /// Returns the borrowed StreamingAssets path, when available.
  #[must_use]
  pub fn streaming_assets_path(self) -> Option<&'a str> {
    self.value.streaming_assets_path()
  }
}

/// A bounded FlatBuffers protocol validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolError {
  message: String,
}

impl ProtocolError {
  pub(crate) fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl fmt::Display for ProtocolError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl Error for ProtocolError {}

/// Constructs a connect request directly in one FlatBuffers allocation.
pub fn write_connect(input: &ConnectInput<'_>) -> Result<FinishedMessage, ProtocolError> {
  validate_input(input)?;
  let mut builder = FlatBufferBuilder::with_capacity(1024);
  let host_settings = host_settings::write(
    &mut builder,
    input.host_settings.unwrap_or(&Default::default()),
  )?;
  let platform = builder.create_string(input.platform);
  let unity_version = builder.create_string(input.unity_version);
  let persistent_data_path = input
    .persistent_data_path
    .map(|value| builder.create_string(value));
  let streaming_assets_path = input
    .streaming_assets_path
    .map(|value| builder.create_string(value));
  let custom_command_types = string_vector(&mut builder, input.custom_command_types);
  let modules = string_vector(&mut builder, input.modules);
  let application_state = wire_common::ApplicationState::create(
    &mut builder,
    &wire_common::ApplicationStateArgs {
      focused: input.focused,
      paused: input.paused,
    },
  );
  let screen = wire_common::ScreenSize::new(input.screen_width, input.screen_height);
  let request = wire::ConnectRequest::create(
    &mut builder,
    &wire::ConnectRequestArgs {
      host_settings: Some(host_settings),
      platform: Some(platform),
      unity_version: Some(unity_version),
      screen: Some(&screen),
      application_state: Some(application_state),
      reduced_motion_preference: wire_motion_preference(input.reduced_motion_preference),
      custom_command_types: Some(custom_command_types),
      modules: Some(modules),
      persistent_data_path,
      streaming_assets_path,
    },
  );
  wire::finish_size_prefixed_connect_request_buffer(&mut builder, request);
  if builder.finished_data().len() > MAXIMUM_MESSAGE_BYTES {
    return Err(ProtocolError::new("connect request exceeds 16 MiB"));
  }
  let (storage, start) = builder.collapse();
  Ok(FinishedMessage::from_storage(storage, start))
}

fn string_vector<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  values: &[&str],
) -> WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>> {
  let offsets = values
    .iter()
    .map(|value| builder.create_string(value))
    .collect::<Vec<_>>();
  builder.create_vector(&offsets)
}

fn validate(value: wire::ConnectRequest<'_>) -> Result<(), ProtocolError> {
  if value.platform().is_empty() || value.unity_version().is_empty() {
    return Err(ProtocolError::new(
      "connect platform and Unity version must be nonempty",
    ));
  }
  if value.screen().width() == 0 || value.screen().height() == 0 {
    return Err(ProtocolError::new(
      "connect screen dimensions must be nonzero",
    ));
  }
  HostSettingsView::new(value.host_settings())?;
  motion_preference(value.reduced_motion_preference())?;
  validate_sorted_unique(value.custom_command_types().iter(), "custom command type")?;
  validate_unique(value.modules().iter(), "module")
}

fn validate_input(input: &ConnectInput<'_>) -> Result<(), ProtocolError> {
  if input.platform.is_empty() || input.unity_version.is_empty() {
    return Err(ProtocolError::new(
      "connect platform and Unity version must be nonempty",
    ));
  }
  if input.screen_width == 0 || input.screen_height == 0 {
    return Err(ProtocolError::new(
      "connect screen dimensions must be nonzero",
    ));
  }
  validate_sorted_unique(
    input.custom_command_types.iter().copied(),
    "custom command type",
  )?;
  validate_unique(input.modules.iter().copied(), "module")
}

fn validate_sorted_unique<'a>(
  values: impl Iterator<Item = &'a str>,
  name: &str,
) -> Result<(), ProtocolError> {
  let mut previous = None;
  for value in values {
    if value.is_empty() || previous.is_some_and(|previous| previous >= value) {
      return Err(ProtocolError::new(format!(
        "{name}s must be nonempty, sorted, and unique"
      )));
    }
    previous = Some(value);
  }
  Ok(())
}

fn validate_unique<'a>(
  values: impl Iterator<Item = &'a str>,
  name: &str,
) -> Result<(), ProtocolError> {
  let mut seen = Vec::new();
  for value in values {
    if value.is_empty() || seen.contains(&value) {
      return Err(ProtocolError::new(format!(
        "{name}s must be nonempty and unique"
      )));
    }
    seen.push(value);
  }
  Ok(())
}

fn motion_preference(
  value: wire_common::ReducedMotionPreference,
) -> Result<ReducedMotionPreference, ProtocolError> {
  match value {
    wire_common::ReducedMotionPreference::Unavailable => Ok(ReducedMotionPreference::Unavailable),
    wire_common::ReducedMotionPreference::Reduce => Ok(ReducedMotionPreference::Reduce),
    wire_common::ReducedMotionPreference::NoPreference => Ok(ReducedMotionPreference::NoPreference),
    _ => Err(ProtocolError::new(
      "connect request has an unknown reduced-motion value",
    )),
  }
}

fn wire_motion_preference(value: ReducedMotionPreference) -> wire_common::ReducedMotionPreference {
  match value {
    ReducedMotionPreference::Unavailable => wire_common::ReducedMotionPreference::Unavailable,
    ReducedMotionPreference::Reduce => wire_common::ReducedMotionPreference::Reduce,
    ReducedMotionPreference::NoPreference => wire_common::ReducedMotionPreference::NoPreference,
  }
}

pub(crate) fn verifier_options() -> VerifierOptions {
  VerifierOptions {
    max_depth: MAXIMUM_TABLE_DEPTH,
    max_tables: MAXIMUM_TABLE_VISITS,
    max_apparent_size: MAXIMUM_APPARENT_BYTES,
    ignore_missing_null_terminator: false,
  }
}
