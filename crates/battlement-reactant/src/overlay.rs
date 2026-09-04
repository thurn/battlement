//! Portal-backed application overlays.

#![allow(private_interfaces)]

use std::{any::TypeId, collections::HashMap};

use battlement::{
  Align, LengthUnits, Overflow, OverlayLayer, OverlayPlacement, PickingMode, PopoverPlacement,
  Prop, StackItem, Style, UiVisualElementProperties,
};

use crate::{
  callback::{Callback, IntoCallback},
  control_behavior,
  element_ref::ElementRef,
  focus::FocusProps,
  host::{Stack, View},
  portal::{PortalTarget, create_portal},
  render::RenderTree,
  render::{Node, Render, RenderSink},
  render_value::Sealed,
  semantics::{ControlBehavior, InteractionProps, SemanticName, SemanticProps},
};

/// Deferred overlay references resolved against the complete desired tree.
#[derive(Clone)]
pub(crate) enum OverlayReference {
  Popover {
    anchor: ElementRef,
    placement: PopoverPlacement,
  },
  Modal {
    initial_focus: Option<ElementRef>,
    restore_focus: Option<ElementRef>,
  },
}

pub(crate) fn resolve_order(trees: &mut [RenderTree]) {
  let mut modal_ids = Vec::new();
  for tree in trees.iter() {
    self::collect_modals(tree, &mut modal_ids);
  }
  let ranks = modal_ids
    .into_iter()
    .enumerate()
    .map(|(index, id)| {
      (
        id,
        i32::try_from(index + 1).expect("Reactant overlay modal rank overflow"),
      )
    })
    .collect::<HashMap<_, _>>();
  for tree in trees {
    self::apply_order(tree, &ranks, 0);
  }
}

/// The root-level Stack that owns one internal overlay portal target.
#[derive(Clone)]
pub struct OverlayHost {
  host: Stack,
}

/// One portal-backed overlay wrapper and its logical child.
#[derive(Clone)]
pub struct Overlay {
  dialog_name: Option<SemanticName>,
  on_dismiss: Option<Callback<()>>,
  target: PortalTarget,
  wrapper: View,
}

impl OverlayHost {
  /// Creates a transparent, noncontributing final Stack layer for `target`.
  #[must_use]
  pub fn new(target: PortalTarget) -> Self {
    Self {
      host: Stack::new()
        .portal_target(target)
        .picking_mode(PickingMode::Ignore)
        .style(
          Style::new()
            .width(100.0_f32.pct())
            .height(100.0_f32.pct())
            .overflow(Overflow::Visible),
        )
        .stack_item(
          StackItem::new()
            .order(i32::MAX)
            .align_self(Align::Stretch)
            .justify_self(Align::Stretch)
            .contributes_to_size(false),
        ),
    }
  }
}

impl Overlay {
  /// Creates an unanchored host-filling Popover-tier wrapper.
  #[must_use]
  pub fn layer(target: PortalTarget) -> Self {
    Self::new(
      target,
      Prop::Set(OverlayPlacement::Layer(OverlayLayer::Popover)),
      None,
      false,
      true,
    )
  }

  /// Creates an anchored popover using bottom-start defaults.
  #[must_use]
  pub fn popover(target: PortalTarget, anchor: ElementRef) -> Self {
    Self::new(
      target,
      Prop::Unset,
      Some(OverlayReference::Popover {
        anchor,
        placement: PopoverPlacement::default(),
      }),
      false,
      false,
    )
  }

  /// Creates a viewport-filling modal focus scope.
  #[must_use]
  pub fn modal(target: PortalTarget, name: impl Into<SemanticName>) -> Self {
    let mut overlay = Self::new(
      target,
      Prop::Unset,
      Some(OverlayReference::Modal {
        initial_focus: None,
        restore_focus: None,
      }),
      true,
      true,
    );
    overlay.dialog_name = Some(name.into());
    overlay
  }

  /// Supplies the modal's optional authoritative dismiss callback.
  #[must_use]
  pub fn on_dismiss<G: 'static>(mut self, callback: impl IntoCallback<(), G>) -> Self {
    assert!(self.dialog_name.is_some(), "only a modal accepts dismissal");
    self.on_dismiss = Some(callback.into_callback());
    self
  }

  /// Replaces anchored-popover placement and collision policy.
  #[must_use]
  pub fn placement(mut self, value: PopoverPlacement) -> Self {
    let Some(OverlayReference::Popover { placement, .. }) =
      self.wrapper.state.overlay_reference.as_mut()
    else {
      panic!("only a popover accepts placement options");
    };
    *placement = value;
    self
  }

  /// Sets the preferred initial focus target for a modal.
  #[must_use]
  pub fn initial_focus(mut self, value: ElementRef) -> Self {
    let Some(OverlayReference::Modal { initial_focus, .. }) =
      self.wrapper.state.overlay_reference.as_mut()
    else {
      panic!("only a modal accepts an initial focus target");
    };
    *initial_focus = Some(value);
    self
  }

  /// Sets the preferred focus target after the final modal closes.
  #[must_use]
  pub fn restore_focus(mut self, value: ElementRef) -> Self {
    let Some(OverlayReference::Modal { restore_focus, .. }) =
      self.wrapper.state.overlay_reference.as_mut()
    else {
      panic!("only a modal accepts a restore focus target");
    };
    *restore_focus = Some(value);
    self
  }

  /// Appends one logical child inside the public overlay wrapper.
  #[must_use]
  pub fn child(mut self, child: impl Render) -> Self {
    self.wrapper.state.children.push(Node::new(child));
    self
  }

  /// Attaches one advanced control behavior to a non-modal overlay wrapper.
  #[must_use]
  pub fn behavior<G: 'static>(mut self, value: ControlBehavior<G>) -> Self {
    assert!(
      self.dialog_name.is_none(),
      "modal dialog behavior is intrinsic"
    );
    self.wrapper = self.wrapper.behavior(value);
    self
  }

  /// Replaces inline declarations on the public overlay wrapper.
  #[must_use]
  pub fn style(mut self, value: Style) -> Self {
    self.wrapper.state.host.visual_element_mut().style = value;
    self
  }

  /// Appends one USS class to the public overlay wrapper.
  #[must_use]
  pub fn class(mut self, value: impl Into<String>) -> Self {
    self.wrapper = self.wrapper.class(value);
    self
  }

  /// Sets the wrapper name used by Unity queries and selectors.
  #[must_use]
  pub fn host_name(mut self, value: impl Into<Prop<String>>) -> Self {
    self.wrapper = self.wrapper.name(value);
    self
  }

  /// Sets whether the wrapper subtree is locally enabled.
  #[must_use]
  pub fn enabled(mut self, value: impl Into<Prop<bool>>) -> Self {
    let value = value.into();
    if matches!(
      self.wrapper.state.overlay_reference,
      Some(OverlayReference::Modal { .. })
    ) {
      assert_ne!(
        value,
        Prop::Set(false),
        "a modal wrapper must remain enabled"
      );
    }
    self.wrapper = self.wrapper.enabled(value);
    self
  }

  /// Sets whether the wrapper may receive programmatic focus.
  #[must_use]
  pub fn focusable(mut self, value: impl Into<Prop<bool>>) -> Self {
    let value = value.into();
    if matches!(
      self.wrapper.state.overlay_reference,
      Some(OverlayReference::Modal { .. })
    ) {
      assert_eq!(
        value,
        Prop::Set(true),
        "a modal wrapper must remain focusable"
      );
    }
    self.wrapper = self.wrapper.focusable(value);
    self
  }

  /// Sets the wrapper's focus-ring position.
  #[must_use]
  pub fn tab_index(mut self, value: impl Into<Prop<i32>>) -> Self {
    let value = value.into();
    if matches!(
      self.wrapper.state.overlay_reference,
      Some(OverlayReference::Modal { .. })
    ) {
      assert_eq!(
        value,
        Prop::Set(-1),
        "a modal wrapper must retain tab index -1"
      );
    }
    self.wrapper = self.wrapper.tab_index(value);
    self
  }

  /// Merges a composable focus declaration bundle into the wrapper.
  #[must_use]
  pub fn focus_props(mut self, value: FocusProps) -> Self {
    value.apply(self.wrapper.state.host.visual_element_mut());
    self.validate_modal_focus_properties();
    self
  }

  /// Attaches the wrapper's single semantic declaration.
  #[must_use]
  pub fn semantic(mut self, value: SemanticProps) -> Self {
    assert!(
      self.dialog_name.is_none(),
      "modal dialog semantics are intrinsic"
    );
    self.wrapper = self.wrapper.semantic(value);
    self
  }

  /// Merges advanced ordinary interaction callbacks.
  #[must_use]
  pub fn interaction_props<G: 'static>(mut self, value: InteractionProps<G>) -> Self {
    self.wrapper = self.wrapper.interaction_props(value);
    self
  }

  /// Sets authored inertness on a non-modal overlay wrapper.
  #[must_use]
  pub fn inert(mut self, value: bool) -> Self {
    assert!(
      !matches!(
        self.wrapper.state.overlay_reference,
        Some(OverlayReference::Modal { .. })
      ) || !value,
      "a modal wrapper cannot be inert"
    );
    self.wrapper = self.wrapper.inert(value);
    self
  }

  fn new(
    target: PortalTarget,
    placement: Prop<OverlayPlacement>,
    reference: Option<OverlayReference>,
    focusable: bool,
    fills_host: bool,
  ) -> Self {
    let alignment = if fills_host {
      Align::Stretch
    } else {
      Align::FlexStart
    };
    let mut wrapper = View::new()
      .picking_mode(PickingMode::Ignore)
      .focusable(focusable)
      .tab_index(-1)
      .stack_item(
        StackItem::new()
          .align_self(alignment)
          .justify_self(alignment)
          .contributes_to_size(false),
      )
      .overlay_placement(placement);
    wrapper.state.overlay_reference = reference;
    Self {
      dialog_name: None,
      on_dismiss: None,
      target,
      wrapper,
    }
  }

  fn validate_modal_focus_properties(&self) {
    if !matches!(
      self.wrapper.state.overlay_reference,
      Some(OverlayReference::Modal { .. })
    ) {
      return;
    }
    let visual = self.wrapper.state.host.visual_element();
    assert_ne!(
      visual.enabled,
      Prop::Set(false),
      "a modal wrapper must remain enabled"
    );
    assert_eq!(
      visual.focusable,
      Prop::Set(true),
      "a modal wrapper must remain focusable"
    );
    assert_eq!(
      visual.tab_index,
      Prop::Set(-1),
      "a modal wrapper must retain tab index -1"
    );
    assert_ne!(
      visual.inert,
      Prop::Set(true),
      "a modal wrapper cannot be inert"
    );
  }
}

impl Render for OverlayHost {}

impl Sealed for OverlayHost {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    self.host.render_into(sink);
  }
}

impl Render for Overlay {}

impl Sealed for Overlay {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    let wrapper = match &self.dialog_name {
      Some(name) => {
        let behavior = control_behavior::dialog(name.clone(), self.on_dismiss.clone());
        self.wrapper.clone().behavior(behavior)
      }
      None => self.wrapper.clone(),
    };
    create_portal(wrapper, self.target.clone()).render_into(sink);
  }
}

fn collect_modals(tree: &RenderTree, values: &mut Vec<battlement::ObjectId>) {
  for position in &tree.positions {
    if let Some(host) = &position.host
      && matches!(
        host.element.visual_element().overlay_placement,
        Prop::Set(OverlayPlacement::Modal { .. })
      )
    {
      values.push(host.object_id);
    }
    self::collect_modals(&position.children, values);
  }
}

fn apply_order(
  tree: &mut RenderTree,
  ranks: &HashMap<battlement::ObjectId, i32>,
  inherited_rank: i32,
) {
  for position in &mut tree.positions {
    let mut child_rank = inherited_rank;
    if let Some(host) = &mut position.host {
      let overlay = host.element.visual_element().overlay_placement.clone();
      let order = match overlay {
        Prop::Set(OverlayPlacement::Modal { .. }) => {
          child_rank = ranks[&host.object_id];
          Some(child_rank * 2)
        }
        Prop::Set(OverlayPlacement::Layer(_) | OverlayPlacement::Popover { .. }) => {
          Some(inherited_rank * 2 + 1)
        }
        Prop::Unset | Prop::Reset => None,
      };
      if let Some(order) = order {
        let visual = host.element.visual_element_mut();
        let Prop::Set(mut item) = visual.stack_item else {
          panic!("Reactant Overlay wrapper lost its private StackItem");
        };
        item.order = order;
        visual.stack_item = Prop::Set(item);
      }
    }
    self::apply_order(&mut position.children, ranks, child_rank);
  }
}
