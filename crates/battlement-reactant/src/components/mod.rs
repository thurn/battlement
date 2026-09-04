//! Complete controls and semantic structures for ordinary Reactant authoring.
//!
//! Actionable controls require their authoritative callback before rendering.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! fn require_render(_: impl Render) {}
//! require_render(Button::new(ls("Save")));
//! ```
//!
//! Composed button content also requires an explicit semantic name.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! fn require_render(_: impl Render) {}
//! require_render(Button::content(Text::new(ls("Save"))).on_press(|| {}));
//! ```
//!
//! Tab lists likewise require their one authoritative selection callback.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! fn require_render(_: impl Render) {}
//! require_render(Tabs::new(ls("Settings"), 0));
//! ```
//!
//! Raw native hosts are intentionally absent from the prelude.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! let _ = ButtonHost::new(ls("Save"));
//! ```

mod choice;
mod press;
mod range;
mod structure;
mod toggle;

pub use choice::{Radio, RadioGroup, Tab, TabPanel, Tabs};
pub use press::{Button, Disclosure, Link, ListBoxOption, PopupButton};
pub use range::{Progress, ScrollArea, Slider};
pub use structure::{
  ColumnHeader, Group, Heading, Image, ListBox, Navigation, Region, RowHeader, Table, TableCell,
  TableRow, Text,
};
pub use toggle::{Checkbox, Switch};

mod tab_strip;
pub use tab_strip::{TabButton, TabStrip};
