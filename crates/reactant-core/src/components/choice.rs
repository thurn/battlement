use battlement::Style;
use trox::LocalizedString;

use crate::{
  callback::{Callback, IntoCallback},
  component::Component,
  context::ContextProvider,
  control_behavior,
  element_ref::{self, ElementRef},
  hooks,
  host::{GroupBox, RadioButton, TabHost, TabView, View},
  prelude::builder,
  props::Missing,
  render::{Children, Render},
  semantics::{SemanticName, SemanticProps},
};

#[builder]
#[derive(Clone)]
struct TabListContext {
  #[builder(required)]
  callback: Callback<u32>,
  #[builder(required)]
  reference: ElementRef,
  #[builder(required)]
  selected_index: u32,
}

impl PartialEq for TabListContext {
  fn eq(&self, other: &Self) -> bool {
    self.reference == other.reference
      && self.selected_index == other.selected_index
      && self.callback.same_identity(&other.callback)
  }
}

/// A native semantic radio group that owns descendant membership.
pub struct RadioGroup {
  children: Children,
  host: GroupBox,
  name: LocalizedString,
}

/// A controlled native radio option.
pub struct Radio<C = Missing> {
  callback: C,
  disabled: bool,
  host: RadioButton,
  name: LocalizedString,
  selected: bool,
}

/// A native semantic tab list that owns descendant tab membership.
pub struct Tabs<C = Missing> {
  callback: C,
  children: Children,
  host: TabView,
  name: LocalizedString,
  selected_index: u32,
}

/// A controlled native tab page.
pub struct Tab {
  disabled: bool,
  host: TabHost,
  index: u32,
  name: LocalizedString,
}

/// The controlled panel associated with the nearest [`Tabs`] component.
pub struct TabPanel {
  children: Children,
  host: View,
  tab_index: u32,
}

impl RadioGroup {
  /// Creates an empty named radio group.
  pub fn new(name: LocalizedString) -> Self {
    Self {
      children: Children::new(()),
      host: GroupBox::new().text(name.clone()),
      name,
    }
  }

  /// Sets the complete logical radio content.
  pub fn child(mut self, child: impl Render) -> Self {
    self.children = Children::new(child);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }
}

impl Component for RadioGroup {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    self
      .host
      .clone()
      .element_ref(reference.clone())
      .semantic(
        SemanticProps::new(battlement::SemanticRole::RadioGroup)
          .name(SemanticName::Text(self.name.clone())),
      )
      .child(
        ContextProvider::new()
          .context(reference)
          .child(self.children.render()),
      )
  }
}

impl Radio<Missing> {
  /// Creates a controlled native radio option.
  pub fn new(name: LocalizedString, selected: bool) -> Self {
    Self {
      callback: Missing,
      disabled: false,
      host: RadioButton::new().label(name.clone()).value(selected),
      name,
      selected,
    }
  }

  /// Supplies the authoritative selection callback.
  pub fn on_select<G: 'static>(self, callback: impl IntoCallback<(), G>) -> Radio<Callback<()>> {
    Radio {
      callback: callback.into_callback(),
      disabled: self.disabled,
      host: self.host,
      name: self.name,
      selected: self.selected,
    }
  }
}

impl<C> Radio<C> {
  /// Sets whether every selection route is unavailable.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Applies advanced native RadioButton customization.
  pub fn configure_host(mut self, configure: impl FnOnce(RadioButton) -> RadioButton) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Radio<Callback<()>> {
  fn render(&self) -> impl Render {
    let group = hooks::use_required_context::<ElementRef>();
    let disabled = self.disabled;
    self
      .host
      .clone()
      .label(self.name.clone())
      .value(self.selected)
      .enabled(!self.disabled)
      .behavior(control_behavior::radio_member(
        group,
        SemanticName::Text(self.name.clone()),
        self.selected,
        self.disabled,
        self.callback.clone(),
      ))
      .on_change_value(
        self
          .callback
          .clone()
          .filter_map_input(move |selected: bool| (selected && !disabled).then_some(())),
      )
  }
}

impl Tabs<Missing> {
  /// Creates an empty controlled native tab list.
  pub fn new(name: LocalizedString, selected_index: u32) -> Self {
    Self {
      callback: Missing,
      children: Children::new(()),
      host: TabView::new().selected_tab_index(selected_index),
      name,
      selected_index,
    }
  }

  /// Supplies the authoritative selected-index callback for every tab route.
  pub fn on_select<G: 'static>(self, callback: impl IntoCallback<u32, G>) -> Tabs<Callback<u32>> {
    Tabs {
      callback: callback.into_callback(),
      children: self.children,
      host: self.host,
      name: self.name,
      selected_index: self.selected_index,
    }
  }
}

impl<C> Tabs<C> {
  /// Sets the complete logical tab content.
  pub fn child(mut self, child: impl Render) -> Self {
    self.children = Children::new(child);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Applies advanced native TabView customization.
  pub fn configure_host(mut self, configure: impl FnOnce(TabView) -> TabView) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Tabs<Callback<u32>> {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    self
      .host
      .clone()
      .selected_tab_index(self.selected_index)
      .on_change_value(self.callback.clone())
      .element_ref(reference.clone())
      .semantic(
        SemanticProps::new(battlement::SemanticRole::TabList)
          .name(SemanticName::Text(self.name.clone())),
      )
      .child(
        ContextProvider::new()
          .context(
            TabListContext::new()
              .callback(self.callback.clone())
              .reference(reference)
              .selected_index(self.selected_index),
          )
          .child(self.children.render()),
      )
  }
}

impl Tab {
  /// Creates a controlled native tab page.
  pub fn new(name: LocalizedString, index: u32) -> Self {
    Self {
      disabled: false,
      host: TabHost::new(name.clone()),
      name,
      index,
    }
  }

  /// Sets whether every selection route is unavailable.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Appends native tab page content.
  pub fn child(mut self, child: impl Render) -> Self {
    self.host = self.host.child(child);
    self
  }

  /// Applies advanced native Tab customization.
  pub fn configure_host(mut self, configure: impl FnOnce(TabHost) -> TabHost) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Tab {
  fn render(&self) -> impl Render {
    let tabs = hooks::use_required_context::<TabListContext>();
    let index = self.index;
    self
      .host
      .clone()
      .text(self.name.clone())
      .enabled(!self.disabled)
      .behavior(control_behavior::tab_member(
        tabs.reference,
        SemanticName::Text(self.name.clone()),
        tabs.selected_index == self.index,
        self.disabled,
        tabs.callback.map_input(move |()| index),
      ))
  }
}

impl TabPanel {
  /// Creates a controlled tab panel for the nearest tab list.
  pub fn new(tab_index: u32, children: impl Into<Children>) -> Self {
    Self {
      children: children.into(),
      host: View::new(),
      tab_index,
    }
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Replaces the native panel host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }
}

impl Component for TabPanel {
  fn render(&self) -> impl Render {
    let tabs = hooks::use_required_context::<TabListContext>();
    self
      .host
      .clone()
      .semantic(control_behavior::tab_panel_for(
        tabs.reference,
        tabs.selected_index == self.tab_index,
      ))
      .child(self.children.render())
  }
}
