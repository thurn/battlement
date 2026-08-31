use battlement::{
  Align, FlexDirection, FlexWrap, LengthUnits, Style, TextAnchor, TextureAddress, WhiteSpace,
};
use battlement_reactant::prelude::*;

use crate::{Control, Game, Interaction, design_system};

battlement_reactant::asset_generator::generate! {
  @text-image ADVANCED_TITLE {
    @canvas 420px 64px;
    @subject 10px 6px 400px 50px;
    @font-file unity("Assets/Original/Command Mono.ttf");
    content: "GENERATED PAINT";
    font-size: 36px;
    font-weight: 700;
    color: transparent;
    background: linear-gradient(90deg, #67e8f9 0%, #fbbf24 48%, #c084fc 100%);
    background-clip: text;
    text-shadow: 0 2px 3px rgba(0, 0, 0, 0.65);
  }
}

battlement_reactant::asset_generator::generate! {
  @background LAYERED_CLIP {
    @canvas 300px 118px;
    @subject 16px 14px 266px 86px;
    background: radial-gradient(circle at 24% 30%, rgba(103, 232, 249, 0.95), transparent 42%), linear-gradient(135deg, #0f766e, #312e81);
    background-blend-mode: screen;
    clip-path: polygon(0% 16%, 12% 0%, 88% 0%, 100% 16%, 94% 100%, 6% 100%);
    box-shadow: 0 6px 8px rgba(0, 0, 0, 0.55);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice RESIZABLE_FRAME {
    @canvas 96px 64px;
    @subject 12px 10px 72px 42px;
    @slices 14px 14px 14px 14px;
    @raster-scale 3;
    background: linear-gradient(135deg, rgba(8, 47, 73, 0.96), rgba(30, 41, 59, 0.96));
    border: 4px solid #67e8f9;
    border-radius: 12px;
    box-shadow: inset 0 0 8px rgba(251, 191, 36, 0.55);
  }
}

battlement_reactant::asset_generator::generate! {
  @background GRADIENT_SWATCH {
    @canvas 112px 70px;
    @subject 4px 4px 104px 62px;
    background: conic-gradient(from 35deg at 50% 50%, #22d3ee, #a78bfa, #fbbf24, #22d3ee);
    border-radius: 12px;
  }
}

battlement_reactant::asset_generator::generate! {
  @background CLIP_SWATCH {
    @canvas 112px 70px;
    @subject 6px 6px 100px 58px;
    background: linear-gradient(120deg, #fbbf24, #f97316);
    clip-path: polygon(0% 50%, 18% 0%, 82% 0%, 100% 50%, 82% 100%, 18% 100%);
  }
}

battlement_reactant::asset_generator::generate! {
  @background SHADOW_SWATCH {
    @canvas 112px 70px;
    @subject 16px 10px 78px 34px;
    background: linear-gradient(135deg, rgb(14, 165, 233), #4338ca);
    border-radius: 10px;
    box-shadow: inset 0 0 8px rgba(255, 255, 255, 0.5), 5px 6px 8px rgba(0, 0, 0, 0.65);
  }
}

battlement_reactant::asset_generator::generate! {
  @background MASK_SWATCH {
    @canvas 112px 70px;
    @subject 4px 4px 104px 62px;
    background: linear-gradient(90deg, #22d3ee, #c084fc);
    mask: radial-gradient(circle at 50% 50%, white 24%, transparent 72%) alpha;
  }
}

battlement_reactant::asset_generator::generate! {
  @background FILTER_SWATCH {
    @canvas 112px 70px;
    @subject 16px 12px 78px 42px;
    background: linear-gradient(135deg, #f43f5e, #fbbf24);
    border-radius: 9px;
    filter: saturate(1.45) contrast(1.18) drop-shadow(3px 4px 3px rgba(0, 0, 0, 0.55));
  }
}

battlement_reactant::asset_generator::generate! {
  @background SKEW_SWATCH {
    @canvas 112px 70px;
    @subject 12px 8px 88px 54px;
    background: linear-gradient(135deg, #34d399, #0f766e);
    border-radius: 8px;
    transform: skew(-12deg, 2deg);
  }
}

pub(crate) struct Assets {
  pub(crate) resized: bool,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for Assets {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("assets-canvas")
      .style(self::canvas(self.compact))
      .child(battlement_reactant::host::Label::new("ASSETS").style(design_system::eyebrow()))
      .child(
        ADVANCED_TITLE
          .image()
          .name("assets-gradient-title")
          .style(self::title(self.compact)),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("assets-gallery")
          .style(self::gallery())
          .child(self::specimen(
            "LAYERED + CLIPPED",
            LAYERED_CLIP.background_style(),
          ))
          .child(self::specimen(
            "GRADIENT",
            GRADIENT_SWATCH.background_style(),
          ))
          .child(self::specimen("CLIP", CLIP_SWATCH.background_style()))
          .child(self::specimen("SHADOW", SHADOW_SWATCH.background_style()))
          .child(self::specimen("MASK", MASK_SWATCH.background_style()))
          .child(self::specimen("FILTER", FILTER_SWATCH.background_style()))
          .child(if self.resized {
            Node::new(self::specimen(
              "SKEW / LATER STATE",
              SKEW_SWATCH.background_style(),
            ))
          } else {
            Node::new(Fragment::new(()))
          }),
      )
      .child(crate::interactive_button(
        if self.resized {
          "RESTORE NINE-SLICE"
        } else {
          "RESIZE NINE-SLICE"
        },
        "assets-resize-action",
        RESIZABLE_FRAME
          .background_style()
          .merge(self::resize_action(
            self.resized,
            crate::control_state(self.interaction, Control::AssetsAction),
          )),
        Control::AssetsAction,
        |game: &mut Game| game.assets_resized = !game.assets_resized,
      ))
  }
}

pub(crate) fn addresses() -> Vec<TextureAddress> {
  [
    ADVANCED_TITLE.texture_address(),
    LAYERED_CLIP.texture_address(),
    RESIZABLE_FRAME.texture_address(),
    GRADIENT_SWATCH.texture_address(),
    CLIP_SWATCH.texture_address(),
    SHADOW_SWATCH.texture_address(),
    MASK_SWATCH.texture_address(),
    FILTER_SWATCH.texture_address(),
    SKEW_SWATCH.texture_address(),
  ]
  .into_iter()
  .collect()
}

fn specimen(label: &'static str, paint: Style) -> impl Render {
  battlement_reactant::host::View::new()
    .style(self::specimen_card())
    .child(battlement_reactant::host::Label::new(label).style(self::specimen_label()))
    .child(battlement_reactant::host::View::new().style(paint.merge(self::swatch())))
}

fn canvas(compact: bool) -> Style {
  design_system::canvas(compact).align_items(Align::FlexStart)
}

fn title(compact: bool) -> Style {
  Style::new()
    .width(if compact { 315.0 } else { 420.0 })
    .height(if compact { 48.0 } else { 64.0 })
    .margin((2, 0, 10, 0))
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .max_width(920.0)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn specimen_card() -> Style {
  Style::new()
    .width(146.0)
    .height(122.0)
    .background_color(design_system::SPECIMEN_BACKGROUND)
    .padding(10.0)
    .margin((0, 10, 10, 0))
}

fn specimen_label() -> Style {
  Style::new()
    .height(28.0)
    .font_size(14.0)
    .color(design_system::MUTED_TEXT)
    .unity_text_align(TextAnchor::MiddleCenter)
    .white_space(WhiteSpace::NoWrap)
}

fn swatch() -> Style {
  Style::new()
    .width(112.0)
    .height(70.0)
    .align_self(Align::Center)
}

fn resize_action(resized: bool, state: design_system::ControlState) -> Style {
  design_system::primary_action(state)
    .width(if resized { 520.0 } else { 300.0 })
    .height(if resized { 92.0 } else { 64.0 })
    .color(design_system::PRIMARY_TEXT)
    .font_size(20.0)
    .margin((8, 0, 0, 0))
}
