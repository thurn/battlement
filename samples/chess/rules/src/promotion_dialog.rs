//! Presentation for the exact promotion request owned by the rules worker.

use crate::{chess_labels, settings};
use battlement::WhiteSpace;
use cozy_chess::Piece;
use reactant::{
  app_context,
  prelude::{
    Align, Button, Color, Component, FlexDirection, Justify, KeyRenderExt, Label, Position, Render,
    Style, View,
  },
  rules::ResponseHandle,
};
use trox::{tx_args, txa};

use crate::{
  chess_prompt::{ChessPrompt, PromotionPrompt},
  reactant_game::ChessGame,
};

/// Prompt-aware component that appears only while promotion awaits a response.
pub struct PromotionDialog;

/// Owned prompt data and response capability for the visible choice buttons.
struct PromotionChoices {
  data: PromotionPrompt,
  handle: ResponseHandle<ChessPrompt<'static>>,
}

impl Component for PromotionDialog {
  /// Projects the current typed rules prompt into a keyed child component.
  ///
  /// Keying by response handle gives each prompt a fresh component identity and
  /// prevents a late click from being confused with a later promotion.
  fn render(&self) -> impl Render {
    reactant::use_game_prompt::<ChessGame>().map(|presented| {
      let ChessPrompt::Promotion(data) = &presented.prompt;
      PromotionChoices {
        data: data.as_ref().clone(),
        handle: presented.handle.clone(),
      }
      .key(presented.handle.clone())
    })
  }
}

impl Component for PromotionChoices {
  /// Renders every response advertised by [`PromotionPrompt`] as a button.
  fn render(&self) -> impl Render {
    let scale = settings::use_settings().desired.text_size.factor();
    let screen = app_context::use_viewport_size();
    let width = (screen.width as f32 - 48.0).min(360.0 * scale).max(1.0);
    View::new()
      .name("promotion-dialog")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(24)
          .top(24.0 + 48.0 * scale)
          .width(width)
          .padding(18)
          .flex_direction(FlexDirection::Column)
          .align_items(Align::Stretch)
          .justify_content(Justify::Center)
          .background_color(Color::rgba(0.03, 0.04, 0.035, 0.96))
          .border_radius(8),
      )
      .child((
        Label::new(txa(
          "Promote {from} to {to}",
          tx_args![from => self.data.from.to_string(), to => self.data.to.to_string()],
          "Promotion move prompt.",
        ))
        .style(
          Style::new()
            .font_size(22.0 * scale)
            .color(Color::WHITE)
            .white_space(WhiteSpace::Normal)
            .margin_bottom(10),
        ),
        View::new()
          .style(
            Style::new()
              .flex_direction(if scale > 1.0 || width < 356.0 {
                FlexDirection::Column
              } else {
                FlexDirection::Row
              })
              .justify_content(Justify::SpaceBetween),
          )
          .child((
            self::choice("Queen", Piece::Queen, &self.data, &self.handle, scale),
            self::choice("Rook", Piece::Rook, &self.data, &self.handle, scale),
            self::choice("Bishop", Piece::Bishop, &self.data, &self.handle, scale),
            self::choice("Knight", Piece::Knight, &self.data, &self.handle, scale),
          )),
      ))
  }
}

/// Builds a button that submits through the prompt's capability-checked handle.
///
/// The UI does not dispatch a custom promotion action. Submitting a typed response
/// resumes the already-running rules action at its `choose` call.
fn choice(
  label: &'static str,
  piece: Piece,
  data: &PromotionPrompt,
  handle: &ResponseHandle<ChessPrompt<'static>>,
  scale: f32,
) -> impl Render {
  let data = data.clone();
  let handle = handle.clone();
  Button::new(chess_labels::promotion(piece))
    .host_name(format!("promote-{}", label.to_ascii_lowercase()))
    .style(
      Style::new()
        .min_width(76.0 * scale)
        .height(42.0 * scale)
        .font_size(14.0 * scale)
        .margin(2),
    )
    .on_press(move || handle.submit::<ChessGame, PromotionPrompt>(&data, piece))
}
