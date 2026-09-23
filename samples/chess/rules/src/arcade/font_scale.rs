//! Shared text-size state and source growth formulas.

use reactant::{hooks, prelude::*};

/// Owns the complete application's text-size selection.
#[builder]
pub struct FontScaleProvider {
  #[builder(required, into)]
  children: Children,
}

/// Player-selectable text sizes from the source application.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FontScale {
  #[default]
  Percent100,
  Percent150,
  Percent200,
}

/// Text roles with distinct source growth rates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontScaleRole {
  Control,
  Navigation,
  Heading,
}

/// Reads the complete application's current text size.
pub fn use_font_scale() -> FontScale {
  hooks::use_required_context::<FontScaleContext>().scale
}

/// Reads the complete application's text-size value and setter.
pub fn use_font_scale_state() -> (FontScale, StateSetter<FontScale>) {
  let context = hooks::use_required_context::<FontScaleContext>();
  (context.scale, context.set_scale)
}

#[derive(Clone, PartialEq)]
struct FontScaleContext {
  scale: FontScale,
  set_scale: StateSetter<FontScale>,
}

impl Component for FontScaleProvider {
  fn render(&self) -> impl Render {
    let (scale, set_scale) = hooks::use_state(FontScale::Percent100);
    ContextProvider::new()
      .context(FontScaleContext { scale, set_scale })
      .child(self.children.render())
  }
}

impl FontScale {
  /// All supported sizes in selector order.
  pub const ALL: [Self; 3] = [Self::Percent100, Self::Percent150, Self::Percent200];

  /// Returns the numeric scaling factor.
  pub const fn factor(self) -> f32 {
    match self {
      Self::Percent100 => 1.0,
      Self::Percent150 => 1.5,
      Self::Percent200 => 2.0,
    }
  }

  /// Returns the visible percentage label.
  pub const fn label(self) -> &'static str {
    match self {
      Self::Percent100 => "100%",
      Self::Percent150 => "150%",
      Self::Percent200 => "200%",
    }
  }

  /// Applies the source growth curve for a text role.
  pub fn dynamic(self, role: FontScaleRole) -> f32 {
    let growth = self.factor() - 1.0;
    1.0
      + growth
        * match role {
          FontScaleRole::Control => 0.65,
          FontScaleRole::Navigation => 0.45,
          FontScaleRole::Heading => 0.2,
        }
  }
}
