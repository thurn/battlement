use trox::{ls, tx};

use crate::{Control, Game, Interaction, design_system};
use battlement::{
  Align, FlexDirection, FlexWrap, ImageScaleMode, LengthUnits, ScrollViewMode, ScrollerVisibility,
  Style, TextAnchor, TextureAddress, WhiteSpace,
};
use battlement_reactant::prelude::*;

battlement_reactant::asset_generator::generate! {
  @background ARCADE_SCREEN_FRAME {
    @canvas 1024px 1536px;
    @subject 21px 21px 982px 1404px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 8px solid transparent;
    background: linear-gradient(110deg, #f4ffff 0%, #53dcff 4%, rgb(8, 116, 239) 12%, #09234c 18%, #19ddff 32%, #e9fbff 50%, #806cff 64%, #ff39c9 83%, #ffd4f4 96%, #ff5ec2 100%);
    clip-path: path("M44.19 0L144.354 0L166.94 26.676L815.06 26.676L837.646 0L937.81 0L982 44.928L982 262.548L963.342 280.8L963.342 1384.344L947.63 1404L34.37 1404L18.658 1384.344L18.658 280.8L0 262.548L0 44.928ZM8 52.416L8 267.556L26.354 285.6L26.354 1376.568L41.81 1396L940.19 1396L955.646 1376.568L955.646 285.6L974 267.556L974 52.416L930.53 8L831.998 8L809.78 34.372L172.22 34.372L150.002 8L51.47 8Z");
    filter: drop-shadow(0 0 10px #368dff24) drop-shadow(0 0 9px #ff2ac018);
  }
}

battlement_reactant::asset_generator::generate! {
  @background SETTINGS_PANEL_FRAME {
    @canvas 887px 1021px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 2px solid transparent;
    background: radial-gradient(ellipse at 7% 46%, #0553b826 0%, transparent 36%) border-box padding-box, linear-gradient(90deg, #0053be12 0%, transparent 25%, transparent 75%, #7e00910e 100%) border-box padding-box, linear-gradient(#041126 0%, #020b1b 100%) border-box padding-box, linear-gradient(110deg, #446690 0%, #2c456f 54%, #875984 100%);
    box-shadow: inset 0 0 45px #000000af;
    clip-path: polygon(0% 0%, 100% 0%, 100% 98.5%, 98.4% 100%, 1.5% 100%, 0% 98.5%);
    filter: drop-shadow(0 0 5px #1c59b447);
    isolation: isolate;
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice ACTION_BUTTON_FRAME {
    @canvas 760px 140px;
    @slices 24px 26px 24px 26px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 6px solid transparent;
    background: linear-gradient(#071027 0%, #020613 100%) border-box padding-box, linear-gradient(110deg, #b9fbff 0%, #3bb9ff 22%, #a49cff 56%, #ff4bd1 90%);
    box-shadow: inset 0 0 27px #000000af;
    clip-path: polygon(2.37% 0%, 97.63% 0%, 100% 12.14%, 100% 87.86%, 97.63% 100%, 2.37% 100%, 0% 87.86%, 0% 12.14%);
    filter: drop-shadow(0 0 10px #3a9affa6);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SMALL_CONTROL_FRAME {
    @canvas 396px 106px;
    @slices 15px 15px 15px 15px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 3px solid transparent;
    background: linear-gradient(#050b1c 0%, #020611 100%) border-box padding-box, linear-gradient(106deg, #5df5ff 0%, #a5cbff 48%, #ff4bc9 100%);
    box-shadow: inset 0 0 24px #000000af;
    clip-path: polygon(2.53% 0%, 97.47% 0%, 100% 9.43%, 100% 90.57%, 97.47% 100%, 2.53% 100%, 0% 90.57%, 0% 9.43%);
    filter: drop-shadow(0 0 6px #2a67ff61);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SETTINGS_TAB_ACTIVE {
    @canvas 288px 154px;
    @subject 12px 12px 264px 130px;
    @slices 30px 42px 18px 42px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid transparent;
    background: linear-gradient(#071831 0%, #030b1d 100%) border-box padding-box, linear-gradient(112deg, #72f5ff 0%, #53afff 44%, #9a83ff 68%, #ff4ed3 100%);
    box-shadow: inset 0 0 34px #000000b0, inset 0 -3px #f14dd7;
    clip-path: polygon(0% 13.85%, 6.82% 0%, 93.18% 0%, 100% 13.85%, 100% 100%, 0% 100%);
    filter: drop-shadow(0 0 10px #2385ff44);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice SETTINGS_TAB_INACTIVE {
    @canvas 288px 154px;
    @subject 12px 12px 264px 130px;
    @slices 30px 42px 18px 42px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid transparent;
    background: linear-gradient(#071328 0%, #020817 100%) border-box padding-box, linear-gradient(112deg, #72f5ff 0%, #53afff 44%, #9a83ff 68%, #ff4ed3 100%);
    box-shadow: inset 0 0 24px #000000b0, inset 0 0 3px #123b78a8;
    clip-path: polygon(0% 13.85%, 6.82% 0%, 93.18% 0%, 100% 13.85%, 100% 100%, 0% 100%);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image GAME_LOGO {
    @canvas 900px 360px;
    @subject 0px 45px 900px 250px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "CHESS CHESS\nREVOLUTION";
    font-size: 160px;
    line-height: 118px;
    letter-spacing: -4px;
    text-align: center;
    white-space: pre;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 2%, #e5f5ff 20%, #74c9ff 38%, #f8fbff 51%, #8d72ff 70%, #ff68d9 94%);
    background-clip: text;
    -webkit-text-stroke: 1.4px #f9ffff;
    filter: drop-shadow(4px 6px #092463) drop-shadow(-3px -2px #61096a) drop-shadow(0 12px 8px #000000);
    transform: scale(1.02, 0.9) skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_PLAY {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "PLAY";
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_SETTINGS {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "SETTINGS";
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_ABOUT {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "ABOUT";
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_QUIT {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "QUIT";
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @text-image ACTION_LABEL_RETURN {
    @canvas 480px 146px;
    @subject 0px 21px 480px 108px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    @font-file unity("Assets/Original/BarlowCondensed-ExtraBoldItalic.ttf");
    content: "RETURN";
    font-size: 91px;
    line-height: 108px;
    letter-spacing: -2px;
    text-align: center;
    white-space: nowrap;
    color: transparent;
    background: linear-gradient(174deg, #ffffff 5%, #dff8ff 31%, #52baff 49%, #f8faff 57%, rgb(128, 110, 255) 77%, #ff6dda 100%);
    background-clip: text;
    -webkit-text-stroke: 1px #f7ffff;
    filter: drop-shadow(3px 5px #122964) drop-shadow(0 7px 5px #000000);
    transform: skewX(-5deg);
  }
}

battlement_reactant::asset_generator::generate! {
  @background CHECKBOX_UNCHECKED {
    @canvas 101px 101px;
    @subject 12px 12px 77px 77px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid #4ba3ff;
    border-radius: 11px;
    background: linear-gradient(#06142b 0%, #02091a 100%);
    box-shadow: inset 0 0 14px #000000af;
    filter: drop-shadow(0 0 10px #166cff80) drop-shadow(0 0 5px #6af6ff70);
  }
}

battlement_reactant::asset_generator::generate! {
  @background CHECKBOX_CHECK {
    @canvas 101px 101px;
    @subject 25px 29px 50px 44px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    background: #61f1ff;
    clip-path: polygon(0% 47%, 14% 32%, 35% 58%, 85% 0%, 100% 14%, 35% 100%);
    filter: drop-shadow(0 0 7px #128dffb0);
  }
}

battlement_reactant::asset_generator::generate! {
  @nine-slice VOLUME_SLIDER_TRACK {
    @canvas 308px 88px;
    @subject 12px 31px 284px 26px;
    @slices 18px 18px 18px 18px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 3px solid transparent;
    border-radius: 8px;
    background: linear-gradient(#061125, #061125) border-box padding-box, linear-gradient(90deg, #13e7ff 0%, #735cff 47%, #ff43c7 76%, #ff326e 100%);
    box-shadow: inset 0 0 8px #000000af;
    filter: drop-shadow(0 0 9px #1868ffb8);
  }
}

battlement_reactant::asset_generator::generate! {
  @background VOLUME_SLIDER_FILL {
    @canvas 278px 20px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border-radius: 4px;
    background: linear-gradient(90deg, #17e9ff 0%, #286fff 35%, #8f5dff 62%, #ff3abe 86%, #ff326d 100%);
    filter: drop-shadow(0 0 8px #2d84ffcc);
  }
}

battlement_reactant::asset_generator::generate! {
  @background VOLUME_SLIDER_TICKS {
    @canvas 284px 10px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    background: repeating-linear-gradient(90deg, transparent 0px, transparent 62px, #465ccb 62px, #465ccb 64px);
  }
}

battlement_reactant::asset_generator::generate! {
  @background VOLUME_SLIDER_HANDLE {
    @canvas 68px 88px;
    @subject 12.5px 12px 43px 64px;
    @allow-clipping top right bottom left;
    @filter-mode trilinear;
    border: 4px solid transparent;
    background: linear-gradient(#07142b 0%, #02091b 100%) border-box padding-box, linear-gradient(135deg, #c8ffff 0%, #599cff 55%, #875fff 100%);
    box-shadow: inset 0 0 12px #000000af;
    clip-path: polygon(23% 0%, 77% 0%, 100% 17%, 100% 83%, 77% 100%, 23% 100%, 0% 83%, 0% 17%);
    filter: drop-shadow(0 0 7px #1479ff);
  }
}

#[builder]
pub(crate) struct Assets {
  pub(crate) resized: bool,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for Assets {
  fn render(&self) -> impl Render {
    battlement_reactant::host::ScrollView::new()
      .name("assets-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .vertical_scroller_style(design_system::effects_scroller())
      .vertical_low_button_style(design_system::effects_scroll_button())
      .vertical_high_button_style(design_system::effects_scroll_button())
      .vertical_track_style(design_system::effects_scroll_track())
      .vertical_dragger_style(design_system::effects_scroll_dragger())
      .vertical_dragger_border_style(design_system::effects_scroll_dragger())
      .style(self::canvas(self.compact))
      .content_container_style(self::content())
      .child(
        battlement_reactant::host::Label::new(tx(
          "MOCKUP ASSETS",
          "Mockup assets section heading.",
        ))
        .style(design_system::eyebrow()),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("assets-gallery")
          .style(self::gallery())
          .child(self::branding_card(self.compact))
          .child(self::frames_card(self.compact))
          .child(self::controls_card(
            self.compact,
            self.resized,
            self.interaction,
          )),
      )
  }
}

pub(crate) fn addresses() -> Vec<TextureAddress> {
  [
    ARCADE_SCREEN_FRAME.texture_address(),
    SETTINGS_PANEL_FRAME.texture_address(),
    ACTION_BUTTON_FRAME.texture_address(),
    SMALL_CONTROL_FRAME.texture_address(),
    SETTINGS_TAB_ACTIVE.texture_address(),
    SETTINGS_TAB_INACTIVE.texture_address(),
    GAME_LOGO.texture_address(),
    ACTION_LABEL_PLAY.texture_address(),
    ACTION_LABEL_SETTINGS.texture_address(),
    ACTION_LABEL_ABOUT.texture_address(),
    ACTION_LABEL_QUIT.texture_address(),
    ACTION_LABEL_RETURN.texture_address(),
    CHECKBOX_UNCHECKED.texture_address(),
    CHECKBOX_CHECK.texture_address(),
    VOLUME_SLIDER_TRACK.texture_address(),
    VOLUME_SLIDER_FILL.texture_address(),
    VOLUME_SLIDER_TICKS.texture_address(),
    VOLUME_SLIDER_HANDLE.texture_address(),
  ]
  .into_iter()
  .collect()
}

fn branding_card(compact: bool) -> impl Render {
  battlement_reactant::host::View::new()
    .style(self::card(if compact { 400.0 } else { 520.0 }))
    .child(self::card_title("BRANDING + BUTTON LABELS"))
    .child(
      GAME_LOGO
        .image()
        .name("assets-game-logo")
        .scale_mode(ImageScaleMode::ScaleToFit)
        .style(self::image(
          if compact { 330.0 } else { 450.0 },
          if compact { 132.0 } else { 180.0 },
        )),
    )
    .child(
      battlement_reactant::host::View::new()
        .style(self::label_grid())
        .child(self::label_image(
          ACTION_LABEL_PLAY,
          "assets-label-play",
          compact,
        ))
        .child(self::label_image(
          ACTION_LABEL_SETTINGS,
          "assets-label-settings",
          compact,
        ))
        .child(self::label_image(
          ACTION_LABEL_ABOUT,
          "assets-label-about",
          compact,
        ))
        .child(self::label_image(
          ACTION_LABEL_QUIT,
          "assets-label-quit",
          compact,
        ))
        .child(self::label_image(
          ACTION_LABEL_RETURN,
          "assets-label-return",
          compact,
        )),
    )
}

fn frames_card(compact: bool) -> impl Render {
  battlement_reactant::host::View::new()
    .style(self::card(if compact { 400.0 } else { 430.0 }))
    .child(self::card_title("SCREEN + PANEL FRAMES"))
    .child(
      battlement_reactant::host::View::new()
        .style(self::row())
        .child(
          ARCADE_SCREEN_FRAME
            .image()
            .name("assets-arcade-screen-frame")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 100.0 } else { 150.0 },
              if compact { 150.0 } else { 225.0 },
            )),
        )
        .child(
          SETTINGS_PANEL_FRAME
            .image()
            .name("assets-settings-panel-frame")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 130.0 } else { 195.0 },
              if compact { 150.0 } else { 225.0 },
            )),
        ),
    )
}

fn controls_card(compact: bool, resized: bool, interaction: Interaction) -> impl Render {
  battlement_reactant::host::View::new()
    .style(self::card(if compact { 400.0 } else { 650.0 }))
    .child(self::card_title("FRAMES + CONTROL PARTS"))
    .child(crate::interactive_button(
      if resized {
        "RESTORE ACTION FRAME"
      } else {
        "STRETCH ACTION FRAME"
      },
      "assets-resize-action",
      ACTION_BUTTON_FRAME
        .background_style()
        .merge(self::resize_action(
          compact,
          resized,
          crate::control_state(interaction, Control::AssetsAction),
        )),
      Control::AssetsAction,
      |game: &mut Game| game.assets_resized = !game.assets_resized,
    ))
    .child(
      battlement_reactant::host::View::new()
        .style(self::row())
        .child(
          battlement_reactant::host::View::new()
            .name("assets-small-control-frame")
            .style(
              SMALL_CONTROL_FRAME
                .background_style()
                .width(if compact { 128.0 } else { 198.0 })
                .height(if compact { 34.0 } else { 53.0 })
                .margin((4, 10)),
            ),
        )
        .child(
          SETTINGS_TAB_ACTIVE
            .image()
            .name("assets-settings-tab-active")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 96.0 } else { 144.0 },
              if compact { 51.0 } else { 77.0 },
            )),
        )
        .child(
          SETTINGS_TAB_INACTIVE
            .image()
            .name("assets-settings-tab-inactive")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 96.0 } else { 144.0 },
              if compact { 51.0 } else { 77.0 },
            )),
        ),
    )
    .child(
      battlement_reactant::host::View::new()
        .style(self::row())
        .child(
          CHECKBOX_UNCHECKED
            .image()
            .name("assets-checkbox-unchecked")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 42.0 } else { 64.0 },
              if compact { 42.0 } else { 64.0 },
            )),
        )
        .child(
          CHECKBOX_CHECK
            .image()
            .name("assets-checkbox-check")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 42.0 } else { 64.0 },
              if compact { 42.0 } else { 64.0 },
            )),
        )
        .child(
          VOLUME_SLIDER_TRACK
            .image()
            .name("assets-volume-slider-track")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 154.0 } else { 231.0 },
              if compact { 44.0 } else { 66.0 },
            )),
        )
        .child(
          VOLUME_SLIDER_HANDLE
            .image()
            .name("assets-volume-slider-handle")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 34.0 } else { 51.0 },
              if compact { 44.0 } else { 66.0 },
            )),
        ),
    )
    .child(
      battlement_reactant::host::View::new()
        .style(self::row())
        .child(
          VOLUME_SLIDER_FILL
            .image()
            .name("assets-volume-slider-fill")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 139.0 } else { 278.0 },
              if compact { 10.0 } else { 20.0 },
            )),
        )
        .child(
          VOLUME_SLIDER_TICKS
            .image()
            .name("assets-volume-slider-ticks")
            .scale_mode(ImageScaleMode::ScaleToFit)
            .style(self::image(
              if compact { 142.0 } else { 284.0 },
              if compact { 5.0 } else { 10.0 },
            )),
        ),
    )
}

fn label_image(
  asset: battlement_reactant::asset_generator::TextImageAsset,
  name: &'static str,
  compact: bool,
) -> impl Render {
  asset
    .image()
    .name(name)
    .scale_mode(ImageScaleMode::ScaleToFit)
    .style(self::image(
      if compact { 104.0 } else { 160.0 },
      if compact { 32.0 } else { 49.0 },
    ))
}

fn canvas(compact: bool) -> Style {
  design_system::canvas(compact)
    .align_items(Align::FlexStart)
    .padding(if compact { (14, 16) } else { (24, 28) })
}

fn content() -> Style {
  Style::new().width(100.0_f32.pct())
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn card(width: f32) -> Style {
  Style::new()
    .width(width)
    .background_color(design_system::SPECIMEN_BACKGROUND)
    .border_color(design_system::CYAN)
    .border_top_width(2.0)
    .padding(14.0)
    .margin((6, 12, 6, 0))
}

fn card_title(text: &'static str) -> impl Render {
  battlement_reactant::host::Label::new(ls(text)).style(
    Style::new()
      .height(28.0)
      .color(design_system::MUTED_TEXT)
      .font_size(16.0)
      .white_space(WhiteSpace::NoWrap),
  )
}

fn label_grid() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn row() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .align_items(Align::Center)
}

fn image(width: f32, height: f32) -> Style {
  Style::new().width(width).height(height).margin((4, 8))
}

fn resize_action(compact: bool, resized: bool, state: design_system::ControlState) -> Style {
  let width = if compact {
    if resized { 360.0 } else { 300.0 }
  } else if resized {
    610.0
  } else {
    420.0
  };
  design_system::primary_action(state)
    .width(width)
    .height(70.0)
    .align_self(Align::FlexStart)
    .background_color(battlement::Color::rgba(0.0, 0.0, 0.0, 0.0))
    .color(design_system::PRIMARY_TEXT)
    .border_width(0.0)
    .font_size(20.0)
    .unity_text_align(TextAnchor::MiddleCenter)
    .margin((4, 8, 8, 8))
}
