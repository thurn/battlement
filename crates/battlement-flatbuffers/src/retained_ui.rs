use std::{
  cell::Cell,
  fmt,
  ops::{Deref, DerefMut},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{ObjectId, UiNode};
use flatbuffers::{Allocator, FlatBufferBuilder, VerifierOptions};

use crate::{
  MAXIMUM_APPARENT_BYTES, MAXIMUM_TABLE_DEPTH, MAXIMUM_TABLE_VISITS, ProtocolError,
  common_generated as common, response_ui, ui_generated as wire,
};

const MAXIMUM_BUILDER_BYTES: usize = 32 * 1024 * 1024;
const MAXIMUM_RETAINED_BYTES: usize = 64 * 1024 * 1024;
const INITIAL_BUILDER_BYTES: usize = 4 * 1024;
const ALLOCATION_FAILURE: &str = "retained UI snapshot allocation budget exhausted";

/// Shared accounting for immutable retained UI snapshot allocations.
///
/// Every builder growth reserves both the old and prospective allocations
/// until the copy completes. Snapshot replacement therefore charges the old
/// generation and the complete new generation concurrently.
#[derive(Clone, Default)]
pub struct RetainedUiBudget {
  allocated: Rc<Cell<usize>>,
}

impl RetainedUiBudget {
  /// Returns currently live allocation capacity in bytes.
  #[must_use]
  pub fn allocated_bytes(&self) -> usize {
    self.allocated.get()
  }

  fn reserve(&self, bytes: usize) -> Result<(), SnapshotAllocationError> {
    let next = self
      .allocated
      .get()
      .checked_add(bytes)
      .ok_or(SnapshotAllocationError)?;
    if next > MAXIMUM_RETAINED_BYTES {
      return Err(SnapshotAllocationError);
    }
    self.allocated.set(next);
    Ok(())
  }

  fn release(&self, bytes: usize) {
    self.allocated.set(
      self
        .allocated
        .get()
        .checked_sub(bytes)
        .expect("retained UI allocation accounting"),
    );
  }
}

#[derive(Debug)]
struct SnapshotAllocationError;

impl fmt::Display for SnapshotAllocationError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(ALLOCATION_FAILURE)
  }
}

struct SnapshotAllocator {
  storage: Box<[u8]>,
  budget: RetainedUiBudget,
}

impl SnapshotAllocator {
  fn new(budget: RetainedUiBudget) -> Result<Self, SnapshotAllocationError> {
    budget.reserve(INITIAL_BUILDER_BYTES)?;
    Ok(Self {
      storage: vec![0; INITIAL_BUILDER_BYTES].into_boxed_slice(),
      budget,
    })
  }
}

impl Deref for SnapshotAllocator {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    &self.storage
  }
}

impl DerefMut for SnapshotAllocator {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.storage
  }
}

// SAFETY: growth preserves the old bytes at the end of the new allocation,
// reports the exact byte length, and exposes the allocation as one mutable slice.
unsafe impl Allocator for SnapshotAllocator {
  type Error = SnapshotAllocationError;

  fn grow_downwards(&mut self) -> Result<(), Self::Error> {
    let old_len = self.storage.len();
    let new_len = old_len.checked_mul(2).ok_or(SnapshotAllocationError)?;
    if new_len > MAXIMUM_BUILDER_BYTES {
      return Err(SnapshotAllocationError);
    }
    // Charge the new allocation while the old allocation is still live. The
    // old charge is released only after ownership has moved to `replacement`.
    self.budget.reserve(new_len)?;
    let mut replacement = vec![0; new_len].into_boxed_slice();
    replacement[new_len - old_len..].copy_from_slice(&self.storage);
    self.storage = replacement;
    self.budget.release(old_len);
    Ok(())
  }

  fn len(&self) -> usize {
    self.storage.len()
  }
}

impl Drop for SnapshotAllocator {
  fn drop(&mut self) {
    self.budget.release(self.storage.len());
  }
}

/// Immutable FlatBuffer-owned declaration arena for one retained UI forest.
///
/// Views borrow this owner and reconstruct generated readers from stable scalar
/// indices. The arena is never mutated or returned to a builder after finish.
pub struct RetainedUiSnapshot {
  storage: SnapshotAllocator,
  start: usize,
}

impl RetainedUiSnapshot {
  /// Writes a complete desired UI forest into a fresh immutable arena.
  pub fn build(
    document_id: ObjectId,
    root_id: ObjectId,
    children: &[UiNode],
  ) -> Result<Self, ProtocolError> {
    Self::build_in(&RetainedUiBudget::default(), document_id, root_id, children)
  }

  /// Writes a desired UI forest while charging a shared 64 MiB live budget.
  pub fn build_in(
    budget: &RetainedUiBudget,
    document_id: ObjectId,
    root_id: ObjectId,
    children: &[UiNode],
  ) -> Result<Self, ProtocolError> {
    if document_id.as_uuid().is_nil() || root_id.as_uuid().is_nil() {
      return Err(ProtocolError::new(
        "retained UI snapshot UUIDs must be nonzero",
      ));
    }
    let allocator =
      SnapshotAllocator::new(budget.clone()).map_err(|_| ProtocolError::new(ALLOCATION_FAILURE))?;
    let built = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut builder = FlatBufferBuilder::new_in(allocator);
      let document =
        response_ui::write_retained_forest(&mut builder, document_id, root_id, children)?;
      builder.finish(document, None);
      Ok::<_, ProtocolError>(builder.collapse_in())
    }));
    let (storage, start) = match built {
      Ok(result) => result?,
      Err(payload) if allocation_panic(&payload) => {
        return Err(ProtocolError::new(ALLOCATION_FAILURE));
      }
      Err(payload) => panic::resume_unwind(payload),
    };
    let result = Self { storage, start };
    if result.storage.len().saturating_sub(result.start) > crate::MAXIMUM_MESSAGE_BYTES {
      return Err(ProtocolError::new("retained UI snapshot exceeds 16 MiB"));
    }
    result.root()?;
    Ok(result)
  }

  /// Writes one complete desired document, including its root declaration,
  /// while charging a shared live-allocation budget.
  pub fn build_document_in(
    budget: &RetainedUiBudget,
    document: &battlement::UiDocument,
  ) -> Result<Self, ProtocolError> {
    if document.document_id.as_uuid().is_nil() || document.root_id.as_uuid().is_nil() {
      return Err(ProtocolError::new(
        "retained UI snapshot UUIDs must be nonzero",
      ));
    }
    let allocator =
      SnapshotAllocator::new(budget.clone()).map_err(|_| ProtocolError::new(ALLOCATION_FAILURE))?;
    let built = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut builder = FlatBufferBuilder::new_in(allocator);
      let root = response_ui::write_document(&mut builder, document)?;
      builder.finish(root, None);
      Ok::<_, ProtocolError>(builder.collapse_in())
    }));
    let (storage, start) = match built {
      Ok(result) => result?,
      Err(payload) if allocation_panic(&payload) => {
        return Err(ProtocolError::new(ALLOCATION_FAILURE));
      }
      Err(payload) => panic::resume_unwind(payload),
    };
    let result = Self { storage, start };
    if result.storage.len().saturating_sub(result.start) > crate::MAXIMUM_MESSAGE_BYTES {
      return Err(ProtocolError::new("retained UI snapshot exceeds 16 MiB"));
    }
    result.root()?;
    Ok(result)
  }

  /// Returns bytes retained by the original builder allocation.
  #[must_use]
  pub fn allocation_bytes(&self) -> usize {
    self.storage.len()
  }

  /// Compares two complete desired declarations without reconstructing owned UI values.
  #[must_use]
  pub fn same_declaration(&self, other: &Self) -> bool {
    self.storage[self.start..] == other.storage[other.start..]
  }

  /// Returns the number of flattened retained nodes.
  pub fn node_count(&self) -> Result<usize, ProtocolError> {
    Ok(self.root()?.nodes().len())
  }

  /// Borrows one flattened node by stable scalar index.
  pub fn node(&self, index: usize) -> Result<RetainedUiNodeView<'_>, ProtocolError> {
    let document = self.root()?;
    let nodes = document.nodes();
    if index >= nodes.len() {
      return Err(ProtocolError::new(
        "retained UI node index is out of bounds",
      ));
    }
    Ok(RetainedUiNodeView {
      value: nodes.get(index),
    })
  }

  /// Finds a flattened node by canonical UUID bytes.
  pub fn find_node(
    &self,
    object_id: [u8; 16],
  ) -> Result<Option<RetainedUiNodeView<'_>>, ProtocolError> {
    let document = self.root()?;
    Ok(
      document
        .nodes()
        .iter()
        .find(|node| uuid(node.object_id()) == object_id)
        .map(|value| RetainedUiNodeView { value }),
    )
  }

  fn root(&self) -> Result<wire::UiDocument<'_>, ProtocolError> {
    let options = VerifierOptions {
      max_depth: MAXIMUM_TABLE_DEPTH,
      max_tables: MAXIMUM_TABLE_VISITS,
      max_apparent_size: MAXIMUM_APPARENT_BYTES,
      ignore_missing_null_terminator: false,
    };
    flatbuffers::root_with_opts::<wire::UiDocument<'_>>(&options, &self.storage[self.start..])
      .map_err(|error| ProtocolError::new(format!("invalid retained UI snapshot: {error}")))
  }
}

fn allocation_panic(payload: &Box<dyn std::any::Any + Send>) -> bool {
  payload
    .downcast_ref::<String>()
    .is_some_and(|message| message.contains(ALLOCATION_FAILURE))
    || payload
      .downcast_ref::<&str>()
      .is_some_and(|message| message.contains(ALLOCATION_FAILURE))
}

/// Borrowed node declaration reconstructed from a retained snapshot owner.
#[derive(Clone, Copy)]
pub struct RetainedUiNodeView<'a> {
  value: wire::UiNode<'a>,
}

impl<'a> RetainedUiNodeView<'a> {
  /// Returns the node's canonical object UUID bytes.
  #[must_use]
  pub fn object_id(self) -> [u8; 16] {
    uuid(self.value.object_id())
  }

  /// Returns the generated UI element kind ordinal.
  #[must_use]
  pub fn element_kind(self) -> u8 {
    self.value.element().kind().0
  }

  /// Returns the number of ordered child identities.
  #[must_use]
  pub fn child_count(self) -> usize {
    self.value.child_ids().len()
  }

  /// Returns one ordered child UUID.
  pub fn child_id(self, index: usize) -> Result<[u8; 16], ProtocolError> {
    let children = self.value.child_ids();
    if index >= children.len() {
      return Err(ProtocolError::new(
        "retained UI child index is out of bounds",
      ));
    }
    Ok(uuid(children.get(index)))
  }

  /// Returns the number of typed properties in this desired declaration.
  #[must_use]
  pub fn property_count(self) -> usize {
    self.value.element().properties().len()
  }

  /// Borrows the authored text property when this element declares one.
  #[must_use]
  pub fn text_property(self) -> Option<RetainedTextPropertyView<'a>> {
    self
      .value
      .element()
      .properties()
      .iter()
      .find(|property| property.key() == wire::UiPropertyKey::Text)
      .map(|value| RetainedTextPropertyView { value })
  }
}

/// Borrowed three-state text declaration from an immutable retained arena.
#[derive(Clone, Copy)]
pub struct RetainedTextPropertyView<'a> {
  value: wire::UiProperty<'a>,
}

impl<'a> RetainedTextPropertyView<'a> {
  /// Returns the authored property state without constructing a `Prop<String>`.
  #[must_use]
  pub fn state(self) -> RetainedPropState {
    match self.value.state() {
      wire::PropState::Unset => RetainedPropState::Unset,
      wire::PropState::Set => RetainedPropState::Set,
      wire::PropState::Reset => RetainedPropState::Reset,
      _ => unreachable!("verified retained UI property state is closed"),
    }
  }

  /// Borrows UTF-8 text only for a Set declaration.
  #[must_use]
  pub fn value(self) -> Option<&'a str> {
    (self.state() == RetainedPropState::Set).then(|| {
      self
        .value
        .value_as_text_property_value()
        .expect("verified retained text property has its typed payload")
        .value()
    })
  }

  pub(crate) fn wire_state(self) -> wire::PropState {
    self.value.state()
  }
}

/// Authored state of a retained sparse property.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedPropState {
  /// No declaration is present in this generation.
  Unset,
  /// The declaration assigns its payload.
  Set,
  /// The declaration restores the host default.
  Reset,
}

fn uuid(value: &common::Uuid) -> [u8; 16] {
  std::array::from_fn(|index| value.bytes().get(index))
}

#[cfg(test)]
mod tests {
  use battlement::{UiLabel, UiNode, object_id};

  use super::*;

  #[test]
  fn retains_flattened_ui_declarations_without_owned_reader_nodes() {
    let child_id = object_id!("10000000-0000-4000-8000-000000000003");
    let snapshot = RetainedUiSnapshot::build(
      object_id!("10000000-0000-4000-8000-000000000001"),
      object_id!("10000000-0000-4000-8000-000000000002"),
      &[UiNode::new(child_id, UiLabel::new("retained"))],
    )
    .unwrap();

    assert_eq!(snapshot.node_count().unwrap(), 1);
    let node = snapshot.node(0).unwrap();
    assert_eq!(node.object_id(), *child_id.as_uuid().as_bytes());
    assert_eq!(node.child_count(), 0);
    assert!(node.property_count() > 0);
    assert!(snapshot.allocation_bytes() >= 4 * 1024);
    assert_eq!(
      snapshot
        .find_node(*child_id.as_uuid().as_bytes())
        .unwrap()
        .unwrap()
        .text_property()
        .unwrap()
        .value(),
      Some("retained")
    );
  }

  #[test]
  fn compares_complete_declarations_in_retained_storage() {
    let document_id = object_id!("10000000-0000-4000-8000-000000000011");
    let root_id = object_id!("10000000-0000-4000-8000-000000000012");
    let child_id = object_id!("10000000-0000-4000-8000-000000000013");
    let first = RetainedUiSnapshot::build(
      document_id,
      root_id,
      &[UiNode::new(child_id, UiLabel::new("same"))],
    )
    .unwrap();
    let same = RetainedUiSnapshot::build(
      document_id,
      root_id,
      &[UiNode::new(child_id, UiLabel::new("same"))],
    )
    .unwrap();
    let changed = RetainedUiSnapshot::build(
      document_id,
      root_id,
      &[UiNode::new(child_id, UiLabel::new("changed"))],
    )
    .unwrap();

    assert!(first.same_declaration(&same));
    assert!(!first.same_declaration(&changed));
  }

  #[test]
  fn charges_growth_overlap_and_releases_the_finished_arena() {
    let budget = RetainedUiBudget::default();
    {
      let mut allocator = SnapshotAllocator::new(budget.clone()).unwrap();
      assert_eq!(budget.allocated_bytes(), INITIAL_BUILDER_BYTES);
      allocator.grow_downwards().unwrap();
      assert_eq!(budget.allocated_bytes(), INITIAL_BUILDER_BYTES * 2);
    }
    assert_eq!(budget.allocated_bytes(), 0);
  }

  #[test]
  fn rejects_growth_before_exceeding_the_shared_live_budget() {
    let budget = RetainedUiBudget::default();
    budget
      .reserve(MAXIMUM_RETAINED_BYTES - INITIAL_BUILDER_BYTES)
      .unwrap();
    let mut allocator = SnapshotAllocator::new(budget.clone()).unwrap();
    let before = budget.allocated_bytes();
    assert!(allocator.grow_downwards().is_err());
    assert_eq!(budget.allocated_bytes(), before);
    drop(allocator);
    budget.release(MAXIMUM_RETAINED_BYTES - INITIAL_BUILDER_BYTES);
    assert_eq!(budget.allocated_bytes(), 0);
  }
}
