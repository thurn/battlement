//! Stable committed host references and one-shot host actions.

#![allow(private_interfaces)]

use std::{
  any::TypeId,
  cell::{Cell, RefCell},
  collections::{HashMap, HashSet},
  hash::{Hash, Hasher},
  rc::{Rc, Weak},
};

use battlement::{
  Command, CommandBody, ElementGeometry, ObjectId, Prop, TextElement, UiElement, UiElementKind,
  UiNode, VisualElementAction, VisualElementProperties,
};

use crate::{
  context,
  geometry::Measurement,
  geometry_runtime::GeometryRuntime,
  hook_storage::{HookKind, HookSlot},
  hooks,
  portal::PortalLayout,
  render::{Render, RenderSink, RenderTree},
  render_value::Sealed,
};

thread_local! {
  static CURRENT_RUNTIME: RefCell<Option<RuntimeContext>> = const { RefCell::new(None) };
}

/// Identifies one committed host owned by a Reactant runtime.
#[derive(Clone)]
pub struct ElementRef {
  inner: Rc<ElementRefInner>,
}

/// A terminal host adapter carrying one element ref.
///
/// Primitive properties and event handlers must be authored before the ref.
///
/// ```compile_fail
/// use battlement::Button;
/// use battlement_reactant::prelude::*;
///
/// struct Invalid;
/// impl Component for Invalid {
///   fn render(&self) -> impl Render {
///     let element_ref = use_element_ref();
///     Button::new("late").element_ref(element_ref).name("invalid")
///   }
/// }
/// ```
///
/// Components and structural values do not expose the host adapter.
///
/// ```compile_fail
/// use battlement::Label;
/// use battlement_reactant::prelude::*;
///
/// struct Child;
/// impl Component for Child {
///   fn render(&self) -> impl Render { Label::new("child") }
/// }
///
/// struct Invalid;
/// impl Component for Invalid {
///   fn render(&self) -> impl Render {
///     Child.element_ref(use_element_ref())
///   }
/// }
/// ```
pub struct Referenced<R> {
  pub(crate) render: R,
  pub(crate) element_ref: ElementRef,
}

pub(crate) struct ElementRefRuntime {
  next_identity: u64,
  attached: HashMap<u64, ElementRef>,
  actions: Vec<QueuedAction>,
}

pub(crate) struct AttachmentSet {
  desired: HashMap<u64, DesiredAttachment>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct ElementAttachment {
  document_id: ObjectId,
  object_id: ObjectId,
  generation: u64,
}

struct ElementRefInner {
  runtime_id: u64,
  identity: u64,
  runtime: Weak<RefCell<ElementRefRuntime>>,
  geometry: Weak<RefCell<GeometryRuntime>>,
  attachment: Cell<Option<ElementAttachment>>,
  next_generation: Cell<u64>,
}

struct ElementRefSlot {
  value: ElementRef,
}

struct DesiredAttachment {
  element_ref: ElementRef,
  document_id: ObjectId,
  object_id: ObjectId,
}

#[derive(Clone, Copy)]
struct QueuedTarget {
  identity: u64,
  attachment: ElementAttachment,
}

struct QueuedAction {
  target: QueuedTarget,
  descendant: Option<QueuedTarget>,
  action: VisualElementAction,
}

#[derive(Clone)]
struct RuntimeContext {
  runtime_id: u64,
  runtime: Weak<RefCell<ElementRefRuntime>>,
  geometry: Weak<RefCell<GeometryRuntime>>,
}

pub(crate) struct RuntimeGuard(Option<RuntimeContext>);

/// Returns a stable element ref for the current mounted component.
pub fn use_element_ref() -> ElementRef {
  hooks::use_slot(
    HookKind::ElementRef,
    TypeId::of::<ElementRef>(),
    |_| ElementRefSlot {
      value: self::create_ref(),
    },
    |slot| slot.value.clone(),
  )
}

impl ElementRef {
  pub(crate) fn identity(&self) -> u64 {
    self.inner.identity
  }

  pub(crate) fn geometry_identity(&self) -> (u64, u64, Option<ObjectId>) {
    (
      self.inner.runtime_id,
      self.inner.identity,
      self
        .inner
        .attachment
        .get()
        .map(|attachment| attachment.object_id),
    )
  }

  /// Returns whether this ref currently identifies a committed host.
  #[must_use]
  pub fn is_attached(&self) -> bool {
    assert!(
      !context::rendering(),
      "Reactant element refs cannot be queried while rendering"
    );
    self.inner.attachment.get().is_some()
  }

  /// Returns the last measurement installed by a committed geometry consumer.
  #[must_use]
  pub fn geometry(&self) -> Measurement<ElementGeometry> {
    assert!(
      !context::rendering(),
      "Reactant element refs cannot be queried while rendering"
    );
    self
      .inner
      .geometry
      .upgrade()
      .map_or_else(Measurement::waiting, |runtime| {
        runtime.borrow().element(self)
      })
  }

  /// Requests focus on the current attachment.
  pub fn focus(&self) {
    self.queue(VisualElementAction::Focus, None);
  }

  /// Requests removal of focus from the current attachment.
  pub fn blur(&self) {
    self.queue(VisualElementAction::Blur, None);
  }

  /// Captures one pointer on the current attachment.
  pub fn capture_pointer(&self, pointer_id: i32) {
    self.queue(VisualElementAction::CapturePointer { pointer_id }, None);
  }

  /// Releases one pointer from the current attachment.
  pub fn release_pointer(&self, pointer_id: i32) {
    self.queue(VisualElementAction::ReleasePointer { pointer_id }, None);
  }

  /// Scrolls until the attached descendant is visible.
  pub fn scroll_to(&self, descendant: &ElementRef) {
    assert!(
      !context::rendering(),
      "Reactant element actions cannot be requested while rendering"
    );
    let Some(descendant_id) = descendant
      .inner
      .attachment
      .get()
      .map(|attachment| attachment.object_id)
    else {
      return;
    };
    self.queue(
      VisualElementAction::ScrollTo { descendant_id },
      Some(descendant),
    );
  }

  /// Sets UTF-16 cursor and selection endpoints on selectable text.
  pub fn select_text(&self, cursor_index: u32, selection_index: u32) {
    self.queue(
      VisualElementAction::SelectText {
        cursor_index,
        selection_index,
      },
      None,
    );
  }

  fn queue(&self, action: VisualElementAction, descendant: Option<&ElementRef>) {
    assert!(
      !context::rendering(),
      "Reactant element actions cannot be requested while rendering"
    );
    self::validate_current_runtime(self.inner.runtime_id);
    let Some(attachment) = self.inner.attachment.get() else {
      return;
    };
    let descendant = descendant.and_then(|descendant| {
      let attachment = descendant.inner.attachment.get()?;
      assert_eq!(
        self.inner.runtime_id, descendant.inner.runtime_id,
        "Reactant element actions cannot cross runtimes"
      );
      Some(QueuedTarget {
        identity: descendant.inner.identity,
        attachment,
      })
    });
    if matches!(action, VisualElementAction::ScrollTo { .. }) && descendant.is_none() {
      return;
    }
    let Some(runtime) = self.inner.runtime.upgrade() else {
      return;
    };
    runtime.borrow_mut().actions.push(QueuedAction {
      target: QueuedTarget {
        identity: self.inner.identity,
        attachment,
      },
      descendant,
      action,
    });
  }

  fn attach(&self, document_id: ObjectId, object_id: ObjectId, force: bool) {
    if !force
      && self
        .inner
        .attachment
        .get()
        .is_some_and(|value| value.document_id == document_id && value.object_id == object_id)
    {
      return;
    }
    let generation = self
      .inner
      .next_generation
      .get()
      .checked_add(1)
      .expect("Reactant element attachment generation overflow");
    self.inner.next_generation.set(generation);
    self.inner.attachment.set(Some(ElementAttachment {
      document_id,
      object_id,
      generation,
    }));
  }
}

impl PartialEq for ElementRef {
  fn eq(&self, other: &Self) -> bool {
    self.inner.runtime_id == other.inner.runtime_id && self.inner.identity == other.inner.identity
  }
}

impl Eq for ElementRef {}

impl Hash for ElementRef {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner.runtime_id.hash(state);
    self.inner.identity.hash(state);
  }
}

impl<R: Render> Render for Referenced<R> {}

impl<R: Render> Sealed for Referenced<R> {
  fn descriptor(&self) -> TypeId {
    self.render.descriptor()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.with_element_ref(self.element_ref.clone(), |sink| {
      self.render.render_into(sink);
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.with_element_ref(self.element_ref, |sink| {
      self.render.render_owned(sink);
    });
  }
}

impl ElementRefRuntime {
  pub(crate) fn new() -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      next_identity: 0,
      attached: HashMap::new(),
      actions: Vec::new(),
    }))
  }

  pub(crate) fn queued_actions(&self) -> usize {
    self.actions.len()
  }

  pub(crate) fn consume_actions(&mut self, count: usize) {
    self.actions.drain(..count);
  }

  pub(crate) fn detach_all(&mut self) {
    for element_ref in self.attached.values() {
      element_ref.inner.attachment.set(None);
    }
    self.attached.clear();
    self.actions.clear();
  }
}

impl AttachmentSet {
  pub(crate) fn geometry_target(
    &self,
    runtime_id: u64,
    element_ref: &ElementRef,
  ) -> Option<(u64, ObjectId)> {
    assert_eq!(
      runtime_id, element_ref.inner.runtime_id,
      "Reactant geometry targets cannot cross runtimes"
    );
    self
      .desired
      .get(&element_ref.inner.identity)
      .map(|attachment| (element_ref.inner.identity, attachment.object_id))
  }

  pub(crate) fn collect<'a>(
    runtime_id: u64,
    roots: impl IntoIterator<Item = (ObjectId, &'a RenderTree)>,
  ) -> Self {
    let mut desired = HashMap::new();
    for (document_id, tree) in roots {
      self::collect_tree(runtime_id, document_id, tree, &mut desired);
    }
    Self { desired }
  }

  pub(crate) fn commit(self, runtime: &mut ElementRefRuntime, reconnect: bool) {
    let desired_ids = self.desired.keys().copied().collect::<HashSet<_>>();
    for (identity, desired) in &self.desired {
      desired
        .element_ref
        .attach(desired.document_id, desired.object_id, reconnect);
      runtime
        .attached
        .insert(*identity, desired.element_ref.clone());
    }
    let detached = runtime
      .attached
      .keys()
      .copied()
      .filter(|identity| !desired_ids.contains(identity))
      .collect::<Vec<_>>();
    for identity in detached {
      if let Some(element_ref) = runtime.attached.remove(&identity) {
        element_ref.inner.attachment.set(None);
      }
    }
  }

  pub(crate) fn action_groups(
    &self,
    runtime: &ElementRefRuntime,
    count: usize,
    layout: &PortalLayout,
  ) -> Vec<Vec<CommandBody>> {
    runtime.actions[..count]
      .iter()
      .filter_map(|queued| self.action_body(queued, layout))
      .map(|body| vec![body])
      .collect()
  }

  fn action_body(&self, queued: &QueuedAction, layout: &PortalLayout) -> Option<CommandBody> {
    if !self.current(queued.target) {
      return None;
    }
    let target = self::find_node(layout, queued.target.attachment.object_id)
      .expect("an attached Reactant element is missing from the desired UI");
    let action = match (&queued.action, queued.descendant) {
      (VisualElementAction::ScrollTo { .. }, Some(descendant)) if self.current(descendant) => {
        assert!(
          target.element.kind() == UiElementKind::ScrollView,
          "Reactant ScrollTo requires a ScrollView"
        );
        assert!(
          self::is_descendant(target, descendant.attachment.object_id),
          "Reactant ScrollTo requires an attached descendant"
        );
        VisualElementAction::ScrollTo {
          descendant_id: descendant.attachment.object_id,
        }
      }
      (VisualElementAction::ScrollTo { .. }, _) => return None,
      (VisualElementAction::Focus, _) => {
        assert!(
          self::focusable(&target.element),
          "Reactant Focus requires a focusable host"
        );
        VisualElementAction::Focus
      }
      (
        VisualElementAction::SelectText {
          cursor_index,
          selection_index,
        },
        _,
      ) => {
        self::validate_selection(&target.element, *cursor_index, *selection_index);
        queued.action.clone()
      }
      _ => queued.action.clone(),
    };
    Some(Command::perform_visual_element_action(target.object_id, action).body)
  }

  fn current(&self, target: QueuedTarget) -> bool {
    self.desired.get(&target.identity).is_some_and(|desired| {
      desired.document_id == target.attachment.document_id
        && desired.object_id == target.attachment.object_id
        && desired.element_ref.inner.attachment.get() == Some(target.attachment)
    })
  }
}

impl HookSlot for ElementRefSlot {
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      value: self.value.clone(),
    })
  }

  fn commit(&mut self) {}

  fn discard_pending(&mut self) {}

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::ElementRef
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<ElementRef>()
  }
}

impl Drop for RuntimeGuard {
  fn drop(&mut self) {
    CURRENT_RUNTIME.with(|current| current.replace(self.0.take()));
  }
}

pub(crate) fn enter_runtime(
  runtime_id: u64,
  runtime: &Rc<RefCell<ElementRefRuntime>>,
  geometry: &Rc<RefCell<GeometryRuntime>>,
) -> RuntimeGuard {
  RuntimeGuard(CURRENT_RUNTIME.with(|current| {
    current.replace(Some(RuntimeContext {
      runtime_id,
      runtime: Rc::downgrade(runtime),
      geometry: Rc::downgrade(geometry),
    }))
  }))
}

fn create_ref() -> ElementRef {
  CURRENT_RUNTIME.with(|current| {
    let current = current.borrow();
    let current = current
      .as_ref()
      .expect("Reactant element refs require a runtime render context");
    let runtime_rc = current
      .runtime
      .upgrade()
      .expect("Reactant element ref runtime is no longer available");
    let mut runtime = runtime_rc.borrow_mut();
    let identity = runtime.next_identity;
    runtime.next_identity = runtime
      .next_identity
      .checked_add(1)
      .expect("Reactant element ref identity overflow");
    ElementRef {
      inner: Rc::new(ElementRefInner {
        runtime_id: current.runtime_id,
        identity,
        runtime: Rc::downgrade(&runtime_rc),
        geometry: current.geometry.clone(),
        attachment: Cell::new(None),
        next_generation: Cell::new(0),
      }),
    }
  })
}

fn validate_current_runtime(runtime_id: u64) {
  CURRENT_RUNTIME.with(|current| {
    if let Some(current) = current.borrow().as_ref() {
      assert_eq!(
        current.runtime_id, runtime_id,
        "Reactant element actions cannot cross runtimes"
      );
    }
  });
}

fn collect_tree(
  runtime_id: u64,
  document_id: ObjectId,
  tree: &RenderTree,
  desired: &mut HashMap<u64, DesiredAttachment>,
) {
  for position in &tree.positions {
    if let Some(element_ref) = &position.element_ref {
      assert_eq!(
        element_ref.inner.runtime_id, runtime_id,
        "a Reactant element ref belongs to another runtime"
      );
      let object_id = position
        .host
        .as_ref()
        .expect("Reactant element refs require a host render value")
        .object_id;
      assert!(
        desired
          .insert(
            element_ref.inner.identity,
            DesiredAttachment {
              element_ref: element_ref.clone(),
              document_id,
              object_id,
            },
          )
          .is_none(),
        "two Reactant hosts cannot share one element ref"
      );
    }
    if let Some(suspense) = &position.suspense {
      self::collect_tree(runtime_id, document_id, &suspense.primary, desired);
    }
    self::collect_tree(runtime_id, document_id, &position.children, desired);
  }
}

fn find_node(layout: &PortalLayout, object_id: ObjectId) -> Option<&UiNode> {
  layout
    .roots
    .iter()
    .flat_map(|root| &root.hosts)
    .chain(layout.externals.values().flat_map(|root| &root.hosts))
    .find_map(|node| self::find_in_node(node, object_id))
}

fn find_in_node(node: &UiNode, object_id: ObjectId) -> Option<&UiNode> {
  if node.object_id == object_id {
    return Some(node);
  }
  node
    .children
    .iter()
    .find_map(|child| self::find_in_node(child, object_id))
}

fn is_descendant(node: &UiNode, descendant_id: ObjectId) -> bool {
  node
    .children
    .iter()
    .any(|child| child.object_id == descendant_id || self::is_descendant(child, descendant_id))
}

fn focusable(element: &UiElement) -> bool {
  match element.visual_element().focusable {
    Prop::Set(value) => value,
    Prop::Reset | Prop::Unset => matches!(
      element.kind(),
      UiElementKind::TextField
        | UiElementKind::Toggle
        | UiElementKind::RadioButton
        | UiElementKind::RadioButtonGroup
        | UiElementKind::ToggleButtonGroup
        | UiElementKind::DropdownField
        | UiElementKind::Button
        | UiElementKind::RepeatButton
        | UiElementKind::Scroller
        | UiElementKind::Slider
        | UiElementKind::SliderInt
        | UiElementKind::MinMaxSlider
        | UiElementKind::Tab
    ),
  }
}

fn validate_selection(element: &UiElement, cursor_index: u32, selection_index: u32) {
  let text = match element {
    UiElement::TextField(value) => match &value.value {
      Prop::Set(text) => text.as_str(),
      Prop::Reset | Prop::Unset => "",
    },
    UiElement::TextElement(TextElement {
      text,
      selectable: Prop::Set(true),
      ..
    }) => match text {
      Prop::Set(text) => text.as_str(),
      Prop::Reset | Prop::Unset => "",
    },
    _ => panic!("Reactant SelectText requires selectable text"),
  };
  let length = text.encode_utf16().count();
  assert!(
    cursor_index as usize <= length && selection_index as usize <= length,
    "Reactant SelectText indices exceed the current text"
  );
}
