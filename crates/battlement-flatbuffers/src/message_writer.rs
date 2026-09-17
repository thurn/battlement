use std::{
  cell::RefCell,
  sync::atomic::{AtomicU64, Ordering},
};

use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{
  FinishedMessage, MAXIMUM_MESSAGE_BYTES, ProtocolError, client_message_generated as client_wire,
  command_core_generated as command_wire, common_generated as common, response::ResponseView,
  response_generated::battlement::flat_buffers::generated as wire,
  retained_ui::RetainedTextPropertyView, ui_event_generated as input_wire, ui_generated as ui_wire,
  world_generated as world_wire,
};

static NEXT_BUILDER_ID: AtomicU64 = AtomicU64::new(1);
static MESSAGE_BUILDERS_CREATED: AtomicU64 = AtomicU64::new(0);
static MESSAGE_BUILDERS_REUSED: AtomicU64 = AtomicU64::new(0);
static MESSAGE_BUILDER_GROWTHS: AtomicU64 = AtomicU64::new(0);
static MESSAGE_BUILDER_COPIED_BYTES: AtomicU64 = AtomicU64::new(0);
const MAXIMUM_BUILDER_BYTES: usize = 32 * 1024 * 1024;
const MAXIMUM_POOLED_BUILDER_BYTES: usize = 1024 * 1024;
const MAXIMUM_IDLE_BUILDER_BYTES: usize = 16 * 1024 * 1024;

thread_local! {
  static MESSAGE_BUILDER_POOL: RefCell<MessageBuilderPool> = RefCell::new(MessageBuilderPool::default());
}

#[derive(Default)]
struct MessageBuilderPool {
  storage: Vec<Vec<u8>>,
  idle_bytes: usize,
}

/// Cumulative direct-response builder allocation diagnostics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MessageWriterDiagnostics {
  /// Builders backed by a fresh allocation.
  pub created: u64,
  /// Builders backed by released transport storage.
  pub reused: u64,
  /// Allocation growth operations observed on finished messages.
  pub growths: u64,
  /// Payload bytes copied by those growth operations.
  pub copied_bytes: u64,
  /// Reusable allocation bytes idle on the current owning thread.
  pub idle_bytes: usize,
}

/// Returns direct-response builder allocation and reuse diagnostics.
#[must_use]
pub fn message_writer_diagnostics() -> MessageWriterDiagnostics {
  MessageWriterDiagnostics {
    created: MESSAGE_BUILDERS_CREATED.load(Ordering::Relaxed),
    reused: MESSAGE_BUILDERS_REUSED.load(Ordering::Relaxed),
    growths: MESSAGE_BUILDER_GROWTHS.load(Ordering::Relaxed),
    copied_bytes: MESSAGE_BUILDER_COPIED_BYTES.load(Ordering::Relaxed),
    idle_bytes: MESSAGE_BUILDER_POOL.with_borrow(|pool| pool.idle_bytes),
  }
}

/// Returns one released transport allocation to the direct-response pool.
///
/// Only power-of-two allocations up to 1 MiB are retained. The pool is local
/// to the owning thread and capped at 16 MiB; all other storage is dropped.
#[doc(hidden)]
pub fn recycle_message_storage(mut storage: Vec<u8>) {
  let allocation = storage.capacity();
  if allocation == 0 || allocation > MAXIMUM_POOLED_BUILDER_BYTES || !allocation.is_power_of_two() {
    return;
  }
  if storage.len() != allocation {
    storage.resize(allocation, 0);
  }
  storage.fill(0);
  MESSAGE_BUILDER_POOL.with_borrow_mut(|pool| {
    if pool.idle_bytes > MAXIMUM_IDLE_BUILDER_BYTES - allocation {
      return;
    }
    pool.idle_bytes += allocation;
    pool.storage.push(storage);
  });
}

fn take_message_storage(minimum: usize) -> Vec<u8> {
  let requested = minimum.max(1).next_power_of_two();
  let reused = MESSAGE_BUILDER_POOL.with_borrow_mut(|pool| {
    let index = pool
      .storage
      .iter()
      .enumerate()
      .filter(|(_, storage)| storage.len() >= requested)
      .min_by_key(|(_, storage)| storage.len())
      .map(|(index, _)| index)?;
    let storage = pool.storage.swap_remove(index);
    pool.idle_bytes -= storage.len();
    Some(storage)
  });
  match reused {
    Some(storage) => {
      MESSAGE_BUILDERS_REUSED.fetch_add(1, Ordering::Relaxed);
      storage
    }
    None => {
      MESSAGE_BUILDERS_CREATED.fetch_add(1, Ordering::Relaxed);
      vec![0; requested]
    }
  }
}

/// Start dependency for a directly constructed command batch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NativeBatchStart {
  /// The batch may begin immediately.
  #[default]
  Now,
  /// Wait through all earlier blocking work.
  AfterEarlierBlockingWork,
  /// Wait through earlier asset preparation.
  AfterEarlierAssetPreparation,
}

/// A typed core-command offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct CoreCommandOffset {
  builder_id: u64,
  value: WIPOffset<wire::CoreCommand<'static>>,
}

/// A typed UI-element offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct UiElementOffset {
  builder_id: u64,
  value: WIPOffset<ui_wire::UiElement<'static>>,
}

/// Direct, child-first authoring context for one UI element declaration.
///
/// Property payloads are written into the parent [`MessageWriter`] as methods
/// are called. The builder retains only FlatBuffer offsets and never creates an
/// owned Battlement UI element.
pub struct UiElementBuilder<'a> {
  writer: &'a mut MessageWriter,
  kind: ui_wire::UiElementKind,
  properties: Vec<WIPOffset<ui_wire::UiProperty<'static>>>,
}

impl UiElementBuilder<'_> {
  /// Assigns text content to an element with a text property.
  #[must_use]
  pub fn text(mut self, value: &str) -> Self {
    let property = self
      .writer
      .ui_text_property(ui_wire::UiPropertyKey::Text, value);
    self.properties.push(property);
    self
  }

  /// Assigns textual control content stored under the value property.
  #[must_use]
  pub fn text_value(mut self, value: &str) -> Self {
    let property = self
      .writer
      .ui_text_property(ui_wire::UiPropertyKey::Value, value);
    self.properties.push(property);
    self
  }

  /// Assigns the common enabled state.
  #[must_use]
  pub fn enabled(mut self, value: bool) -> Self {
    let property = self
      .writer
      .ui_bool_property(ui_wire::UiPropertyKey::Enabled, value);
    self.properties.push(property);
    self
  }

  /// Assigns a boolean control value.
  #[must_use]
  pub fn bool_value(mut self, value: bool) -> Self {
    let property = self
      .writer
      .ui_bool_property(ui_wire::UiPropertyKey::Value, value);
    self.properties.push(property);
    self
  }

  /// Assigns a floating-point control value.
  #[must_use]
  pub fn float_value(mut self, value: f32) -> Self {
    let property = self
      .writer
      .ui_float_property(ui_wire::UiPropertyKey::Value, value);
    self.properties.push(property);
    self
  }

  /// Assigns a signed integer control value.
  #[must_use]
  pub fn int_value(mut self, value: i32) -> Self {
    let property = self
      .writer
      .ui_int_property(ui_wire::UiPropertyKey::Value, value);
    self.properties.push(property);
    self
  }

  /// Assigns an optional single-selection index.
  #[must_use]
  pub fn selected_index(mut self, value: Option<u32>) -> Self {
    let property = self
      .writer
      .ui_optional_uint_property(ui_wire::UiPropertyKey::SelectedIndex, value);
    self.properties.push(property);
    self
  }

  /// Assigns a sorted set of selected indices.
  #[must_use]
  pub fn selected_indices(mut self, values: impl IntoIterator<Item = u32>) -> Self {
    let property = self
      .writer
      .ui_uint_list_property(ui_wire::UiPropertyKey::SelectedIndices, values);
    self.properties.push(property);
    self
  }

  /// Assigns an optional dropdown selection pair.
  #[must_use]
  pub fn selection(mut self, index: Option<u32>, value: Option<&str>) -> Self {
    let property = self
      .writer
      .ui_choice_property(ui_wire::UiPropertyKey::Selection, index, value);
    self.properties.push(property);
    self
  }

  /// Assigns both endpoints of a min-max slider.
  #[must_use]
  pub fn range(mut self, min: f32, max: f32) -> Self {
    let min = self
      .writer
      .ui_float_property(ui_wire::UiPropertyKey::MinValue, min);
    let max = self
      .writer
      .ui_float_property(ui_wire::UiPropertyKey::MaxValue, max);
    self.properties.extend([min, max]);
    self
  }

  /// Assigns the selected tab index.
  #[must_use]
  pub fn selected_tab_index(mut self, value: u32) -> Self {
    let property = self
      .writer
      .ui_uint_property(ui_wire::UiPropertyKey::SelectedTabIndex, value);
    self.properties.push(property);
    self
  }

  /// Assigns the initial repeat delay and positive repeat interval.
  #[must_use]
  pub fn repeat_timing(mut self, delay_ms: u32, interval_ms: std::num::NonZeroU32) -> Self {
    let delay = self
      .writer
      .ui_uint_property(ui_wire::UiPropertyKey::DelayMs, delay_ms);
    let interval = self
      .writer
      .ui_uint_property(ui_wire::UiPropertyKey::IntervalMs, interval_ms.get());
    self.properties.extend([delay, interval]);
    self
  }

  /// Assigns the element name.
  #[must_use]
  pub fn name(mut self, value: &str) -> Self {
    let property = self
      .writer
      .ui_text_property(ui_wire::UiPropertyKey::Name, value);
    self.properties.push(property);
    self
  }

  /// Makes this element transparent to pointer picking.
  #[must_use]
  pub fn ignore_picking(mut self) -> Self {
    let property = self.writer.ui_enum_property(
      ui_wire::UiPropertyKey::PickingMode,
      ui_wire::UiEnumCatalog::PickingMode,
      1,
    );
    self.properties.push(property);
    self
  }

  /// Selects absolute layout positioning.
  #[must_use]
  pub fn absolute(mut self) -> Self {
    let property = self.writer.ui_enum_property(
      ui_wire::UiPropertyKey::StylePosition,
      ui_wire::UiEnumCatalog::Position,
      1,
    );
    self.properties.push(property);
    self
  }

  /// Shows or hides an element through its display style.
  #[must_use]
  pub fn displayed(mut self, visible: bool) -> Self {
    let property = self.writer.ui_enum_property(
      ui_wire::UiPropertyKey::StyleDisplay,
      ui_wire::UiEnumCatalog::Display,
      u32::from(!visible),
    );
    self.properties.push(property);
    self
  }

  /// Assigns a pixel top inset.
  #[must_use]
  pub fn top(mut self, pixels: f32) -> Self {
    let property = self
      .writer
      .ui_pixel_property(ui_wire::UiPropertyKey::StyleTop, pixels);
    self.properties.push(property);
    self
  }

  /// Assigns a pixel left inset.
  #[must_use]
  pub fn left(mut self, pixels: f32) -> Self {
    let property = self
      .writer
      .ui_pixel_property(ui_wire::UiPropertyKey::StyleLeft, pixels);
    self.properties.push(property);
    self
  }

  /// Assigns a percentage width.
  #[must_use]
  pub fn width_percent(mut self, percentage: f32) -> Self {
    let property = self
      .writer
      .ui_percentage_property(ui_wire::UiPropertyKey::StyleWidth, percentage);
    self.properties.push(property);
    self
  }

  /// Assigns a pixel height.
  #[must_use]
  pub fn height(mut self, pixels: f32) -> Self {
    let property = self
      .writer
      .ui_pixel_property(ui_wire::UiPropertyKey::StyleHeight, pixels);
    self.properties.push(property);
    self
  }

  /// Assigns a pixel right inset.
  #[must_use]
  pub fn right(mut self, pixels: f32) -> Self {
    let property = self
      .writer
      .ui_pixel_property(ui_wire::UiPropertyKey::StyleRight, pixels);
    self.properties.push(property);
    self
  }

  /// Assigns vertical and horizontal pixel padding.
  #[must_use]
  pub fn padding(mut self, vertical: f32, horizontal: f32) -> Self {
    for (key, pixels) in [
      (ui_wire::UiPropertyKey::StylePaddingTop, vertical),
      (ui_wire::UiPropertyKey::StylePaddingRight, horizontal),
      (ui_wire::UiPropertyKey::StylePaddingBottom, vertical),
      (ui_wire::UiPropertyKey::StylePaddingLeft, horizontal),
    ] {
      let property = self.writer.ui_pixel_property(key, pixels);
      self.properties.push(property);
    }
    self
  }

  /// Assigns vertical and horizontal pixel margins.
  #[must_use]
  pub fn margin(mut self, vertical: f32, horizontal: f32) -> Self {
    for (key, pixels) in [
      (ui_wire::UiPropertyKey::StyleMarginTop, vertical),
      (ui_wire::UiPropertyKey::StyleMarginRight, horizontal),
      (ui_wire::UiPropertyKey::StyleMarginBottom, vertical),
      (ui_wire::UiPropertyKey::StyleMarginLeft, horizontal),
    ] {
      let property = self.writer.ui_pixel_property(key, pixels);
      self.properties.push(property);
    }
    self
  }

  /// Assigns top, right, bottom, and left pixel margins.
  #[must_use]
  pub fn margin_edges(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
    for (key, pixels) in [
      (ui_wire::UiPropertyKey::StyleMarginTop, top),
      (ui_wire::UiPropertyKey::StyleMarginRight, right),
      (ui_wire::UiPropertyKey::StyleMarginBottom, bottom),
      (ui_wire::UiPropertyKey::StyleMarginLeft, left),
    ] {
      let property = self.writer.ui_pixel_property(key, pixels);
      self.properties.push(property);
    }
    self
  }

  /// Assigns the pixel font size.
  #[must_use]
  pub fn font_size(mut self, pixels: f32) -> Self {
    let property = self
      .writer
      .ui_pixel_property(ui_wire::UiPropertyKey::StyleFontSize, pixels);
    self.properties.push(property);
    self
  }

  /// Assigns the foreground color.
  #[must_use]
  pub fn color(mut self, value: [f64; 4]) -> Self {
    let property = self
      .writer
      .ui_color_property(ui_wire::UiPropertyKey::StyleColor, value);
    self.properties.push(property);
    self
  }

  /// Assigns the background color.
  #[must_use]
  pub fn background_color(mut self, value: [f64; 4]) -> Self {
    let property = self
      .writer
      .ui_color_property(ui_wire::UiPropertyKey::StyleBackgroundColor, value);
    self.properties.push(property);
    self
  }

  /// Assigns one pixel radius to every border corner.
  #[must_use]
  pub fn border_radius(mut self, pixels: f32) -> Self {
    for key in [
      ui_wire::UiPropertyKey::StyleBorderTopLeftRadius,
      ui_wire::UiPropertyKey::StyleBorderTopRightRadius,
      ui_wire::UiPropertyKey::StyleBorderBottomRightRadius,
      ui_wire::UiPropertyKey::StyleBorderBottomLeftRadius,
    ] {
      let property = self.writer.ui_pixel_property(key, pixels);
      self.properties.push(property);
    }
    self
  }

  /// Assigns one color to every border edge.
  #[must_use]
  pub fn border_color(mut self, value: [f64; 4]) -> Self {
    for key in [
      ui_wire::UiPropertyKey::StyleBorderTopColor,
      ui_wire::UiPropertyKey::StyleBorderRightColor,
      ui_wire::UiPropertyKey::StyleBorderBottomColor,
      ui_wire::UiPropertyKey::StyleBorderLeftColor,
    ] {
      let property = self.writer.ui_color_property(key, value);
      self.properties.push(property);
    }
    self
  }

  /// Assigns one pixel width to every border edge.
  #[must_use]
  pub fn border_width(mut self, pixels: f32) -> Self {
    for key in [
      ui_wire::UiPropertyKey::StyleBorderTopWidth,
      ui_wire::UiPropertyKey::StyleBorderRightWidth,
      ui_wire::UiPropertyKey::StyleBorderBottomWidth,
      ui_wire::UiPropertyKey::StyleBorderLeftWidth,
    ] {
      let property = self.writer.ui_float_property(key, pixels);
      self.properties.push(property);
    }
    self
  }

  /// Centers text horizontally and vertically.
  #[must_use]
  pub fn text_align_middle_center(mut self) -> Self {
    let property = self.writer.ui_enum_property(
      ui_wire::UiPropertyKey::StyleUnityTextAlign,
      ui_wire::UiEnumCatalog::TextAnchor,
      4,
    );
    self.properties.push(property);
    self
  }

  /// Completes this declaration and returns an offset owned by the writer.
  #[must_use]
  pub fn finish(self) -> UiElementOffset {
    let builder_id = self.writer.builder_id;
    let value = self.writer.ui_element(self.kind, &self.properties).value;
    UiElementOffset { builder_id, value }
  }
}

/// A typed flattened UI-node offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct UiNodeOffset {
  builder_id: u64,
  value: WIPOffset<ui_wire::UiNode<'static>>,
}

/// A typed UI-document offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct UiDocumentOffset {
  builder_id: u64,
  value: WIPOffset<ui_wire::UiDocument<'static>>,
}

/// A typed parallel-group offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct ParallelGroupOffset {
  builder_id: u64,
  value: WIPOffset<wire::ParallelCommandGroup<'static>>,
}

/// A typed response-message offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct ResponseMessageOffset {
  builder_id: u64,
  value: WIPOffset<wire::ResponseMessageEntry<'static>>,
}

/// A typed prepared-asset offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct PreparedAssetOffset {
  builder_id: u64,
  value: WIPOffset<world_wire::PreparedAsset<'static>>,
}

/// A typed scene offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct SceneOffset {
  builder_id: u64,
  value: WIPOffset<world_wire::Scene<'static>>,
}

/// A typed game-object offset owned by one [`MessageWriter`].
#[derive(Clone, Copy)]
pub struct GameObjectOffset {
  builder_id: u64,
  value: WIPOffset<world_wire::GameObject<'static>>,
}

/// Asset kind used by direct snapshot construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePreparedAssetKind {
  /// Loadable scene.
  Scene,
  /// Prefab asset.
  Prefab,
  /// Particle-system prefab asset.
  ParticleEffect,
  /// Renderer or UI material.
  Material,
  /// Texture asset.
  Texture,
  /// Audio clip asset.
  AudioClip,
  /// TextMesh Pro font asset.
  TextMeshProFont,
}

/// Image scaling mode used by direct snapshot image objects.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NativeImageFit {
  /// Stretch to the authored dimensions.
  #[default]
  Stretch,
  /// Preserve aspect ratio inside the authored dimensions.
  Contain,
}

/// Scene ownership used by a directly constructed game object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeParentScene {
  /// The snapshot's primary scene.
  Primary,
  /// One explicitly identified scene.
  Scene([u8; 16]),
  /// Unity's persistent scene.
  Persistent,
}

/// Pointer subscription used by a directly constructed game object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePointerEvent {
  /// Pointer-enter events.
  Enter,
  /// Pointer-exit events.
  Exit,
  /// Pointer-down events.
  Down,
  /// Pointer-up events.
  Up,
  /// Pointer-click events.
  Click,
}

/// A schema-stable physical-key ordinal used by direct snapshot input setup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativePhysicalKey(u16);

#[allow(missing_docs)]
impl NativePhysicalKey {
  pub const ESCAPE: Self = Self(0);
  pub const MINUS: Self = Self(24);
  pub const EQUAL: Self = Self(25);
  pub const KEY_L: Self = Self(39);
  pub const KEY_R: Self = Self(45);
  pub const ENTER: Self = Self(60);
  pub const SHIFT_LEFT: Self = Self(61);
  pub const SHIFT_RIGHT: Self = Self(62);
  pub const CONTROL_LEFT: Self = Self(63);
  pub const CONTROL_RIGHT: Self = Self(64);
  pub const META_LEFT: Self = Self(67);
  pub const META_RIGHT: Self = Self(68);
  pub const SPACE: Self = Self(72);
  pub const ARROW_LEFT: Self = Self(80);
  pub const ARROW_RIGHT: Self = Self(81);
  pub const ARROW_UP: Self = Self(82);
  pub const ARROW_DOWN: Self = Self(83);
  pub const NUMPAD_ENTER: Self = Self(103);
}

/// A schema-stable controller-button ordinal used by direct snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeControllerButton(u8);

#[allow(missing_docs)]
impl NativeControllerButton {
  pub const SOUTH: Self = Self(0);
  pub const EAST: Self = Self(1);
  pub const LEFT_SHOULDER: Self = Self(4);
  pub const RIGHT_SHOULDER: Self = Self(5);
  pub const START: Self = Self(8);
}

/// Direct snapshot controller-input declaration.
#[derive(Clone, Copy, Debug)]
pub struct NativeControllerInput<'a> {
  /// Subscribed controller buttons.
  pub buttons: &'a [NativeControllerButton],
  /// Whether directional navigation is enabled.
  pub navigation_enabled: bool,
  /// Optional normalized stick dead zone.
  pub stick_dead_zone: Option<f64>,
  /// Optional initial repeat delay.
  pub repeat_delay_ms: Option<u64>,
  /// Optional subsequent repeat interval.
  pub repeat_interval_ms: Option<u64>,
}

/// Input state authored directly into a snapshot.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeSnapshotInput<'a> {
  /// Whether gameplay input starts disabled.
  pub disabled: bool,
  /// Globally subscribed physical keys.
  pub global_keys: &'a [NativePhysicalKey],
  /// Optional controller input declaration.
  pub controller: Option<NativeControllerInput<'a>>,
}

/// Drag behavior used by a directly constructed game object.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NativeDragMode {
  /// Dragging is disabled.
  #[default]
  None,
  /// Snap the object origin to the pointer.
  SnapToPointer,
  /// Preserve the initial pointer offset.
  PreserveOffset,
}

/// Easing supported by direct finite position tweens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeEasing {
  /// Symmetric sinusoidal ease-in/ease-out.
  InOutSine,
}

/// Dense transform values written directly into a snapshot struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativeTransform {
  /// Local XYZ position.
  pub position: [f64; 3],
  /// Local XYZW rotation.
  pub rotation: [f64; 4],
  /// Local XYZ scale.
  pub scale: [f64; 3],
}

impl Default for NativeTransform {
  fn default() -> Self {
    Self {
      position: [0.0; 3],
      rotation: [0.0, 0.0, 0.0, 1.0],
      scale: [1.0; 3],
    }
  }
}

/// Shared placement fields for direct snapshot objects.
#[derive(Clone, Copy, Debug)]
pub struct NativeObjectPlacement<'a> {
  /// Scene containing the object.
  pub parent_scene: NativeParentScene,
  /// Optional parent game-object UUID.
  pub parent_id: Option<[u8; 16]>,
  /// Initial active state.
  pub active: bool,
  /// Initial local transform.
  pub transform: NativeTransform,
  /// Subscribed pointer events.
  pub pointer_events: &'a [NativePointerEvent],
  /// Initial dragging policy.
  pub drag_mode: NativeDragMode,
}

impl Default for NativeObjectPlacement<'_> {
  fn default() -> Self {
    Self {
      parent_scene: NativeParentScene::Primary,
      parent_id: None,
      active: true,
      transform: NativeTransform::default(),
      pointer_events: &[],
      drag_mode: NativeDragMode::None,
    }
  }
}

/// Constructs a core response directly into one FlatBuffer allocation.
///
/// Every returned offset is tagged with this writer's identity. Composition
/// checks that identity before passing an offset to FlatBuffers, preventing an
/// offset from another builder or an earlier generation from being reused.
pub struct MessageWriter {
  builder_id: u64,
  builder: FlatBufferBuilder<'static>,
  initial_allocation_bytes: usize,
}

impl Default for MessageWriter {
  fn default() -> Self {
    Self::with_capacity(4 * 1024)
  }
}

impl MessageWriter {
  /// Writes one validated reconciliation command directly into this response.
  pub fn command(
    &mut self,
    command: &battlement::Command,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    use battlement::Validate;

    command
      .validate()
      .map_err(|failure| ProtocolError::new(format!("invalid response command: {failure}")))?;
    let value = crate::response::write_command(&mut self.builder, command)?;
    Ok(CoreCommandOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes one validated application snapshot directly into this response.
  pub fn snapshot_message(
    &mut self,
    snapshot: &battlement::Snapshot,
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    use battlement::Validate;

    snapshot
      .validate()
      .map_err(|failure| ProtocolError::new(format!("invalid response snapshot: {failure}")))?;
    let snapshot = crate::response::write_snapshot(&mut self.builder, snapshot)?;
    let value = wire::ResponseMessageEntry::create(
      &mut self.builder,
      &wire::ResponseMessageEntryArgs {
        message_type: wire::ResponseMessage::Snapshot,
        message: Some(snapshot.as_union_value()),
      },
    );
    Ok(ResponseMessageOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Sets one diagnostics metadata entry.
  pub fn set_diagnostics_metadata(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    key: &str,
    value: &str,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    if key.is_empty() {
      return Err(ProtocolError::new("diagnostics metadata key is empty"));
    }
    let key = self.builder.create_string(key);
    let value = self.builder.create_string(value);
    let payload = command_wire::DiagnosticsPayload::create(
      &mut self.builder,
      &command_wire::DiagnosticsPayloadArgs {
        key: Some(key),
        value: Some(value),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::Diagnostics,
      wire::CoreCommandPayload::DiagnosticsPayload,
      payload.as_union_value(),
    )
  }

  /// Opens an absolute external URL on the host platform.
  pub fn open_external_url(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    url: &str,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    let url = self.builder.create_string(url);
    let payload = command_wire::ExternalUrlPayload::create(
      &mut self.builder,
      &command_wire::ExternalUrlPayloadArgs { url: Some(url) },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ApplicationOpenUrl,
      wire::CoreCommandPayload::ExternalUrlPayload,
      payload.as_union_value(),
    )
  }

  /// Creates an exclusive writer with bounded initial capacity.
  #[must_use]
  pub fn with_capacity(capacity: usize) -> Self {
    assert!(
      capacity <= MAXIMUM_BUILDER_BYTES,
      "message writer capacity exceeds 32 MiB"
    );
    let storage = take_message_storage(capacity);
    let initial_allocation_bytes = storage.len();
    Self {
      builder_id: next_builder_id(),
      builder: FlatBufferBuilder::from_vec(storage),
      initial_allocation_bytes,
    }
  }

  /// Writes one prepared-asset catalog entry directly into a snapshot.
  pub fn prepared_asset(
    &mut self,
    kind: NativePreparedAssetKind,
    address: &str,
  ) -> PreparedAssetOffset {
    let address = self.builder.create_string(address);
    let value = world_wire::PreparedAsset::create(
      &mut self.builder,
      &world_wire::PreparedAssetArgs {
        kind: match kind {
          NativePreparedAssetKind::Scene => world_wire::PreparedAssetKind::Scene,
          NativePreparedAssetKind::Prefab => world_wire::PreparedAssetKind::Prefab,
          NativePreparedAssetKind::ParticleEffect => world_wire::PreparedAssetKind::ParticleEffect,
          NativePreparedAssetKind::Material => world_wire::PreparedAssetKind::Material,
          NativePreparedAssetKind::Texture => world_wire::PreparedAssetKind::Texture,
          NativePreparedAssetKind::AudioClip => world_wire::PreparedAssetKind::AudioClip,
          NativePreparedAssetKind::TextMeshProFont => {
            world_wire::PreparedAssetKind::TextMeshProFont
          }
        },
        address: Some(address),
      },
    );
    PreparedAssetOffset {
      builder_id: self.builder_id,
      value,
    }
  }

  /// Writes a default screen-space UI-document host object directly.
  pub fn ui_document_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    root_id: [u8; 16],
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_uuid(root_id, "UI document root")?;
    let filters = self.builder.create_vector(&[
      world_wire::DynamicAtlasFilter::Readability,
      world_wire::DynamicAtlasFilter::Size,
      world_wire::DynamicAtlasFilter::Format,
      world_wire::DynamicAtlasFilter::ColorSpace,
      world_wire::DynamicAtlasFilter::FilterMode,
    ]);
    let atlas = world_wire::DynamicAtlasSettings::create(
      &mut self.builder,
      &world_wire::DynamicAtlasSettingsArgs {
        filters: Some(filters),
        ..Default::default()
      },
    );
    let reference_resolution = common::ScreenSize::new(1200, 800);
    let clear = common::RgbaColor::new(0.0, 0.0, 0.0, 0.0);
    let panel = world_wire::PanelSettings::create(
      &mut self.builder,
      &world_wire::PanelSettingsArgs {
        scale_mode: world_wire::PanelScaleMode::ConstantPixelSize,
        scale: 1.0,
        reference_resolution: Some(&reference_resolution),
        color_clear_value: Some(&clear),
        dynamic_atlas: Some(atlas),
        ..Default::default()
      },
    );
    let root_id = common::Uuid::new(&root_id);
    let world_size = common::ScreenSize::new(1920, 1080);
    let content = world_wire::UiDocumentObject::create(
      &mut self.builder,
      &world_wire::UiDocumentObjectArgs {
        root_id: Some(&root_id),
        panel_settings: Some(panel),
        world_space_size: Some(&world_size),
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::UiDocument,
      world_wire::GameObjectContent::UiDocumentObject,
      content.as_union_value(),
    )
  }

  /// Writes one scene catalog entry directly into a snapshot.
  pub fn scene(&mut self, scene_id: [u8; 16], address: &str) -> Result<SceneOffset, ProtocolError> {
    require_uuid(scene_id, "scene")?;
    let address = self.builder.create_string(address);
    let scene_id = common::Uuid::new(&scene_id);
    let value = world_wire::Scene::create(
      &mut self.builder,
      &world_wire::SceneArgs {
        scene_id: Some(&scene_id),
        address: Some(address),
      },
    );
    Ok(SceneOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes an empty game object without constructing an owned game-object graph.
  pub fn empty_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
  ) -> Result<GameObjectOffset, ProtocolError> {
    let content =
      world_wire::EmptyObject::create(&mut self.builder, &world_wire::EmptyObjectArgs::default());
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Empty,
      world_wire::GameObjectContent::EmptyObject,
      content.as_union_value(),
    )
  }

  /// Writes a camera game object without constructing an owned game-object graph.
  #[allow(clippy::too_many_arguments)]
  pub fn camera_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    field_of_view: f64,
    clear_color: [f64; 4],
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_finite(&clear_color, "camera clear color")?;
    if !field_of_view.is_finite() {
      return Err(ProtocolError::new("camera field of view must be finite"));
    }
    let clear_color = common::RgbaColor::new(
      clear_color[0],
      clear_color[1],
      clear_color[2],
      clear_color[3],
    );
    let content = world_wire::CameraObject::create(
      &mut self.builder,
      &world_wire::CameraObjectArgs {
        field_of_view,
        clear_mode: world_wire::CameraClearMode::SolidColor,
        clear_color: Some(&clear_color),
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Camera,
      world_wire::GameObjectContent::CameraObject,
      content.as_union_value(),
    )
  }

  /// Writes an orthographic camera game object directly.
  pub fn orthographic_camera_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    size: f64,
    clear_color: [f64; 4],
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_finite(&clear_color, "camera clear color")?;
    if !size.is_finite() {
      return Err(ProtocolError::new(
        "camera orthographic size must be finite",
      ));
    }
    let clear_color = common::RgbaColor::new(
      clear_color[0],
      clear_color[1],
      clear_color[2],
      clear_color[3],
    );
    let content = world_wire::CameraObject::create(
      &mut self.builder,
      &world_wire::CameraObjectArgs {
        projection: world_wire::CameraProjection::Orthographic,
        orthographic_size: size,
        clear_mode: world_wire::CameraClearMode::SolidColor,
        clear_color: Some(&clear_color),
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Camera,
      world_wire::GameObjectContent::CameraObject,
      content.as_union_value(),
    )
  }

  /// Writes a text game object without constructing an owned game-object graph.
  #[allow(clippy::too_many_arguments)]
  pub fn text_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    text: &str,
    font: &str,
    size: f64,
    wrap_width: Option<f64>,
  ) -> Result<GameObjectOffset, ProtocolError> {
    self.text_object_with_color(object_id, placement, text, font, size, wrap_width, [1.0; 4])
  }

  /// Writes a colored text game object directly.
  #[allow(clippy::too_many_arguments)]
  pub fn text_object_with_color(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    text: &str,
    font: &str,
    size: f64,
    wrap_width: Option<f64>,
    color: [f64; 4],
  ) -> Result<GameObjectOffset, ProtocolError> {
    if !size.is_finite() || wrap_width.is_some_and(|value| !value.is_finite()) {
      return Err(ProtocolError::new("text dimensions must be finite"));
    }
    require_finite(&color, "text color")?;
    let text = self.builder.create_string(text);
    let font = self.builder.create_string(font);
    let color = common::RgbaColor::new(color[0], color[1], color[2], color[3]);
    let content = world_wire::TextObject::create(
      &mut self.builder,
      &world_wire::TextObjectArgs {
        text: Some(text),
        font: Some(font),
        size,
        color: Some(&color),
        horizontal: world_wire::HorizontalAlignment::Center,
        vertical: world_wire::VerticalAlignment::Middle,
        wrap_width,
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Text,
      world_wire::GameObjectContent::TextObject,
      content.as_union_value(),
    )
  }

  /// Writes an image game object directly.
  #[allow(clippy::too_many_arguments)]
  pub fn image_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    texture: &str,
    width: f64,
    height: f64,
    fit: NativeImageFit,
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_finite(&[width, height], "image dimensions")?;
    let texture = self.builder.create_string(texture);
    let tint = common::RgbColor::new(1.0, 1.0, 1.0);
    let content = world_wire::ImageObject::create(
      &mut self.builder,
      &world_wire::ImageObjectArgs {
        texture: Some(texture),
        width,
        height,
        fit: match fit {
          NativeImageFit::Stretch => world_wire::ImageFit::Stretch,
          NativeImageFit::Contain => world_wire::ImageFit::Contain,
        },
        tint: Some(&tint),
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Image,
      world_wire::GameObjectContent::ImageObject,
      content.as_union_value(),
    )
  }

  /// Writes a point-light game object directly.
  pub fn point_light_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    intensity: f64,
    range: f64,
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_finite(&[intensity, range], "light values")?;
    let color = common::RgbaColor::new(1.0, 1.0, 1.0, 1.0);
    let content = world_wire::LightObject::create(
      &mut self.builder,
      &world_wire::LightObjectArgs {
        color: Some(&color),
        intensity,
        range,
        ..Default::default()
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Light,
      world_wire::GameObjectContent::LightObject,
      content.as_union_value(),
    )
  }

  /// Writes a prefab game object, optional animator state, and material assignments directly.
  pub fn prefab_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    address: &str,
    materials: &[(u32, &str)],
    animator_state: Option<&str>,
  ) -> Result<GameObjectOffset, ProtocolError> {
    let address = self.builder.create_string(address);
    let materials = materials
      .iter()
      .map(|(slot, address)| {
        let address = self.builder.create_string(address);
        world_wire::MaterialAssignment::create(
          &mut self.builder,
          &world_wire::MaterialAssignmentArgs {
            slot: *slot,
            address: Some(address),
          },
        )
      })
      .collect::<Vec<_>>();
    let materials = self.builder.create_vector(&materials);
    let animator = animator_state.map(|state| {
      let state = self.builder.create_string(state);
      let bool_parameters = self
        .builder
        .create_vector::<WIPOffset<world_wire::AnimatorBoolParameter<'static>>>(&[]);
      let int_parameters = self
        .builder
        .create_vector::<WIPOffset<world_wire::AnimatorIntParameter<'static>>>(&[]);
      let float_parameters = self
        .builder
        .create_vector::<WIPOffset<world_wire::AnimatorFloatParameter<'static>>>(&[]);
      world_wire::AnimatorState::create(
        &mut self.builder,
        &world_wire::AnimatorStateArgs {
          state: Some(state),
          bool_parameters: Some(bool_parameters),
          int_parameters: Some(int_parameters),
          float_parameters: Some(float_parameters),
          ..Default::default()
        },
      )
    });
    let content = world_wire::PrefabObject::create(
      &mut self.builder,
      &world_wire::PrefabObjectArgs {
        address: Some(address),
        materials: Some(materials),
        animator,
      },
    );
    self.game_object(
      object_id,
      placement,
      world_wire::GameObjectKind::Prefab,
      world_wire::GameObjectContent::PrefabObject,
      content.as_union_value(),
    )
  }

  /// Writes a cube game object and its material assignments directly.
  pub fn cube_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    materials: &[(u32, &str)],
  ) -> Result<GameObjectOffset, ProtocolError> {
    self.primitive_object(
      object_id,
      placement,
      materials,
      world_wire::GameObjectKind::Cube,
    )
  }

  /// Writes a plane game object and its material assignments directly.
  pub fn plane_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    materials: &[(u32, &str)],
  ) -> Result<GameObjectOffset, ProtocolError> {
    self.primitive_object(
      object_id,
      placement,
      materials,
      world_wire::GameObjectKind::Plane,
    )
  }

  fn primitive_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    materials: &[(u32, &str)],
    kind: world_wire::GameObjectKind,
  ) -> Result<GameObjectOffset, ProtocolError> {
    let materials = materials
      .iter()
      .map(|(slot, address)| {
        let address = self.builder.create_string(address);
        world_wire::MaterialAssignment::create(
          &mut self.builder,
          &world_wire::MaterialAssignmentArgs {
            slot: *slot,
            address: Some(address),
          },
        )
      })
      .collect::<Vec<_>>();
    let materials = self.builder.create_vector(&materials);
    let content = world_wire::PrimitiveObject::create(
      &mut self.builder,
      &world_wire::PrimitiveObjectArgs {
        materials: Some(materials),
      },
    );
    self.game_object(
      object_id,
      placement,
      kind,
      world_wire::GameObjectContent::PrimitiveObject,
      content.as_union_value(),
    )
  }

  /// Writes a complete snapshot message from offsets owned by this writer.
  pub fn snapshot(
    &mut self,
    session_id: [u8; 16],
    prepared_assets: &[PreparedAssetOffset],
    scenes: &[SceneOffset],
    primary_scene_id: Option<[u8; 16]>,
    objects: &[GameObjectOffset],
    input_camera_id: Option<[u8; 16]>,
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    self.snapshot_with_ui(
      session_id,
      prepared_assets,
      scenes,
      primary_scene_id,
      objects,
      input_camera_id,
      &[],
    )
  }

  /// Writes a complete snapshot containing flattened UI-document offsets.
  #[allow(clippy::too_many_arguments)]
  pub fn snapshot_with_ui(
    &mut self,
    session_id: [u8; 16],
    prepared_assets: &[PreparedAssetOffset],
    scenes: &[SceneOffset],
    primary_scene_id: Option<[u8; 16]>,
    objects: &[GameObjectOffset],
    input_camera_id: Option<[u8; 16]>,
    ui_documents: &[UiDocumentOffset],
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    self.snapshot_with_configuration(
      session_id,
      prepared_assets,
      scenes,
      primary_scene_id,
      objects,
      input_camera_id,
      ui_documents,
      NativeSnapshotInput::default(),
    )
  }

  /// Writes a complete snapshot with direct global-key and controller setup.
  #[allow(clippy::too_many_arguments)]
  pub fn snapshot_with_configuration(
    &mut self,
    session_id: [u8; 16],
    prepared_assets: &[PreparedAssetOffset],
    scenes: &[SceneOffset],
    primary_scene_id: Option<[u8; 16]>,
    objects: &[GameObjectOffset],
    input_camera_id: Option<[u8; 16]>,
    ui_documents: &[UiDocumentOffset],
    input: NativeSnapshotInput<'_>,
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    require_uuid(session_id, "snapshot session")?;
    if let Some(id) = primary_scene_id {
      require_uuid(id, "primary scene")?;
    }
    if let Some(id) = input_camera_id {
      require_uuid(id, "input camera")?;
    }
    self.require_offsets(
      prepared_assets
        .iter()
        .map(|value| value.builder_id)
        .chain(scenes.iter().map(|value| value.builder_id))
        .chain(objects.iter().map(|value| value.builder_id))
        .chain(ui_documents.iter().map(|value| value.builder_id)),
    )?;
    let prepared_assets = self.builder.create_vector(
      &prepared_assets
        .iter()
        .map(|value| value.value)
        .collect::<Vec<_>>(),
    );
    let scenes = self
      .builder
      .create_vector(&scenes.iter().map(|value| value.value).collect::<Vec<_>>());
    let objects = self
      .builder
      .create_vector(&objects.iter().map(|value| value.value).collect::<Vec<_>>());
    let ui = self.builder.create_vector(
      &ui_documents
        .iter()
        .map(|value| value.value)
        .collect::<Vec<_>>(),
    );
    let global_keys = self.builder.create_vector(
      &input
        .global_keys
        .iter()
        .map(|key| input_wire::PhysicalKey(key.0))
        .collect::<Vec<_>>(),
    );
    let controller_input = input.controller.map(|controller| {
      let buttons = self.builder.create_vector(
        &controller
          .buttons
          .iter()
          .map(|button| client_wire::ControllerButton(button.0))
          .collect::<Vec<_>>(),
      );
      command_wire::ControllerInputSettings::create(
        &mut self.builder,
        &command_wire::ControllerInputSettingsArgs {
          buttons: Some(buttons),
          navigation_enabled: controller.navigation_enabled,
          stick_dead_zone: controller.stick_dead_zone,
          repeat_delay_ms: controller.repeat_delay_ms,
          repeat_interval_ms: controller.repeat_interval_ms,
        },
      )
    });
    let panel_input_configuration = world_wire::PanelInputConfiguration::create(
      &mut self.builder,
      &world_wire::PanelInputConfigurationArgs::default(),
    );
    let session_id = common::Uuid::new(&session_id);
    let primary_scene_id = primary_scene_id.map(|value| common::Uuid::new(&value));
    let input_camera_id = input_camera_id.map(|value| common::Uuid::new(&value));
    let snapshot = wire::Snapshot::create(
      &mut self.builder,
      &wire::SnapshotArgs {
        session_id: Some(&session_id),
        prepared_assets: Some(prepared_assets),
        scenes: Some(scenes),
        primary_scene_id: primary_scene_id.as_ref(),
        objects: Some(objects),
        ui: Some(ui),
        panel_input_configuration: Some(panel_input_configuration),
        input_camera_id: input_camera_id.as_ref(),
        input_disabled: input.disabled,
        global_keys: Some(global_keys),
        controller_input,
      },
    );
    let value = wire::ResponseMessageEntry::create(
      &mut self.builder,
      &wire::ResponseMessageEntryArgs {
        message_type: wire::ResponseMessage::Snapshot,
        message: Some(snapshot.as_union_value()),
      },
    );
    Ok(ResponseMessageOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes a text-content command without constructing a `CommandBody`.
  pub fn text_set_content(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    content: &str,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "text object")?;
    let content = self.builder.create_string(content);
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::TextContentPayload::create(
      &mut self.builder,
      &command_wire::TextContentPayloadArgs {
        object_id: Some(&object_id),
        content: Some(content),
      },
    );
    let command_id = common::Uuid::new(&command_id);
    let value = wire::CoreCommand::create(
      &mut self.builder,
      &wire::CoreCommandArgs {
        command_id: Some(&command_id),
        blocking,
        kind: wire::CoreCommandKind::TextSetContent,
        payload_type: wire::CoreCommandPayload::TextContentPayload,
        payload: Some(payload.as_union_value()),
      },
    );
    Ok(CoreCommandOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes an immediate local-position command from scalar values.
  pub fn set_local_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    position: [f64; 3],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "position object")?;
    if !position.iter().all(|value| value.is_finite()) {
      return Err(ProtocolError::new("position components must be finite"));
    }
    let object_id = common::Uuid::new(&object_id);
    let position = common::Vector3d::new(position[0], position[1], position[2]);
    let payload = command_wire::PositionPayload::create(
      &mut self.builder,
      &command_wire::PositionPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        position: Some(&position),
      },
    );
    let command_id = common::Uuid::new(&command_id);
    let value = wire::CoreCommand::create(
      &mut self.builder,
      &wire::CoreCommandArgs {
        command_id: Some(&command_id),
        blocking,
        kind: wire::CoreCommandKind::TransformSetLocalPosition,
        payload_type: wire::CoreCommandPayload::PositionPayload,
        payload: Some(payload.as_union_value()),
      },
    );
    Ok(CoreCommandOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes an immediate world-position command from scalar values.
  pub fn set_world_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    position: [f64; 3],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "position object")?;
    require_finite(&position, "position components")?;
    let object_id = common::Uuid::new(&object_id);
    let position = common::Vector3d::new(position[0], position[1], position[2]);
    let payload = command_wire::PositionPayload::create(
      &mut self.builder,
      &command_wire::PositionPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        position: Some(&position),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::TransformSetWorldPosition,
      wire::CoreCommandPayload::PositionPayload,
      payload.as_union_value(),
    )
  }

  /// Writes an immediate local-rotation command from scalar values.
  pub fn set_local_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.set_rotation(
      command_id,
      blocking,
      object_id,
      rotation,
      wire::CoreCommandKind::TransformSetLocalRotation,
    )
  }

  /// Writes an immediate world-rotation command from scalar values.
  pub fn set_world_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.set_rotation(
      command_id,
      blocking,
      object_id,
      rotation,
      wire::CoreCommandKind::TransformSetWorldRotation,
    )
  }

  /// Writes an immediate local-scale command from scalar values.
  pub fn set_local_scale(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    scale: [f64; 3],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "scale object")?;
    require_finite(&scale, "scale components")?;
    let object_id = common::Uuid::new(&object_id);
    let scale = common::Vector3d::new(scale[0], scale[1], scale[2]);
    let payload = command_wire::ScalePayload::create(
      &mut self.builder,
      &command_wire::ScalePayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        scale: Some(&scale),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::TransformSetLocalScale,
      wire::CoreCommandPayload::ScalePayload,
      payload.as_union_value(),
    )
  }

  /// Writes a finite local-position tween directly into the response builder.
  #[allow(clippy::too_many_arguments)]
  pub fn tween_local_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    position: [f64; 3],
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "position object")?;
    require_finite(&position, "position components")?;
    let object_id = common::Uuid::new(&object_id);
    let position = common::Vector3d::new(position[0], position[1], position[2]);
    let tween = command_wire::Tween::new(
      0,
      duration_ms,
      match easing {
        NativeEasing::InOutSine => command_wire::Easing::InOutSine,
      },
      command_wire::TweenRepeatKind::Once,
      0,
      command_wire::RepeatMode::Restart,
    );
    let payload = command_wire::TweenPositionPayload::create(
      &mut self.builder,
      &command_wire::TweenPositionPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        position: Some(&position),
        tween: Some(&tween),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::TransformTweenLocalPosition,
      wire::CoreCommandPayload::TweenPositionPayload,
      payload.as_union_value(),
    )
  }

  /// Writes a finite world-position tween directly into the response builder.
  pub fn tween_world_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    position: [f64; 3],
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.tween_position(
      command_id,
      blocking,
      object_id,
      position,
      duration_ms,
      easing,
      wire::CoreCommandKind::TransformTweenWorldPosition,
    )
  }

  /// Writes a finite local-rotation tween directly into the response builder.
  pub fn tween_local_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.tween_rotation(
      command_id,
      blocking,
      object_id,
      rotation,
      duration_ms,
      easing,
      wire::CoreCommandKind::TransformTweenLocalRotation,
    )
  }

  /// Writes a finite world-rotation tween directly into the response builder.
  pub fn tween_world_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.tween_rotation(
      command_id,
      blocking,
      object_id,
      rotation,
      duration_ms,
      easing,
      wire::CoreCommandKind::TransformTweenWorldRotation,
    )
  }

  /// Writes a finite local-scale tween directly into the response builder.
  pub fn tween_local_scale(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    scale: [f64; 3],
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "scale object")?;
    require_finite(&scale, "scale components")?;
    let object_id = common::Uuid::new(&object_id);
    let scale = common::Vector3d::new(scale[0], scale[1], scale[2]);
    let tween = finite_tween(duration_ms, easing);
    let payload = command_wire::TweenScalePayload::create(
      &mut self.builder,
      &command_wire::TweenScalePayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        scale: Some(&scale),
        tween: Some(&tween),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::TransformTweenLocalScale,
      wire::CoreCommandPayload::TweenScalePayload,
      payload.as_union_value(),
    )
  }

  /// Writes a renderer material assignment directly into the response builder.
  pub fn set_material(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    address: &str,
    slot: Option<u32>,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "renderer object")?;
    let address = self.builder.create_string(address);
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::SetMaterialPayload::create(
      &mut self.builder,
      &command_wire::SetMaterialPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        address: Some(address),
        slot,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::RendererSetMaterial,
      wire::CoreCommandPayload::SetMaterialPayload,
      payload.as_union_value(),
    )
  }

  /// Additively loads one prepared content scene.
  pub fn load_scene(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    scene_id: [u8; 16],
    address: &str,
    make_primary: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(scene_id, "scene")?;
    let address = self.builder.create_string(address);
    let scene_id = common::Uuid::new(&scene_id);
    let payload = command_wire::SceneLoadPayload::create(
      &mut self.builder,
      &command_wire::SceneLoadPayloadArgs {
        scene_id: Some(&scene_id),
        address: Some(address),
        make_primary,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::SceneLoad,
      wire::CoreCommandPayload::SceneLoadPayload,
      payload.as_union_value(),
    )
  }

  /// Unloads a non-primary content scene.
  pub fn unload_scene(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    scene_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.scene_id_command(
      command_id,
      blocking,
      scene_id,
      wire::CoreCommandKind::SceneUnload,
    )
  }

  /// Selects a loaded scene as the primary content scene.
  pub fn set_primary_scene(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    scene_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.scene_id_command(
      command_id,
      blocking,
      scene_id,
      wire::CoreCommandKind::SceneSetPrimary,
    )
  }

  /// Writes a game-object creation command around an object offset from this writer.
  pub fn create_object(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object: GameObjectOffset,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.require_offsets([object.builder_id])?;
    let payload = command_wire::ObjectCreatePayload::create(
      &mut self.builder,
      &command_wire::ObjectCreatePayloadArgs {
        object: Some(object.value),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ObjectCreate,
      wire::CoreCommandPayload::ObjectCreatePayload,
      payload.as_union_value(),
    )
  }

  /// Writes a game-object destruction command.
  pub fn destroy_object(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "destroyed object")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::ObjectIdPayload::create(
      &mut self.builder,
      &command_wire::ObjectIdPayloadArgs {
        object_id: Some(&object_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ObjectDestroy,
      wire::CoreCommandPayload::ObjectIdPayload,
      payload.as_union_value(),
    )
  }

  /// Writes an immediate game-object active-state command.
  pub fn set_object_active(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    active: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "active object")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::ObjectSetActivePayload::create(
      &mut self.builder,
      &command_wire::ObjectSetActivePayloadArgs {
        object_id: Some(&object_id),
        active,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ObjectSetActive,
      wire::CoreCommandPayload::ObjectSetActivePayload,
      payload.as_union_value(),
    )
  }

  /// Writes an immediate object-reparent command.
  pub fn reparent_object(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    parent_id: Option<[u8; 16]>,
    world_position_stays: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "reparented object")?;
    if let Some(parent_id) = parent_id {
      require_uuid(parent_id, "new object parent")?;
      if parent_id == object_id {
        return Err(ProtocolError::new("an object cannot be parented to itself"));
      }
    }
    let object_id = common::Uuid::new(&object_id);
    let parent_id = parent_id.map(|value| common::Uuid::new(&value));
    let payload = command_wire::ObjectReparentPayload::create(
      &mut self.builder,
      &command_wire::ObjectReparentPayloadArgs {
        object_id: Some(&object_id),
        parent_id: parent_id.as_ref(),
        world_position_stays,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ObjectReparent,
      wire::CoreCommandPayload::ObjectReparentPayload,
      payload.as_union_value(),
    )
  }

  /// Writes a finite particle effect spawned at a game object's current position.
  pub fn spawn_particle_at_object(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    address: &str,
    object_id: [u8; 16],
    lifetime_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "particle target")?;
    require_positive_duration(lifetime_ms, "particle lifetime")?;
    let address = self.builder.create_string(address);
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::ParticleSpawnPayload::create(
      &mut self.builder,
      &command_wire::ParticleSpawnPayloadArgs {
        address: Some(address),
        location_kind: command_wire::ParticleSpawnLocationKind::GameObject,
        object_id: Some(&object_id),
        world_position: None,
        lifetime_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ParticleSpawn,
      wire::CoreCommandPayload::ParticleSpawnPayload,
      payload.as_union_value(),
    )
  }

  /// Writes a finite particle effect spawned at a world position.
  pub fn spawn_particle_at_world_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    address: &str,
    position: [f64; 3],
    lifetime_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_finite(&position, "particle world position")?;
    require_positive_duration(lifetime_ms, "particle lifetime")?;
    let address = self.builder.create_string(address);
    let position = common::Vector3d::new(position[0], position[1], position[2]);
    let payload = command_wire::ParticleSpawnPayload::create(
      &mut self.builder,
      &command_wire::ParticleSpawnPayloadArgs {
        address: Some(address),
        location_kind: command_wire::ParticleSpawnLocationKind::WorldPosition,
        object_id: None,
        world_position: Some(&position),
        lifetime_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ParticleSpawn,
      wire::CoreCommandPayload::ParticleSpawnPayload,
      payload.as_union_value(),
    )
  }

  /// Writes audio playback with finite gain, pitch, and fade-in values.
  #[allow(clippy::too_many_arguments)]
  pub fn play_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    address: &str,
    volume: f64,
    pitch: f64,
    loop_: bool,
    fade_in_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_finite(&[volume, pitch], "audio volume and pitch")?;
    if !(0.0..=1.0).contains(&volume) {
      return Err(ProtocolError::new(
        "audio volume must be between zero and one",
      ));
    }
    if !(0.0 < pitch && pitch <= 3.0) {
      return Err(ProtocolError::new(
        "audio pitch must be greater than zero and at most three",
      ));
    }
    require_duration(fade_in_ms, "audio fade-in")?;
    if blocking && loop_ {
      return Err(ProtocolError::new("looping audio must be nonblocking"));
    }
    let address = self.builder.create_string(address);
    let payload = command_wire::AudioPlayPayload::create(
      &mut self.builder,
      &command_wire::AudioPlayPayloadArgs {
        address: Some(address),
        volume,
        pitch,
        loop_,
        fade_in_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioPlay,
      wire::CoreCommandPayload::AudioPlayPayload,
      payload.as_union_value(),
    )
  }

  /// Starts or restarts every particle system rooted at an existing object.
  pub fn play_particles(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    restart: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    if blocking {
      return Err(ProtocolError::new("particle play must be nonblocking"));
    }
    require_uuid(object_id, "particle object")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::ParticlePlayPayload::create(
      &mut self.builder,
      &command_wire::ParticlePlayPayloadArgs {
        object_id: Some(&object_id),
        restart,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ParticlePlay,
      wire::CoreCommandPayload::ParticlePlayPayload,
      payload.as_union_value(),
    )
  }

  /// Stops every particle system rooted at an existing object.
  pub fn stop_particles(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    clear: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "particle object")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = command_wire::ParticleStopPayload::create(
      &mut self.builder,
      &command_wire::ParticleStopPayloadArgs {
        object_id: Some(&object_id),
        clear,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ParticleStop,
      wire::CoreCommandPayload::ParticleStopPayload,
      payload.as_union_value(),
    )
  }

  /// Stops a playing audio command, optionally fading it out.
  pub fn stop_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    fade_out_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    require_duration(fade_out_ms, "audio fade-out")?;
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioStopPayload::create(
      &mut self.builder,
      &command_wire::AudioStopPayloadArgs {
        audio_command_id: Some(&audio_command_id),
        fade_out_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioStop,
      wire::CoreCommandPayload::AudioStopPayload,
      payload.as_union_value(),
    )
  }

  /// Pauses a playing audio command.
  pub fn pause_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.audio_playback_control(
      command_id,
      blocking,
      audio_command_id,
      wire::CoreCommandKind::AudioPause,
    )
  }

  /// Resumes a paused audio command.
  pub fn resume_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.audio_playback_control(
      command_id,
      blocking,
      audio_command_id,
      wire::CoreCommandKind::AudioResume,
    )
  }

  /// Seeks a playing audio command to a position.
  pub fn seek_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    position_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    if position_ms > 922_337_203_685 {
      return Err(ProtocolError::new(
        "audio seek position exceeds the supported range",
      ));
    }
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioSeekPayload::create(
      &mut self.builder,
      &command_wire::AudioSeekPayloadArgs {
        audio_command_id: Some(&audio_command_id),
        position_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioSeek,
      wire::CoreCommandPayload::AudioSeekPayload,
      payload.as_union_value(),
    )
  }

  /// Enables or disables buffering for a playing audio command.
  pub fn set_audio_buffering(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    buffering: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioBufferingPayload::create(
      &mut self.builder,
      &command_wire::AudioBufferingPayloadArgs {
        audio_command_id: Some(&audio_command_id),
        buffering,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioSetBuffering,
      wire::CoreCommandPayload::AudioBufferingPayload,
      payload.as_union_value(),
    )
  }

  /// Replaces the clip used by a playing audio command.
  pub fn replace_audio(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    address: &str,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    let address = self.builder.create_string(address);
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioReplacePayload::create(
      &mut self.builder,
      &command_wire::AudioReplacePayloadArgs {
        audio_command_id: Some(&audio_command_id),
        address: Some(address),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioReplace,
      wire::CoreCommandPayload::AudioReplacePayload,
      payload.as_union_value(),
    )
  }

  /// Tweens a playing audio command's gain once with finite timing.
  pub fn tween_audio_volume(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    volume: f64,
    duration_ms: u64,
    easing: NativeEasing,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    require_finite(&[volume], "audio volume")?;
    if !(0.0..=1.0).contains(&volume) {
      return Err(ProtocolError::new(
        "audio volume must be between zero and one",
      ));
    }
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let tween = finite_tween(duration_ms, easing);
    let payload = command_wire::TweenAudioVolumePayload::create(
      &mut self.builder,
      &command_wire::TweenAudioVolumePayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        audio_command_id: Some(&audio_command_id),
        volume,
        tween: Some(&tween),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioTweenVolume,
      wire::CoreCommandPayload::TweenAudioVolumePayload,
      payload.as_union_value(),
    )
  }

  /// Sets the gain of a playing audio command immediately.
  pub fn set_audio_volume(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    volume: f64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    require_finite(&[volume], "audio volume")?;
    if !(0.0..=1.0).contains(&volume) {
      return Err(ProtocolError::new(
        "audio volume must be between zero and one",
      ));
    }
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioVolumePayload::create(
      &mut self.builder,
      &command_wire::AudioVolumePayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        audio_command_id: Some(&audio_command_id),
        volume,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::AudioSetVolume,
      wire::CoreCommandPayload::AudioVolumePayload,
      payload.as_union_value(),
    )
  }

  /// Writes a positive blocking delay.
  pub fn wait(
    &mut self,
    command_id: [u8; 16],
    duration_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_positive_duration(duration_ms, "wait duration")?;
    let payload = command_wire::WaitPayload::create(
      &mut self.builder,
      &command_wire::WaitPayloadArgs { duration_ms },
    );
    self.core_command(
      command_id,
      true,
      wire::CoreCommandKind::TimeWait,
      wire::CoreCommandPayload::WaitPayload,
      payload.as_union_value(),
    )
  }

  /// Cancels an earlier command operation.
  pub fn cancel_operation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    canceled_command_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(canceled_command_id, "canceled command")?;
    let canceled_command_id = common::Uuid::new(&canceled_command_id);
    let payload = command_wire::CancelOperationPayload::create(
      &mut self.builder,
      &command_wire::CancelOperationPayloadArgs {
        command_id: Some(&canceled_command_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::OperationCancel,
      wire::CoreCommandPayload::CancelOperationPayload,
      payload.as_union_value(),
    )
  }

  /// Writes a controller-motor vibration request.
  pub fn vibrate_controller(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    low_frequency: f64,
    high_frequency: f64,
    duration_ms: u64,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_finite(
      &[low_frequency, high_frequency],
      "controller motor intensities",
    )?;
    if !(0.0..=1.0).contains(&low_frequency) || !(0.0..=1.0).contains(&high_frequency) {
      return Err(ProtocolError::new(
        "controller motor intensities must be between zero and one",
      ));
    }
    if duration_ms > 922_337_203_685 {
      return Err(ProtocolError::new(
        "controller vibration duration exceeds the supported range",
      ));
    }
    let payload = command_wire::ControllerVibrationPayload::create(
      &mut self.builder,
      &command_wire::ControllerVibrationPayloadArgs {
        low_frequency,
        high_frequency,
        duration_ms,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::ControllerVibrate,
      wire::CoreCommandPayload::ControllerVibrationPayload,
      payload.as_union_value(),
    )
  }

  /// Shows or hides the log viewer (`fps_viewer == false`) or FPS viewer.
  pub fn set_debug_ui(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    fps_viewer: bool,
    visible: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    let payload = command_wire::DebugUiPayload::create(
      &mut self.builder,
      &command_wire::DebugUiPayloadArgs {
        surface: if fps_viewer {
          command_wire::DebugUiSurface::FpsViewer
        } else {
          command_wire::DebugUiSurface::LogViewer
        },
        visible,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::DebugUi,
      wire::CoreCommandPayload::DebugUiPayload,
      payload.as_union_value(),
    )
  }

  /// Writes an input-enabled state command.
  pub fn set_input_enabled(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    enabled: bool,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    let payload = command_wire::SetInputEnabledPayload::create(
      &mut self.builder,
      &command_wire::SetInputEnabledPayloadArgs { enabled },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::InputSetEnabled,
      wire::CoreCommandPayload::SetInputEnabledPayload,
      payload.as_union_value(),
    )
  }

  /// Writes an empty generic visual element directly into this message.
  pub fn visual_element(&mut self) -> UiElementOffset {
    self.ui_element(ui_wire::UiElementKind::VisualElement, &[])
  }

  /// Begins direct construction of a generic visual element.
  pub fn visual_element_builder(&mut self) -> UiElementBuilder<'_> {
    UiElementBuilder {
      writer: self,
      kind: ui_wire::UiElementKind::VisualElement,
      properties: Vec::new(),
    }
  }

  /// Begins direct construction of a box element.
  pub fn box_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Box)
  }

  /// Begins direct construction of a label and writes its text immediately.
  pub fn label_builder(&mut self, text: &str) -> UiElementBuilder<'_> {
    let property = self.ui_text_property(ui_wire::UiPropertyKey::Text, text);
    UiElementBuilder {
      writer: self,
      kind: ui_wire::UiElementKind::Label,
      properties: vec![property],
    }
  }

  /// Begins a sparse label update without assigning its text.
  pub fn label_update_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Label)
  }

  /// Begins a sparse text-field update.
  pub fn text_field_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::TextField)
  }

  /// Begins a sparse toggle update.
  pub fn toggle_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Toggle)
  }

  /// Begins a sparse radio-button update.
  pub fn radio_button_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::RadioButton)
  }

  /// Begins a sparse radio-button-group update.
  pub fn radio_button_group_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::RadioButtonGroup)
  }

  /// Begins a sparse toggle-button-group update.
  pub fn toggle_button_group_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::ToggleButtonGroup)
  }

  /// Begins a sparse dropdown update.
  pub fn dropdown_field_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::DropdownField)
  }

  /// Begins a sparse button update.
  pub fn button_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Button)
  }

  /// Begins a sparse repeat-button update.
  pub fn repeat_button_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::RepeatButton)
  }

  /// Begins a sparse scroller update.
  pub fn scroller_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Scroller)
  }

  /// Begins a sparse float-slider update.
  pub fn slider_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::Slider)
  }

  /// Begins a sparse integer-slider update.
  pub fn slider_int_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::SliderInt)
  }

  /// Begins a sparse min-max-slider update.
  pub fn min_max_slider_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::MinMaxSlider)
  }

  /// Begins a sparse tab-view update.
  pub fn tab_view_builder(&mut self) -> UiElementBuilder<'_> {
    self.ui_element_builder(ui_wire::UiElementKind::TabView)
  }

  fn ui_element_builder(&mut self, kind: ui_wire::UiElementKind) -> UiElementBuilder<'_> {
    UiElementBuilder {
      writer: self,
      kind,
      properties: Vec::new(),
    }
  }

  /// Writes one flattened UI node from offsets owned by this writer.
  pub fn ui_node(
    &mut self,
    object_id: [u8; 16],
    element: UiElementOffset,
    child_ids: &[[u8; 16]],
  ) -> Result<UiNodeOffset, ProtocolError> {
    require_uuid(object_id, "UI node")?;
    self.require_offsets([element.builder_id])?;
    for child_id in child_ids {
      require_uuid(*child_id, "UI child")?;
    }
    let child_ids = child_ids.iter().map(common::Uuid::new).collect::<Vec<_>>();
    let child_ids = self.builder.create_vector(&child_ids);
    let object_id = common::Uuid::new(&object_id);
    let value = ui_wire::UiNode::create(
      &mut self.builder,
      &ui_wire::UiNodeArgs {
        object_id: Some(&object_id),
        element: Some(element.value),
        child_ids: Some(child_ids),
      },
    );
    Ok(UiNodeOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes one flattened UI document from offsets owned by this writer.
  pub fn ui_document(
    &mut self,
    document_id: [u8; 16],
    root_id: [u8; 16],
    root_element: UiElementOffset,
    root_child_ids: &[[u8; 16]],
    nodes: &[UiNodeOffset],
  ) -> Result<UiDocumentOffset, ProtocolError> {
    require_uuid(document_id, "UI document")?;
    require_uuid(root_id, "UI document root")?;
    self.require_offsets(
      std::iter::once(root_element.builder_id).chain(nodes.iter().map(|value| value.builder_id)),
    )?;
    for child_id in root_child_ids {
      require_uuid(*child_id, "UI document root child")?;
    }
    let root_child_ids = root_child_ids
      .iter()
      .map(common::Uuid::new)
      .collect::<Vec<_>>();
    let root_child_ids = self.builder.create_vector(&root_child_ids);
    let nodes = self
      .builder
      .create_vector(&nodes.iter().map(|value| value.value).collect::<Vec<_>>());
    let document_id = common::Uuid::new(&document_id);
    let root_id = common::Uuid::new(&root_id);
    let value = ui_wire::UiDocument::create(
      &mut self.builder,
      &ui_wire::UiDocumentArgs {
        document_id: Some(&document_id),
        root_id: Some(&root_id),
        root_element: Some(root_element.value),
        root_child_ids: Some(root_child_ids),
        nodes: Some(nodes),
      },
    );
    Ok(UiDocumentOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes a complete label declaration directly into this message.
  pub fn label(&mut self, text: &str) -> UiElementOffset {
    let text = self.builder.create_string(text);
    let text = ui_wire::TextPropertyValue::create(
      &mut self.builder,
      &ui_wire::TextPropertyValueArgs { value: Some(text) },
    );
    let property = ui_wire::UiProperty::create(
      &mut self.builder,
      &ui_wire::UiPropertyArgs {
        key: ui_wire::UiPropertyKey::Text,
        state: ui_wire::PropState::Set,
        style_value_kind: ui_wire::StyleValueKind::Value,
        value_type: ui_wire::UiPropertyValue::TextPropertyValue,
        value: Some(text.as_union_value()),
      },
    );
    let properties = self.builder.create_vector(&[property]);
    let event_subscriptions = self
      .builder
      .create_vector::<WIPOffset<ui_wire::UiEventSubscriptionValue<'static>>>(&[]);
    let part_styles = self
      .builder
      .create_vector::<WIPOffset<ui_wire::PartStyle<'static>>>(&[]);
    let value = ui_wire::UiElement::create(
      &mut self.builder,
      &ui_wire::UiElementArgs {
        kind: ui_wire::UiElementKind::Label,
        usage_hints: 0,
        properties: Some(properties),
        event_subscriptions: Some(event_subscriptions),
        part_styles: Some(part_styles),
      },
    );
    UiElementOffset {
      builder_id: self.builder_id,
      value,
    }
  }

  fn ui_element(
    &mut self,
    kind: ui_wire::UiElementKind,
    properties: &[WIPOffset<ui_wire::UiProperty<'static>>],
  ) -> UiElementOffset {
    let properties = self.builder.create_vector(properties);
    let event_subscriptions = self
      .builder
      .create_vector::<WIPOffset<ui_wire::UiEventSubscriptionValue<'static>>>(&[]);
    let part_styles = self
      .builder
      .create_vector::<WIPOffset<ui_wire::PartStyle<'static>>>(&[]);
    let value = ui_wire::UiElement::create(
      &mut self.builder,
      &ui_wire::UiElementArgs {
        kind,
        usage_hints: 0,
        properties: Some(properties),
        event_subscriptions: Some(event_subscriptions),
        part_styles: Some(part_styles),
      },
    );
    UiElementOffset {
      builder_id: self.builder_id,
      value,
    }
  }

  fn ui_text_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    text: &str,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let text = self.builder.create_string(text);
    let value = ui_wire::TextPropertyValue::create(
      &mut self.builder,
      &ui_wire::TextPropertyValueArgs { value: Some(text) },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::TextPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_enum_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    catalog: ui_wire::UiEnumCatalog,
    ordinal: u32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let value = ui_wire::EnumPropertyValue::create(
      &mut self.builder,
      &ui_wire::EnumPropertyValueArgs {
        catalog,
        value: ordinal,
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::EnumPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_bool_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value: bool,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let value = ui_wire::BoolPropertyValue::create(
      &mut self.builder,
      &ui_wire::BoolPropertyValueArgs { value },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::BoolPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_float_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value: f32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    assert!(value.is_finite(), "UI float value must be finite");
    let value = ui_wire::FloatPropertyValue::create(
      &mut self.builder,
      &ui_wire::FloatPropertyValueArgs { value },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::FloatPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_int_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value: i32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let value = ui_wire::IntPropertyValue::create(
      &mut self.builder,
      &ui_wire::IntPropertyValueArgs { value },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::IntPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_uint_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value: u32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let value = ui_wire::UIntPropertyValue::create(
      &mut self.builder,
      &ui_wire::UIntPropertyValueArgs { value },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::UIntPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_optional_uint_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value: Option<u32>,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let value = value.map(|value| {
      ui_wire::UIntPropertyValue::create(
        &mut self.builder,
        &ui_wire::UIntPropertyValueArgs { value },
      )
      .as_union_value()
    });
    ui_wire::UiProperty::create(
      &mut self.builder,
      &ui_wire::UiPropertyArgs {
        key,
        state: ui_wire::PropState::Set,
        style_value_kind: ui_wire::StyleValueKind::Value,
        value_type: if value.is_some() {
          ui_wire::UiPropertyValue::UIntPropertyValue
        } else {
          ui_wire::UiPropertyValue::NONE
        },
        value,
      },
    )
  }

  fn ui_uint_list_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    values: impl IntoIterator<Item = u32>,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    let values = values.into_iter().collect::<Vec<_>>();
    let values = self.builder.create_vector(&values);
    let value = ui_wire::UIntListPropertyValue::create(
      &mut self.builder,
      &ui_wire::UIntListPropertyValueArgs {
        values: Some(values),
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::UIntListPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_choice_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    index: Option<u32>,
    text: Option<&str>,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    assert_eq!(index.is_some(), text.is_some(), "choice fields must match");
    let text = text.map(|value| self.builder.create_string(value));
    let value = ui_wire::ChoicePropertyValue::create(
      &mut self.builder,
      &ui_wire::ChoicePropertyValueArgs {
        kind: if index.is_some() {
          ui_wire::ChoiceKind::Index
        } else {
          ui_wire::ChoiceKind::None
        },
        index: index.unwrap_or_default(),
        value: text,
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::ChoicePropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_pixel_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    pixels: f32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    assert!(pixels.is_finite(), "UI pixel value must be finite");
    let value = ui_wire::LengthPropertyValue::create(
      &mut self.builder,
      &ui_wire::LengthPropertyValueArgs {
        kind: ui_wire::LengthKind::Pixels,
        pixels,
        percentage: 0.0,
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::LengthPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_percentage_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    percentage: f32,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    assert!(percentage.is_finite(), "UI percentage must be finite");
    let value = ui_wire::LengthPropertyValue::create(
      &mut self.builder,
      &ui_wire::LengthPropertyValueArgs {
        kind: ui_wire::LengthKind::Percent,
        pixels: 0.0,
        percentage,
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::LengthPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_color_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    color: [f64; 4],
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    assert!(
      color.iter().all(|component| component.is_finite()),
      "UI color components must be finite"
    );
    let color = common::RgbaColor::new(color[0], color[1], color[2], color[3]);
    let value = ui_wire::ColorPropertyValue::create(
      &mut self.builder,
      &ui_wire::ColorPropertyValueArgs {
        value: Some(&color),
      },
    );
    self.ui_property(
      key,
      ui_wire::UiPropertyValue::ColorPropertyValue,
      value.as_union_value(),
    )
  }

  fn ui_property(
    &mut self,
    key: ui_wire::UiPropertyKey,
    value_type: ui_wire::UiPropertyValue,
    value: WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> WIPOffset<ui_wire::UiProperty<'static>> {
    ui_wire::UiProperty::create(
      &mut self.builder,
      &ui_wire::UiPropertyArgs {
        key,
        state: ui_wire::PropState::Set,
        style_value_kind: ui_wire::StyleValueKind::Value,
        value_type,
        value: Some(value),
      },
    )
  }

  /// Copies one changed text declaration from an immutable retained snapshot
  /// into this response without constructing `Prop<String>` or `UiElement`.
  pub fn retained_label(&mut self, text: RetainedTextPropertyView<'_>) -> UiElementOffset {
    let value = text.value().map(|value| {
      let value = self.builder.create_string(value);
      ui_wire::TextPropertyValue::create(
        &mut self.builder,
        &ui_wire::TextPropertyValueArgs { value: Some(value) },
      )
      .as_union_value()
    });
    let property = ui_wire::UiProperty::create(
      &mut self.builder,
      &ui_wire::UiPropertyArgs {
        key: ui_wire::UiPropertyKey::Text,
        state: text.wire_state(),
        style_value_kind: ui_wire::StyleValueKind::Value,
        value_type: if value.is_some() {
          ui_wire::UiPropertyValue::TextPropertyValue
        } else {
          ui_wire::UiPropertyValue::NONE
        },
        value,
      },
    );
    let properties = self.builder.create_vector(&[property]);
    let subscriptions = self
      .builder
      .create_vector::<WIPOffset<ui_wire::UiEventSubscriptionValue<'static>>>(&[]);
    let part_styles = self
      .builder
      .create_vector::<WIPOffset<ui_wire::PartStyle<'static>>>(&[]);
    let value = ui_wire::UiElement::create(
      &mut self.builder,
      &ui_wire::UiElementArgs {
        kind: ui_wire::UiElementKind::Label,
        usage_hints: 0,
        properties: Some(properties),
        event_subscriptions: Some(subscriptions),
        part_styles: Some(part_styles),
      },
    );
    UiElementOffset {
      builder_id: self.builder_id,
      value,
    }
  }

  /// Writes a sparse visual-element property update without an owned `UiElement`.
  pub fn update_visual_element(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    element: UiElementOffset,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "visual element")?;
    self.require_offsets([element.builder_id])?;
    let object_id = common::Uuid::new(&object_id);
    let payload = ui_wire::VisualElementUpdatePayload::create(
      &mut self.builder,
      &ui_wire::VisualElementUpdatePayloadArgs {
        kind: ui_wire::VisualElementUpdateKind::Properties,
        object_id: Some(&object_id),
        element: Some(element.value),
        parent_id: None,
        child_index: None,
      },
    );
    let command_id = common::Uuid::new(&command_id);
    let value = wire::CoreCommand::create(
      &mut self.builder,
      &wire::CoreCommandArgs {
        command_id: Some(&command_id),
        blocking,
        kind: wire::CoreCommandKind::VisualElementUpdate,
        payload_type: wire::CoreCommandPayload::VisualElementUpdatePayload,
        payload: Some(payload.as_union_value()),
      },
    );
    Ok(CoreCommandOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Moves a visual element to a new sibling index without an element payload.
  pub fn update_visual_element_index(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    child_index: u32,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "visual element")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = ui_wire::VisualElementUpdatePayload::create(
      &mut self.builder,
      &ui_wire::VisualElementUpdatePayloadArgs {
        kind: ui_wire::VisualElementUpdateKind::Index,
        object_id: Some(&object_id),
        element: None,
        parent_id: None,
        child_index: Some(child_index),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::VisualElementUpdate,
      wire::CoreCommandPayload::VisualElementUpdatePayload,
      payload.as_union_value(),
    )
  }

  /// Reparents a visual element without reconstructing its element payload.
  pub fn update_visual_element_parent(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    parent_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    require_uuid(object_id, "visual element")?;
    require_uuid(parent_id, "visual-element parent")?;
    let object_id = common::Uuid::new(&object_id);
    let parent_id = common::Uuid::new(&parent_id);
    let payload = ui_wire::VisualElementUpdatePayload::create(
      &mut self.builder,
      &ui_wire::VisualElementUpdatePayloadArgs {
        kind: ui_wire::VisualElementUpdateKind::Parent,
        object_id: Some(&object_id),
        element: None,
        parent_id: Some(&parent_id),
        child_index: None,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::VisualElementUpdate,
      wire::CoreCommandPayload::VisualElementUpdatePayload,
      payload.as_union_value(),
    )
  }

  /// Creates a flattened UI subtree beneath an existing parent.
  pub fn create_visual_element(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    parent_id: [u8; 16],
    child_index: Option<u32>,
    root_id: [u8; 16],
    nodes: &[UiNodeOffset],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(parent_id, "visual-element parent")?;
    require_uuid(root_id, "visual-element subtree root")?;
    if nodes.is_empty() {
      return Err(ProtocolError::new(
        "a visual-element subtree cannot be empty",
      ));
    }
    self.require_offsets(nodes.iter().map(|node| node.builder_id))?;
    let nodes = self
      .builder
      .create_vector(&nodes.iter().map(|node| node.value).collect::<Vec<_>>());
    let parent_id = common::Uuid::new(&parent_id);
    let root_id = common::Uuid::new(&root_id);
    let payload = ui_wire::VisualElementCreatePayload::create(
      &mut self.builder,
      &ui_wire::VisualElementCreatePayloadArgs {
        parent_id: Some(&parent_id),
        child_index,
        nodes: Some(nodes),
        root_id: Some(&root_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::VisualElementCreate,
      wire::CoreCommandPayload::VisualElementCreatePayload,
      payload.as_union_value(),
    )
  }

  /// Destroys a UI subtree by its root object identity.
  pub fn destroy_visual_element(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "destroyed visual element")?;
    let object_id = common::Uuid::new(&object_id);
    let payload = ui_wire::VisualElementDestroyPayload::create(
      &mut self.builder,
      &ui_wire::VisualElementDestroyPayloadArgs {
        object_id: Some(&object_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::VisualElementDestroy,
      wire::CoreCommandPayload::VisualElementDestroyPayload,
      payload.as_union_value(),
    )
  }

  /// Writes a focus action for one UI element.
  pub fn focus_visual_element(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::Focus,
      0,
      None,
      0,
      0,
    )
  }

  /// Writes a blur action for one UI element.
  pub fn blur_visual_element(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::Blur,
      0,
      None,
      0,
      0,
    )
  }

  /// Writes a pointer-capture action for one UI element.
  pub fn capture_visual_element_pointer(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    pointer_id: i32,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::CapturePointer,
      pointer_id,
      None,
      0,
      0,
    )
  }

  /// Writes a pointer-release action for one UI element.
  pub fn release_visual_element_pointer(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    pointer_id: i32,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::ReleasePointer,
      pointer_id,
      None,
      0,
      0,
    )
  }

  /// Writes a scroll-to-descendant action for one UI element.
  pub fn scroll_visual_element_to(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    descendant_id: [u8; 16],
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(descendant_id, "scroll descendant")?;
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::ScrollTo,
      0,
      Some(descendant_id),
      0,
      0,
    )
  }

  /// Writes a text-selection action for one UI element.
  pub fn select_visual_element_text(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    cursor_index: u32,
    selection_index: u32,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    self.visual_element_action(
      command_id,
      blocking,
      object_id,
      ui_wire::VisualElementActionKind::SelectText,
      0,
      None,
      cursor_index,
      selection_index,
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn visual_element_action(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    kind: ui_wire::VisualElementActionKind,
    pointer_id: i32,
    descendant_id: Option<[u8; 16]>,
    cursor_index: u32,
    selection_index: u32,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "visual element action target")?;
    let object_id = common::Uuid::new(&object_id);
    let descendant_id = descendant_id.map(|value| common::Uuid::new(&value));
    let payload = ui_wire::VisualElementActionPayload::create(
      &mut self.builder,
      &ui_wire::VisualElementActionPayloadArgs {
        object_id: Some(&object_id),
        kind,
        streaks: None,
        pointer_id,
        descendant_id: descendant_id.as_ref(),
        cursor_index,
        selection_index,
      },
    );
    self.core_command(
      command_id,
      blocking,
      wire::CoreCommandKind::VisualElementPerformAction,
      wire::CoreCommandPayload::VisualElementActionPayload,
      payload.as_union_value(),
    )
  }

  /// Wraps core commands in one parallel group.
  pub fn parallel_group(
    &mut self,
    commands: &[CoreCommandOffset],
  ) -> Result<ParallelGroupOffset, ProtocolError> {
    if commands.is_empty() {
      return Err(ProtocolError::new("a command group cannot be empty"));
    }
    self.require_offsets(commands.iter().map(|value| value.builder_id))?;
    let commands = commands
      .iter()
      .map(|command| {
        wire::CommandEntry::create(
          &mut self.builder,
          &wire::CommandEntryArgs {
            command_type: wire::CommandEntryPayload::CoreCommand,
            command: Some(command.value.as_union_value()),
          },
        )
      })
      .collect::<Vec<_>>();
    let commands = self.builder.create_vector(&commands);
    let value = wire::ParallelCommandGroup::create(
      &mut self.builder,
      &wire::ParallelCommandGroupArgs {
        commands: Some(commands),
      },
    );
    Ok(ParallelGroupOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Writes one batch message around previously constructed groups.
  pub fn batch(
    &mut self,
    batch_id: [u8; 16],
    session_id: [u8; 16],
    caused_by_action_id: Option<[u8; 16]>,
    start: NativeBatchStart,
    groups: &[ParallelGroupOffset],
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    self.scoped_batch(
      batch_id,
      session_id,
      caused_by_action_id,
      start,
      None,
      None,
      groups,
    )
  }

  /// Writes owned game work or independent cancellation and destruction cleanup.
  #[allow(clippy::too_many_arguments)]
  pub fn scoped_batch(
    &mut self,
    batch_id: [u8; 16],
    session_id: [u8; 16],
    caused_by_action_id: Option<[u8; 16]>,
    start: NativeBatchStart,
    work_scope: Option<u64>,
    cancel_scope: Option<u64>,
    groups: &[ParallelGroupOffset],
  ) -> Result<ResponseMessageOffset, ProtocolError> {
    require_uuid(batch_id, "batch")?;
    require_uuid(session_id, "batch session")?;
    if let Some(action_id) = caused_by_action_id {
      require_uuid(action_id, "causing action")?;
    }
    if groups.is_empty() && cancel_scope.is_none() {
      return Err(ProtocolError::new("a batch cannot have no command groups"));
    }
    self.require_offsets(groups.iter().map(|value| value.builder_id))?;
    let groups = groups.iter().map(|group| group.value).collect::<Vec<_>>();
    let groups = self.builder.create_vector(&groups);
    let batch_id = common::Uuid::new(&batch_id);
    let session_id = common::Uuid::new(&session_id);
    let caused_by_action_id = caused_by_action_id.map(|value| common::Uuid::new(&value));
    let batch = wire::Batch::create(
      &mut self.builder,
      &wire::BatchArgs {
        batch_id: Some(&batch_id),
        session_id: Some(&session_id),
        caused_by_action_id: caused_by_action_id.as_ref(),
        work_scope,
        cancel_scope,
        start: match start {
          NativeBatchStart::Now => wire::BatchStart::Now,
          NativeBatchStart::AfterEarlierBlockingWork => wire::BatchStart::AfterEarlierBlockingWork,
          NativeBatchStart::AfterEarlierAssetPreparation => {
            wire::BatchStart::AfterEarlierAssetPreparation
          }
        },
        groups: Some(groups),
      },
    );
    let value = wire::ResponseMessageEntry::create(
      &mut self.builder,
      &wire::ResponseMessageEntryArgs {
        message_type: wire::ResponseMessage::Batch,
        message: Some(batch.as_union_value()),
      },
    );
    Ok(ResponseMessageOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  /// Finishes and verifies the response, consuming the writer and its offsets.
  pub fn finish(
    mut self,
    session_id: [u8; 16],
    messages: &[ResponseMessageOffset],
  ) -> Result<FinishedMessage, ProtocolError> {
    require_uuid(session_id, "response session")?;
    self.require_offsets(messages.iter().map(|value| value.builder_id))?;
    let messages = messages
      .iter()
      .map(|message| message.value)
      .collect::<Vec<_>>();
    let messages = self.builder.create_vector(&messages);
    let session_id = common::Uuid::new(&session_id);
    let response = wire::Response::create(
      &mut self.builder,
      &wire::ResponseArgs {
        session_id: Some(&session_id),
        messages: Some(messages),
      },
    );
    wire::finish_size_prefixed_response_buffer(&mut self.builder, response);
    let (storage, start) = self.builder.collapse();
    record_message_growth(self.initial_allocation_bytes, storage.capacity());
    let message = FinishedMessage::from_storage(storage, start);
    if message.as_bytes().len() > MAXIMUM_MESSAGE_BYTES {
      return Err(ProtocolError::new("response exceeds 16 MiB"));
    }
    if message.allocation_bytes() > MAXIMUM_BUILDER_BYTES {
      return Err(ProtocolError::new(
        "response builder allocation exceeds 32 MiB",
      ));
    }
    ResponseView::read(message.as_bytes())?;
    Ok(message)
  }

  fn game_object(
    &mut self,
    object_id: [u8; 16],
    placement: NativeObjectPlacement<'_>,
    kind: world_wire::GameObjectKind,
    content_type: world_wire::GameObjectContent,
    content: WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Result<GameObjectOffset, ProtocolError> {
    require_uuid(object_id, "game object")?;
    if let Some(parent_id) = placement.parent_id {
      require_uuid(parent_id, "parent object")?;
    }
    require_finite(&placement.transform.position, "object position")?;
    require_finite(&placement.transform.rotation, "object rotation")?;
    require_finite(&placement.transform.scale, "object scale")?;
    let scene_id = match placement.parent_scene {
      NativeParentScene::Scene(id) => {
        require_uuid(id, "parent scene")?;
        Some(common::Uuid::new(&id))
      }
      NativeParentScene::Primary | NativeParentScene::Persistent => None,
    };
    let parent_scene = world_wire::ParentScene::create(
      &mut self.builder,
      &world_wire::ParentSceneArgs {
        kind: match placement.parent_scene {
          NativeParentScene::Primary => world_wire::ParentSceneKind::PrimaryScene,
          NativeParentScene::Scene(_) => world_wire::ParentSceneKind::Scene,
          NativeParentScene::Persistent => world_wire::ParentSceneKind::Persistent,
        },
        scene_id: scene_id.as_ref(),
      },
    );
    let pointer_events = placement
      .pointer_events
      .iter()
      .map(|event| match event {
        NativePointerEvent::Enter => world_wire::PointerEventKind::Enter,
        NativePointerEvent::Exit => world_wire::PointerEventKind::Exit,
        NativePointerEvent::Down => world_wire::PointerEventKind::Down,
        NativePointerEvent::Up => world_wire::PointerEventKind::Up,
        NativePointerEvent::Click => world_wire::PointerEventKind::Click,
      })
      .collect::<Vec<_>>();
    let pointer_events = self.builder.create_vector(&pointer_events);
    let position = common::Vector3d::new(
      placement.transform.position[0],
      placement.transform.position[1],
      placement.transform.position[2],
    );
    let rotation = common::Quaterniond::new(
      placement.transform.rotation[0],
      placement.transform.rotation[1],
      placement.transform.rotation[2],
      placement.transform.rotation[3],
    );
    let scale = common::Vector3d::new(
      placement.transform.scale[0],
      placement.transform.scale[1],
      placement.transform.scale[2],
    );
    let local_transform = common::LocalTransform::new(&position, &rotation, &scale);
    let object_id = common::Uuid::new(&object_id);
    let parent_id = placement.parent_id.map(|value| common::Uuid::new(&value));
    let value = world_wire::GameObject::create(
      &mut self.builder,
      &world_wire::GameObjectArgs {
        object_id: Some(&object_id),
        parent_scene: Some(parent_scene),
        parent_id: parent_id.as_ref(),
        active: placement.active,
        local_transform: Some(&local_transform),
        pointer_events: Some(pointer_events),
        drag_mode: match placement.drag_mode {
          NativeDragMode::None => world_wire::DragMode::None,
          NativeDragMode::SnapToPointer => world_wire::DragMode::SnapToPointer,
          NativeDragMode::PreserveOffset => world_wire::DragMode::PreserveOffset,
        },
        kind,
        content_type,
        content: Some(content),
      },
    );
    Ok(GameObjectOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  fn set_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
    kind: wire::CoreCommandKind,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "rotation object")?;
    require_finite(&rotation, "rotation components")?;
    if rotation.iter().all(|component| *component == 0.0) {
      return Err(ProtocolError::new(
        "rotation quaternion must have nonzero length",
      ));
    }
    let object_id = common::Uuid::new(&object_id);
    let rotation = common::Quaterniond::new(rotation[0], rotation[1], rotation[2], rotation[3]);
    let payload = command_wire::RotationPayload::create(
      &mut self.builder,
      &command_wire::RotationPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        rotation: Some(&rotation),
      },
    );
    self.core_command(
      command_id,
      blocking,
      kind,
      wire::CoreCommandPayload::RotationPayload,
      payload.as_union_value(),
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn tween_position(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    position: [f64; 3],
    duration_ms: u64,
    easing: NativeEasing,
    kind: wire::CoreCommandKind,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "position object")?;
    require_finite(&position, "position components")?;
    let object_id = common::Uuid::new(&object_id);
    let position = common::Vector3d::new(position[0], position[1], position[2]);
    let tween = finite_tween(duration_ms, easing);
    let payload = command_wire::TweenPositionPayload::create(
      &mut self.builder,
      &command_wire::TweenPositionPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        position: Some(&position),
        tween: Some(&tween),
      },
    );
    self.core_command(
      command_id,
      blocking,
      kind,
      wire::CoreCommandPayload::TweenPositionPayload,
      payload.as_union_value(),
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn tween_rotation(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    object_id: [u8; 16],
    rotation: [f64; 4],
    duration_ms: u64,
    easing: NativeEasing,
    kind: wire::CoreCommandKind,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(object_id, "rotation object")?;
    require_finite(&rotation, "rotation components")?;
    if rotation.iter().all(|component| *component == 0.0) {
      return Err(ProtocolError::new(
        "rotation quaternion must have nonzero length",
      ));
    }
    let object_id = common::Uuid::new(&object_id);
    let rotation = common::Quaterniond::new(rotation[0], rotation[1], rotation[2], rotation[3]);
    let tween = finite_tween(duration_ms, easing);
    let payload = command_wire::TweenRotationPayload::create(
      &mut self.builder,
      &command_wire::TweenRotationPayloadArgs {
        on_conflict: command_wire::ConflictPolicy::Cancel,
        object_id: Some(&object_id),
        rotation: Some(&rotation),
        tween: Some(&tween),
      },
    );
    self.core_command(
      command_id,
      blocking,
      kind,
      wire::CoreCommandPayload::TweenRotationPayload,
      payload.as_union_value(),
    )
  }

  fn core_command(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    kind: wire::CoreCommandKind,
    payload_type: wire::CoreCommandPayload,
    payload: WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(command_id, "command")?;
    let command_id = common::Uuid::new(&command_id);
    let value = wire::CoreCommand::create(
      &mut self.builder,
      &wire::CoreCommandArgs {
        command_id: Some(&command_id),
        blocking,
        kind,
        payload_type,
        payload: Some(payload),
      },
    );
    Ok(CoreCommandOffset {
      builder_id: self.builder_id,
      value,
    })
  }

  fn audio_playback_control(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    audio_command_id: [u8; 16],
    kind: wire::CoreCommandKind,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(audio_command_id, "audio command")?;
    let audio_command_id = common::Uuid::new(&audio_command_id);
    let payload = command_wire::AudioPlaybackPayload::create(
      &mut self.builder,
      &command_wire::AudioPlaybackPayloadArgs {
        audio_command_id: Some(&audio_command_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      kind,
      wire::CoreCommandPayload::AudioPlaybackPayload,
      payload.as_union_value(),
    )
  }

  fn scene_id_command(
    &mut self,
    command_id: [u8; 16],
    blocking: bool,
    scene_id: [u8; 16],
    kind: wire::CoreCommandKind,
  ) -> Result<CoreCommandOffset, ProtocolError> {
    require_uuid(scene_id, "scene")?;
    let scene_id = common::Uuid::new(&scene_id);
    let payload = command_wire::SceneIdPayload::create(
      &mut self.builder,
      &command_wire::SceneIdPayloadArgs {
        scene_id: Some(&scene_id),
      },
    );
    self.core_command(
      command_id,
      blocking,
      kind,
      wire::CoreCommandPayload::SceneIdPayload,
      payload.as_union_value(),
    )
  }

  fn require_offsets(
    &self,
    builder_ids: impl IntoIterator<Item = u64>,
  ) -> Result<(), ProtocolError> {
    if builder_ids
      .into_iter()
      .any(|builder_id| builder_id != self.builder_id)
    {
      return Err(ProtocolError::new(
        "a FlatBuffer offset belongs to another message writer",
      ));
    }
    Ok(())
  }
}

fn require_uuid(value: [u8; 16], field: &str) -> Result<(), ProtocolError> {
  if value == [0; 16] {
    return Err(ProtocolError::new(format!("{field} UUID is zero")));
  }
  Ok(())
}

fn require_finite(values: &[f64], field: &str) -> Result<(), ProtocolError> {
  if values.iter().all(|value| value.is_finite()) {
    Ok(())
  } else {
    Err(ProtocolError::new(format!("{field} must be finite")))
  }
}

fn require_duration(value_ms: u64, field: &str) -> Result<(), ProtocolError> {
  if value_ms > 86_400_000 {
    return Err(ProtocolError::new(format!("{field} cannot exceed one day")));
  }
  Ok(())
}

fn require_positive_duration(value_ms: u64, field: &str) -> Result<(), ProtocolError> {
  if value_ms == 0 {
    return Err(ProtocolError::new(format!("{field} must be positive")));
  }
  require_duration(value_ms, field)
}

fn finite_tween(duration_ms: u64, easing: NativeEasing) -> command_wire::Tween {
  command_wire::Tween::new(
    0,
    duration_ms,
    match easing {
      NativeEasing::InOutSine => command_wire::Easing::InOutSine,
    },
    command_wire::TweenRepeatKind::Once,
    0,
    command_wire::RepeatMode::Restart,
  )
}

fn next_builder_id() -> u64 {
  loop {
    let id = NEXT_BUILDER_ID.fetch_add(1, Ordering::Relaxed);
    if id != 0 {
      return id;
    }
  }
}

fn record_message_growth(initial: usize, final_allocation: usize) {
  if final_allocation <= initial {
    return;
  }
  let mut allocation = initial;
  let mut growths = 0u64;
  let mut copied_bytes = 0u64;
  while allocation < final_allocation {
    copied_bytes = copied_bytes.saturating_add(allocation as u64);
    allocation = allocation.saturating_mul(2);
    growths += 1;
  }
  MESSAGE_BUILDER_GROWTHS.fetch_add(growths, Ordering::Relaxed);
  MESSAGE_BUILDER_COPIED_BYTES.fetch_add(copied_bytes, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
  use battlement::{UiLabel, UiNode, object_id};

  use super::*;

  #[test]
  fn constructs_a_text_update_without_an_owned_command_graph() {
    let mut writer = MessageWriter::default();
    let command = writer
      .text_set_content([1; 16], true, [2; 16], "ready")
      .unwrap();
    let group = writer.parallel_group(&[command]).unwrap();
    let batch = writer
      .batch(
        [3; 16],
        [4; 16],
        Some([5; 16]),
        NativeBatchStart::Now,
        &[group],
      )
      .unwrap();
    let finished = writer.finish([4; 16], &[batch]).unwrap();
    let response = ResponseView::read(finished.as_bytes()).unwrap();
    assert_eq!(response.session_id(), [4; 16]);
    assert_eq!(response.message_count(), 1);
  }

  #[test]
  fn reuses_released_storage_without_exposing_old_finished_bytes() {
    MESSAGE_BUILDER_POOL.with_borrow_mut(|pool| {
      pool.storage.clear();
      pool.idle_bytes = 0;
    });
    let first = MessageWriter::default()
      .finish([0x91; 16], &[])
      .expect("finish first response");
    let allocation = first.allocation_bytes();
    let (storage, _) = first.into_storage();
    recycle_message_storage(storage);
    assert_eq!(message_writer_diagnostics().idle_bytes, allocation);

    let reused_before = message_writer_diagnostics().reused;
    let second = MessageWriter::default()
      .finish([0x92; 16], &[])
      .expect("finish response in reused storage");
    assert!(message_writer_diagnostics().reused > reused_before);
    assert_eq!(message_writer_diagnostics().idle_bytes, 0);
    let view = ResponseView::read(second.as_bytes()).expect("read reused response");
    assert_eq!(view.session_id(), [0x92; 16]);
  }

  #[test]
  fn constructs_a_sparse_label_update_in_the_response_builder() {
    let mut writer = MessageWriter::default();
    let label = writer.label("ready");
    let command = writer
      .update_visual_element([1; 16], true, [2; 16], label)
      .unwrap();
    let group = writer.parallel_group(&[command]).unwrap();
    let batch = writer
      .batch([3; 16], [4; 16], None, NativeBatchStart::Now, &[group])
      .unwrap();
    let finished = writer.finish([4; 16], &[batch]).unwrap();
    assert_eq!(
      ResponseView::read(finished.as_bytes())
        .unwrap()
        .message_count(),
      1
    );
  }

  #[test]
  fn constructs_typed_control_updates_without_owned_ui_elements() {
    let mut writer = MessageWriter::default();
    let elements = [
      writer.text_field_builder().text_value("新しい").finish(),
      writer.toggle_builder().bool_value(false).finish(),
      writer.radio_button_builder().bool_value(true).finish(),
      writer
        .radio_button_group_builder()
        .selected_index(Some(2))
        .finish(),
      writer
        .toggle_button_group_builder()
        .selected_indices([0, 2])
        .finish(),
      writer
        .dropdown_field_builder()
        .selection(Some(1), Some("Solar"))
        .finish(),
      writer.button_builder().text("Done").enabled(false).finish(),
      writer
        .repeat_button_builder()
        .repeat_timing(200, std::num::NonZeroU32::new(100).unwrap())
        .finish(),
      writer.scroller_builder().float_value(42.5).finish(),
      writer.slider_builder().float_value(12.5).finish(),
      writer.slider_int_builder().int_value(7).finish(),
      writer.min_max_slider_builder().range(2.0, 8.0).finish(),
      writer.tab_view_builder().selected_tab_index(3).finish(),
    ];
    let mut commands = Vec::new();
    for (index, element) in elements.into_iter().enumerate() {
      commands.push(
        writer
          .update_visual_element(
            [(index + 1) as u8; 16],
            false,
            [20 + index as u8; 16],
            element,
          )
          .unwrap(),
      );
    }
    commands.push(
      writer
        .update_visual_element_index([40; 16], false, [41; 16], 3)
        .unwrap(),
    );
    commands.push(
      writer
        .update_visual_element_parent([44; 16], false, [41; 16], [45; 16])
        .unwrap(),
    );
    let group = writer.parallel_group(&commands).unwrap();
    let batch = writer
      .batch([42; 16], [43; 16], None, NativeBatchStart::Now, &[group])
      .unwrap();
    let finished = writer.finish([43; 16], &[batch]).unwrap();
    assert_eq!(
      ResponseView::read(finished.as_bytes())
        .unwrap()
        .message_count(),
      1
    );
  }

  #[test]
  fn constructs_transient_ui_actions_without_owned_commands() {
    let mut writer = MessageWriter::default();
    let commands = [
      writer
        .focus_visual_element([1; 16], false, [20; 16])
        .unwrap(),
      writer
        .blur_visual_element([2; 16], false, [20; 16])
        .unwrap(),
      writer
        .capture_visual_element_pointer([3; 16], false, [20; 16], 17)
        .unwrap(),
      writer
        .release_visual_element_pointer([4; 16], false, [20; 16], 17)
        .unwrap(),
      writer
        .scroll_visual_element_to([5; 16], false, [20; 16], [21; 16])
        .unwrap(),
      writer
        .select_visual_element_text([6; 16], false, [20; 16], 11, 3)
        .unwrap(),
    ];
    let group = writer.parallel_group(&commands).unwrap();
    let batch = writer
      .batch([30; 16], [31; 16], None, NativeBatchStart::Now, &[group])
      .unwrap();
    let finished = writer.finish([31; 16], &[batch]).unwrap();
    assert_eq!(
      ResponseView::read(finished.as_bytes())
        .unwrap()
        .message_count(),
      1
    );
  }

  #[test]
  fn constructs_a_created_ui_subtree_without_owned_commands() {
    MESSAGE_BUILDER_POOL.with_borrow_mut(|pool| {
      pool.storage.clear();
      pool.idle_bytes = 0;
    });
    recycle_message_storage(vec![0xa5; 64 * 1024]);
    let mut writer = MessageWriter::default();
    let transient_element = writer
      .box_builder()
      .background_color([0.08, 0.2, 0.24, 1.0])
      .finish();
    let transient_node = writer.ui_node([10; 16], transient_element, &[]).unwrap();
    let create_transient = writer
      .create_visual_element([1; 16], false, [19; 16], None, [10; 16], &[transient_node])
      .unwrap();
    let updated_transient = writer
      .box_builder()
      .name("updated-callback-result")
      .background_color([0.1, 0.36, 0.4, 1.0])
      .finish();
    let update_transient = writer
      .update_visual_element([2; 16], false, [10; 16], updated_transient)
      .unwrap();
    let parent_transient = writer
      .update_visual_element_parent([3; 16], false, [10; 16], [19; 16])
      .unwrap();
    let index_transient = writer
      .update_visual_element_index([4; 16], false, [10; 16], 0)
      .unwrap();
    let destroy_transient = writer
      .destroy_visual_element([5; 16], false, [10; 16])
      .unwrap();
    let child_element = writer
      .label_builder("Hello, world")
      .font_size(26.0)
      .finish();
    let child = writer.ui_node([21; 16], child_element, &[]).unwrap();
    let root_element = writer
      .box_builder()
      .name("rust-callback-result")
      .background_color([0.12, 0.38, 0.22, 1.0])
      .padding(22.0, 22.0)
      .margin(12.0, 12.0)
      .finish();
    let root = writer.ui_node([20; 16], root_element, &[[21; 16]]).unwrap();
    let create_greeting = writer
      .create_visual_element([6; 16], false, [19; 16], None, [20; 16], &[root, child])
      .unwrap();
    let button = writer.button_builder().text("Hide").finish();
    let update_button = writer
      .update_visual_element([7; 16], false, [22; 16], button)
      .unwrap();
    let groups = [
      writer.parallel_group(&[create_transient]).unwrap(),
      writer.parallel_group(&[update_transient]).unwrap(),
      writer.parallel_group(&[parent_transient]).unwrap(),
      writer.parallel_group(&[index_transient]).unwrap(),
      writer.parallel_group(&[destroy_transient]).unwrap(),
      writer
        .parallel_group(&[create_greeting, update_button])
        .unwrap(),
    ];
    let batch = writer
      .batch(
        [8; 16],
        [9; 16],
        Some([11; 16]),
        NativeBatchStart::Now,
        &groups,
      )
      .unwrap();
    let finished = writer.finish([9; 16], &[batch]).unwrap();
    assert_eq!(
      ResponseView::read(finished.as_bytes())
        .unwrap()
        .message_count(),
      1
    );
  }

  #[test]
  fn copies_a_changed_label_directly_from_retained_snapshot_storage() {
    let label_id = object_id!("10000000-0000-4000-8000-000000000003");
    let snapshot = crate::RetainedUiSnapshot::build(
      object_id!("10000000-0000-4000-8000-000000000001"),
      object_id!("10000000-0000-4000-8000-000000000002"),
      &[UiNode::new(label_id, UiLabel::new("新しい 🏰"))],
    )
    .unwrap();
    let text = snapshot
      .find_node(*label_id.as_uuid().as_bytes())
      .unwrap()
      .unwrap()
      .text_property()
      .unwrap();
    let mut writer = MessageWriter::default();
    let label = writer.retained_label(text);
    let command = writer
      .update_visual_element([1; 16], false, *label_id.as_uuid().as_bytes(), label)
      .unwrap();
    let group = writer.parallel_group(&[command]).unwrap();
    let batch = writer
      .batch([3; 16], [4; 16], None, NativeBatchStart::Now, &[group])
      .unwrap();
    let finished = writer.finish([4; 16], &[batch]).unwrap();
    assert_eq!(
      ResponseView::read(finished.as_bytes())
        .unwrap()
        .message_count(),
      1
    );
  }

  #[test]
  fn rejects_an_offset_from_another_builder() {
    let mut first = MessageWriter::default();
    let command = first
      .text_set_content([1; 16], true, [2; 16], "ready")
      .unwrap();
    let mut second = MessageWriter::default();
    assert!(second.parallel_group(&[command]).is_err());
  }

  #[test]
  fn constructs_a_flattened_ui_snapshot_without_an_owned_document_graph() {
    let mut writer = MessageWriter::default();
    let asset = writer.prepared_asset(NativePreparedAssetKind::Scene, "fixture/scene");
    let scene = writer.scene([2; 16], "fixture/scene").unwrap();
    let host = writer
      .ui_document_object([3; 16], NativeObjectPlacement::default(), [4; 16])
      .unwrap();
    let root = writer.visual_element();
    let label = writer.label("ready");
    let node = writer.ui_node([5; 16], label, &[]).unwrap();
    let document = writer
      .ui_document([3; 16], [4; 16], root, &[[5; 16]], &[node])
      .unwrap();
    let snapshot = writer
      .snapshot_with_ui(
        [1; 16],
        &[asset],
        &[scene],
        Some([2; 16]),
        &[host],
        None,
        &[document],
      )
      .unwrap();
    let finished = writer.finish([1; 16], &[snapshot]).unwrap();
    let response = ResponseView::read(finished.as_bytes()).unwrap();
    assert_eq!(response.session_id(), [1; 16]);
    assert_eq!(response.message_count(), 1);
  }
}
