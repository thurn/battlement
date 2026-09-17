use std::{any::TypeId, collections::HashMap};

use battlement::ObjectId;
use uuid::Uuid;

use crate::{
  context,
  render::{RenderPosition, RenderTree},
  work_scope::WorkScope,
};

#[derive(Default)]
pub(crate) struct IdentityIndex<'a> {
  positions: HashMap<Uuid, Entry<'a>>,
}

struct Entry<'a> {
  position: &'a RenderPosition,
  scope: IdentityScope,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct IdentityLifetime(pub(crate) u64);

#[derive(Clone, Copy, Default, PartialEq)]
struct IdentityScope {
  work: Option<u64>,
  lifetime: u64,
}

impl<'a> IdentityIndex<'a> {
  pub(crate) fn new(trees: impl IntoIterator<Item = &'a RenderTree>) -> Self {
    let mut index = Self::default();
    for tree in trees {
      index.collect(tree, IdentityScope::default());
    }
    index
  }

  pub(crate) fn matching(&self, id: Uuid, descriptor: TypeId) -> Option<&'a RenderPosition> {
    self
      .positions
      .get(&id)
      .filter(|entry| entry.scope == self::current_scope())
      .map(|entry| entry.position)
      .filter(|position| position.descriptor == descriptor)
  }

  pub(crate) fn new_host_id(&self, id: Option<Uuid>) -> ObjectId {
    if let Some(id) = id
      && self::current_scope().work.is_none()
      && !self.positions.contains_key(&id)
    {
      let object_id = ObjectId::from_uuid(id).expect("presentation IDs cannot be nil");
      if !crate::element_ref::native_identity_unavailable(object_id) {
        return object_id;
      }
    }
    ObjectId::new_v4()
  }

  fn collect(&mut self, tree: &'a RenderTree, inherited: IdentityScope) {
    for position in &tree.positions {
      if position.terminal_visual {
        continue;
      }
      let work = position
        .provider
        .as_ref()
        .and_then(|provider| provider.get::<WorkScope>())
        .map(|scope| scope.0)
        .or(inherited.work)
        .filter(|scope| *scope != 0);
      let lifetime = position
        .provider
        .as_ref()
        .and_then(|provider| provider.get::<IdentityLifetime>())
        .map_or(inherited.lifetime, |scope| scope.0);
      let scope = IdentityScope { work, lifetime };
      if let Some(id) = position.presentation_id {
        assert!(
          self
            .positions
            .insert(id, Entry { position, scope })
            .is_none(),
          "duplicate live presentation UUID: {id}"
        );
      }
      if let Some(suspense) = &position.suspense {
        self.collect(&suspense.primary, scope);
      }
      self.collect(&position.children, scope);
    }
  }
}

fn current_scope() -> IdentityScope {
  IdentityScope {
    work: context::read_optional::<WorkScope>()
      .map(|scope| scope.0)
      .filter(|scope| *scope != 0),
    lifetime: context::read_optional::<IdentityLifetime>().map_or(0, |scope| scope.0),
  }
}
