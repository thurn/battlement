//! Geometric input for deterministic inline rectangle fixtures.
use crate::UiWorld;
use battlement_types::{ObjectId, Rect};
use battlement_ui::{
  Display, FlexDirection, LengthOrAuto, OverlayPlacement, PanelPoint, PickingMode, Position, Prop,
  StyleValue, UiVisualElementProperties, Visibility,
};

impl UiWorld {
  /// Picks inline pixel/percentage rectangles in a document; native tests own USS and text measurement.
  pub fn pick(&self, root: ObjectId, point: PanelPoint, viewport: Rect) -> Option<ObjectId> {
    let mut entries = Vec::new();
    self.pick_rects(root, viewport, viewport, &mut entries);
    let modal = entries
      .iter()
      .filter(|(id, _)| self.is_modal(*id))
      .max_by_key(|(id, _)| self.overlay_order(*id))
      .map(|(id, _)| *id);
    entries
      .into_iter()
      .rev()
      .find_map(|(id, rect)| {
        if modal.is_some_and(|scope| !self.in_modal_scope(id, scope)) {
          return None;
        }
        let element = &self.elements[&id];
        if element.picking_mode() == Some(PickingMode::Ignore) {
          return None;
        }
        (point.x >= rect.x
          && point.y >= rect.y
          && point.x < rect.x + rect.width
          && point.y < rect.y + rect.height)
          .then_some(id)
      })
      .or(modal)
  }

  /// Whether a target is presented and accepts ordinary input through its ancestry.
  pub fn input_eligible(&self, id: ObjectId) -> bool {
    let Some(element) = self.elements.get(&id) else {
      return false;
    };
    let visual = element.element.visual_element();
    if visual.enabled == Prop::Set(false) || visual.inert == Prop::Set(true) {
      return false;
    }
    if visual.style.display == Prop::Set(StyleValue::Value(Display::None))
      || visual.style.visibility == Prop::Set(StyleValue::Value(Visibility::Hidden))
    {
      return false;
    }
    if self
      .active_modal(element.document_root_id)
      .is_some_and(|scope| !self.in_modal_scope(id, scope))
    {
      return false;
    }
    self.presented_in_hierarchy(id)
  }

  /// Whether any presented UI modal owns the pointer surface.
  pub fn has_modal(&self) -> bool {
    self
      .elements
      .keys()
      .any(|id| self.is_modal(*id) && self.presented_in_hierarchy(*id))
  }

  fn active_modal(&self, root: ObjectId) -> Option<ObjectId> {
    self
      .elements
      .values()
      .filter(|e| e.document_root_id == root)
      .filter(|e| self.is_modal(e.object_id) && self.presented_in_hierarchy(e.object_id))
      .max_by_key(|e| self.overlay_order(e.object_id))
      .map(|e| e.object_id)
  }

  fn is_modal(&self, id: ObjectId) -> bool {
    matches!(
      self.elements[&id]
        .element
        .visual_element()
        .overlay_placement,
      Prop::Set(OverlayPlacement::Modal { .. })
    )
  }

  fn overlay_order(&self, id: ObjectId) -> i32 {
    match self.elements[&id].element.visual_element().stack_item {
      Prop::Set(item) => item.order,
      _ => 0,
    }
  }

  fn in_modal_scope(&self, id: ObjectId, scope: ObjectId) -> bool {
    let mut current = Some(id);
    while let Some(id) = current {
      if id == scope {
        return true;
      }
      let element = &self.elements[&id];
      if matches!(
        element.element.visual_element().overlay_placement,
        Prop::Set(_)
      ) {
        return self.overlay_order(id) > self.overlay_order(scope);
      }
      current = element.parent_id;
    }
    false
  }

  fn presented_in_hierarchy(&self, id: ObjectId) -> bool {
    let element = &self.elements[&id];
    let visual = element.element.visual_element();
    if visual.enabled == Prop::Set(false) || visual.inert == Prop::Set(true) {
      return false;
    }
    if visual.style.display == Prop::Set(StyleValue::Value(Display::None))
      || visual.style.visibility == Prop::Set(StyleValue::Value(Visibility::Hidden))
    {
      return false;
    }
    element
      .parent_id
      .is_none_or(|parent| self.presented_in_hierarchy(parent))
  }

  fn pick_rects(
    &self,
    id: ObjectId,
    available: Rect,
    viewport: Rect,
    result: &mut Vec<(ObjectId, Rect)>,
  ) {
    let element = &self.elements[&id];
    let visual = element.element.visual_element();
    if visual.style.display == Prop::Set(StyleValue::Value(Display::None))
      || visual.style.visibility == Prop::Set(StyleValue::Value(Visibility::Hidden))
    {
      return;
    }
    let modal = matches!(
      visual.overlay_placement,
      Prop::Set(OverlayPlacement::Modal { .. })
    );
    let containing = if modal { viewport } else { available };
    let style = &visual.style;
    let rect = if modal || element.is_document_root {
      containing
    } else {
      Rect {
        x: containing.x + length(&style.left, containing.width).unwrap_or(0.0),
        y: containing.y + length(&style.top, containing.height).unwrap_or(0.0),
        width: length(&style.width, containing.width).unwrap_or(containing.width),
        height: length(&style.height, containing.height).unwrap_or(containing.height),
      }
    };
    result.push((id, rect));
    let row = style.flex_direction == Prop::Set(StyleValue::Value(FlexDirection::Row));
    let mut offset = 0.0;
    let mut children = element.children.iter().collect::<Vec<_>>();
    children.sort_by_key(|child| self.overlay_order(**child));
    for child in children {
      let child_style = self.elements[child].style();
      let absolute = child_style.position == Prop::Set(StyleValue::Value(Position::Absolute));
      let mut child_rect = rect;
      if !absolute {
        if row {
          child_rect.x += offset;
        } else {
          child_rect.y += offset;
        }
      }
      self.pick_rects(*child, child_rect, viewport, result);
      if !absolute {
        offset += if row {
          length(&child_style.width, rect.width).unwrap_or(0.0)
        } else {
          length(&child_style.height, rect.height).unwrap_or(0.0)
        };
      }
    }
  }
}

fn length(value: &Prop<StyleValue<LengthOrAuto>>, extent: f64) -> Option<f64> {
  match value {
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(px))) => Some(f64::from(*px)),
    Prop::Set(StyleValue::Value(LengthOrAuto::Percent(percent))) => {
      Some(extent * f64::from(*percent) / 100.0)
    }
    _ => None,
  }
}
