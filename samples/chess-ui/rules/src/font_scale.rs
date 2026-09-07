//! Shared text-size state and source growth formulas.

use battlement_reactant::{hooks, prelude::*};

#[derive(Clone, PartialEq)]
struct FontScaleContext {
  scale: FontScale,
  set_scale: StateSetter<FontScale>,
}

/// Owns the complete application's text-size selection.
#[builder]
pub struct FontScaleProvider {
  #[builder(required, into)]
  children: Children,
}

impl Component for FontScaleProvider {
  fn render(&self) -> impl Render {
    let (scale, set_scale) = hooks::use_state(FontScale::Percent100);
    ContextProvider::new()
      .context(Some(FontScaleContext { scale, set_scale }))
      .child(self.children.render())
  }
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
  Body,
  Control,
  Navigation,
  Heading,
}

/// Provides one text-size selection to a logical descendant tree.
pub fn provider(scale: FontScale, child: impl Render) -> impl Render {
  ContextProvider::new().context(Some(scale)).child(child)
}

/// Reads the current text size, defaulting to 100% outside a provider.
pub fn use_font_scale() -> FontScale {
  let state = hooks::use_context::<Option<FontScaleContext>>();
  let fixed = hooks::use_context::<Option<FontScale>>();
  state
    .map(|context| context.scale)
    .or(fixed)
    .unwrap_or_default()
}

/// Reads the complete application's text-size value and setter.
pub fn use_font_scale_state() -> (FontScale, StateSetter<FontScale>) {
  let context = hooks::use_context::<Option<FontScaleContext>>();
  let fallback = hooks::use_state(FontScale::Percent100);
  context
    .map(|context| (context.scale, context.set_scale))
    .unwrap_or(fallback)
}

/// Reads the nearest text size unless an isolated specimen supplies an override.
pub fn use_font_scale_override(override_scale: Option<FontScale>) -> FontScale {
  let context_scale = use_font_scale();
  override_scale.unwrap_or(context_scale)
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
          FontScaleRole::Body => 1.0,
          FontScaleRole::Control => 0.65,
          FontScaleRole::Navigation => 0.45,
          FontScaleRole::Heading => 0.2,
        }
  }
}
