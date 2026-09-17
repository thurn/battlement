use std::any::TypeId;

use uuid::Uuid;

use crate::{
  identity::IdentifiedMarker,
  render::{self, Node, RenderSink, RenderTree},
};

impl RenderSink<'_> {
  pub(crate) fn push_identified(
    &mut self,
    id: Uuid,
    retained_render: Option<Node>,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let descriptor = TypeId::of::<IdentifiedMarker>();
    let empty = RenderTree::default();
    let committed = self
      .identities
      .matching(id, descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children =
      render::sink_with_scope(committed, self.variant_scope.clone(), self.identities);
    render(&mut children);
    let (children, pending) = match RenderSink::finish_child(children) {
      Ok(value) => value,
      Err(error) => {
        self.error = Some(error);
        return;
      }
    };
    self.pending.extend(pending);
    self.push(descriptor, None, children);
    let position = self
      .positions
      .last_mut()
      .expect("identified contribution was appended");
    position.presentation_id = Some(id);
    position.retained_render = retained_render;
  }
}
