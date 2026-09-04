use trox::LocalizedString;

use battlement::SemanticRole;

use crate::{
  component::Component,
  context::RequiredContext,
  control_behavior,
  element_ref::{self, ElementRef},
  hooks,
  host::{ButtonHost, View},
  prelude::{EventCallback, builder},
  render::{Children, Render},
  semantics::{SemanticMembership, SemanticName, SemanticProps},
};

static STRIP: RequiredContext<StripContext> = RequiredContext::new();

/// A controlled strip of independently styled tab buttons, without native page containers.
///
/// Owns descendant membership and selection proposals. Directional navigation and
/// panel composition remain application choices; every enabled tab is a Tab stop.
#[builder]
pub struct TabStrip {
  #[builder(required)]
  /// Accessible name.
  label: LocalizedString,
  #[builder(required)]
  /// Index selected by the parent.
  selected_index: u32,
  #[builder(required)]
  /// Receives each activation proposal.
  on_select: EventCallback<u32>,
  #[builder(required, into)]
  /// Tab buttons in display order.
  children: Children,
  #[builder(default = View::new())]
  /// Styled host for the tab list.
  host: View,
}

/// An ordinary button that proposes its index to the nearest [`TabStrip`].
///
/// Activating the selected tab still proposes selection. The parent decides
/// whether to accept a proposal; the button never stores a selected value.
#[builder]
pub struct TabButton {
  #[builder(required)]
  /// Accessible name.
  label: LocalizedString,
  #[builder(required)]
  /// Index proposed when activated.
  index: u32,
  /// Disables every activation route.
  disabled: bool,
  #[builder(default = ButtonHost::new(trox::ls("")))]
  /// Styled host for the tab button.
  host: ButtonHost,
}

#[derive(Clone)]
struct StripContext {
  reference: ElementRef,
  /// Index selected by the parent.
  selected_index: u32,
  /// Receives each activation proposal.
  on_select: EventCallback<u32>,
}

impl PartialEq for StripContext {
  fn eq(&self, other: &Self) -> bool {
    self.reference == other.reference
      && self.selected_index == other.selected_index
      && self.on_select.same_identity(&other.on_select)
  }
}

impl Component for TabStrip {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    self
      .host
      .clone()
      .element_ref(reference.clone())
      .semantic(
        SemanticProps::new(SemanticRole::TabList).name(SemanticName::Text(self.label.clone())),
      )
      .child(
        STRIP
          .provider(StripContext {
            reference,
            selected_index: self.selected_index,
            on_select: self.on_select.clone(),
          })
          .child(self.children.render()),
      )
  }
}

impl Component for TabButton {
  fn render(&self) -> impl Render {
    let strip = hooks::use_required_context(&STRIP);
    self
      .host
      .clone()
      .text(self.label.clone())
      .enabled(!self.disabled)
      .behavior(
        control_behavior::button(
          SemanticName::Text(self.label.clone()),
          None,
          self.disabled,
          strip.on_select.map_input({
            let index = self.index;
            move |()| index
          }),
        )
        .map_semantic(|mut semantic| {
          semantic.role = SemanticRole::Tab;
          semantic.state.selected = Some(strip.selected_index == self.index);
          semantic.membership = Some(SemanticMembership::Tab(strip.reference));
          semantic
        }),
      )
  }
}
