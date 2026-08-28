//! Render values supported by the Reactant tree builder.

use battlement::{Label, UiNode};

/// A value Reactant can lower into native host descriptions.
pub trait Render: private::Sealed + 'static {}

impl Render for () {}

impl Render for Label {}

pub(crate) fn lower<R>(value: R, committed: &[UiNode]) -> Vec<UiNode>
where
  R: Render,
{
  value.lower(committed)
}

mod private {
  use battlement::{Label, ObjectId, UiNode};

  pub trait Sealed {
    fn lower(self, committed: &[UiNode]) -> Vec<UiNode>;
  }

  impl Sealed for () {
    fn lower(self, _committed: &[UiNode]) -> Vec<UiNode> {
      Vec::new()
    }
  }

  impl Sealed for Label {
    fn lower(self, committed: &[UiNode]) -> Vec<UiNode> {
      let object_id = committed
        .first()
        .map_or_else(ObjectId::new_v4, |node| node.object_id);
      vec![UiNode::new(object_id, self)]
    }
  }
}
