use battlement::{ImageSource, Prop, SemanticRole, Style};
use trox::LocalizedString;

use crate::{
  component::Component,
  host::{ImageHost, Label, TextElement, View},
  render::Render,
  semantics::{SemanticName, SemanticProps, SemanticVisibility},
};

/// A named single-selection list whose children are [`crate::components::ListBoxOption`] values.
pub type ListBox = SemanticContainer<ListBoxKind>;

/// A named semantic table whose children are [`TableRow`] values.
pub type Table = SemanticContainer<TableKind>;

/// One semantic table row.
pub type TableRow = SemanticContainer<TableRowKind>;

/// A named navigation landmark.
pub type Navigation = SemanticContainer<NavigationKind>;

/// A named content region.
pub type Region = SemanticContainer<RegionKind>;

/// An optional named semantic group.
pub type Group = SemanticContainer<GroupKind>;

/// A named data cell in its nearest semantic table row.
pub type TableCell = SemanticText<TableCellKind>;

/// A header scoped to its semantic table column.
pub type ColumnHeader = SemanticText<ColumnHeaderKind>;

/// A header scoped to its semantic table row.
pub type RowHeader = SemanticText<RowHeaderKind>;

#[doc(hidden)]
pub struct SemanticContainer<K> {
  host: View,
  kind: K,
}

#[doc(hidden)]
pub struct SemanticText<K> {
  host: Label,
  _kind: K,
  name: LocalizedString,
}

#[doc(hidden)]
pub struct ListBoxKind(LocalizedString);

#[doc(hidden)]
pub struct TableKind(LocalizedString);

#[doc(hidden)]
pub struct TableRowKind;

#[doc(hidden)]
pub struct NavigationKind(LocalizedString);

#[doc(hidden)]
pub struct RegionKind(LocalizedString);

#[doc(hidden)]
pub struct GroupKind(Option<LocalizedString>);

#[doc(hidden)]
pub struct TableCellKind;

#[doc(hidden)]
pub struct ColumnHeaderKind;

#[doc(hidden)]
pub struct RowHeaderKind;

/// A semantic heading rendered with Unity's native Label.
pub struct Heading {
  host: Label,
  level: u8,
  name: LocalizedString,
}

/// An informative image rendered with Unity's native Image host.
pub struct Image {
  host: ImageHost,
  name: LocalizedString,
  source: Prop<ImageSource>,
}

/// Visible static text with semantic-tree participation enabled by default.
pub struct Text {
  host: TextElement,
  value: LocalizedString,
  visibility: SemanticVisibility,
}

trait ContainerKind: 'static {
  fn semantic(&self) -> SemanticProps;
}

trait TextKind: 'static {
  fn role() -> SemanticRole;
}

impl ListBox {
  /// Creates a named single-selection list box.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(ListBoxKind(name))
  }
}

impl Table {
  /// Creates a named semantic table.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(TableKind(name))
  }
}

impl TableRow {
  /// Creates one semantic table row.
  pub fn new() -> Self {
    Self::with_kind(TableRowKind)
  }
}

impl Navigation {
  /// Creates a named navigation landmark.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(NavigationKind(name))
  }
}

impl Region {
  /// Creates a named content region.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(RegionKind(name))
  }
}

impl Group {
  /// Creates an optionally named semantic group.
  pub fn new(name: Option<LocalizedString>) -> Self {
    Self::with_kind(GroupKind(name))
  }
}

impl TableCell {
  /// Creates a named data cell.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(name, TableCellKind)
  }
}

impl ColumnHeader {
  /// Creates a named column header.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(name, ColumnHeaderKind)
  }
}

impl RowHeader {
  /// Creates a named row header.
  pub fn new(name: LocalizedString) -> Self {
    Self::with_kind(name, RowHeaderKind)
  }
}

impl<K> SemanticContainer<K> {
  fn with_kind(kind: K) -> Self {
    Self {
      host: View::new(),
      kind,
    }
  }

  /// Appends one logical child.
  pub fn child(mut self, child: impl Render) -> Self {
    self.host = self.host.child(child);
    self
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Applies advanced native View customization.
  pub fn configure_host(mut self, configure: impl FnOnce(View) -> View) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl<K> Component for SemanticContainer<K>
where
  K: ContainerKind,
{
  fn render(&self) -> impl Render {
    self.host.clone().semantic(self.kind.semantic())
  }
}

impl<K> SemanticText<K> {
  fn with_kind(name: LocalizedString, kind: K) -> Self {
    Self {
      host: Label::new(name.clone()),
      _kind: kind,
      name,
    }
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

  /// Applies advanced native Label customization.
  pub fn configure_host(mut self, configure: impl FnOnce(Label) -> Label) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl<K> Component for SemanticText<K>
where
  K: TextKind,
{
  fn render(&self) -> impl Render {
    self
      .host
      .clone()
      .semantic(SemanticProps::new(K::role()).name(SemanticName::Text(self.name.clone())))
  }
}

impl Heading {
  /// Creates a semantic heading at the given one-based level.
  pub fn new(name: LocalizedString, level: u8) -> Self {
    Self {
      host: Label::new(name.clone()),
      level,
      name,
    }
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }
}

impl Component for Heading {
  fn render(&self) -> impl Render {
    self.host.clone().semantic(
      SemanticProps::new(SemanticRole::Heading)
        .name(SemanticName::Text(self.name.clone()))
        .heading_level(self.level),
    )
  }
}

impl Image {
  /// Creates an informative image requiring a semantic name.
  pub fn new(name: LocalizedString) -> Self {
    Self {
      host: ImageHost::new(),
      name,
      source: Prop::Unset,
    }
  }

  /// Sets or resets the native image source.
  pub fn source(mut self, source: impl Into<Prop<ImageSource>>) -> Self {
    self.source = source.into();
    self
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Applies advanced native Image customization.
  pub fn configure_host(mut self, configure: impl FnOnce(ImageHost) -> ImageHost) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Image {
  fn render(&self) -> impl Render {
    self
      .host
      .clone()
      .source(self.source.clone())
      .semantic(SemanticProps::new(SemanticRole::Image).name(SemanticName::Text(self.name.clone())))
  }
}

impl Text {
  /// Creates exposed static text.
  pub fn new(value: LocalizedString) -> Self {
    Self {
      host: TextElement::new(value.clone()),
      value,
      visibility: SemanticVisibility::Exposed,
    }
  }

  /// Creates text used only as a content-derived semantic name source.
  pub fn name_source(value: LocalizedString) -> Self {
    Self {
      host: TextElement::new(value.clone()),
      value,
      visibility: SemanticVisibility::NameSourceOnly,
    }
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }
}

impl Component for Text {
  fn render(&self) -> impl Render {
    self.host.clone().semantic(
      SemanticProps::new(SemanticRole::StaticText)
        .name(SemanticName::Text(self.value.clone()))
        .visibility(self.visibility),
    )
  }
}

impl ContainerKind for ListBoxKind {
  fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::ListBox).name(SemanticName::Text(self.0.clone()))
  }
}

impl ContainerKind for TableKind {
  fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::Table).name(SemanticName::Text(self.0.clone()))
  }
}

impl ContainerKind for TableRowKind {
  fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::Row)
  }
}

impl ContainerKind for NavigationKind {
  fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::Navigation).name(SemanticName::Text(self.0.clone()))
  }
}

impl ContainerKind for RegionKind {
  fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::Region).name(SemanticName::Text(self.0.clone()))
  }
}

impl ContainerKind for GroupKind {
  fn semantic(&self) -> SemanticProps {
    self.0.clone().map_or_else(
      || SemanticProps::new(SemanticRole::Group),
      |name| SemanticProps::new(SemanticRole::Group).name(SemanticName::Text(name)),
    )
  }
}

impl TextKind for TableCellKind {
  fn role() -> SemanticRole {
    SemanticRole::Cell
  }
}

impl TextKind for ColumnHeaderKind {
  fn role() -> SemanticRole {
    SemanticRole::ColumnHeader
  }
}

impl TextKind for RowHeaderKind {
  fn role() -> SemanticRole {
    SemanticRole::RowHeader
  }
}
