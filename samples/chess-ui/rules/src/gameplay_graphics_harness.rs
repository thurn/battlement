//! Resettable Gameplay and Graphics settings specimens.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Align, Color, FlexDirection,
  ScrollerVisibility, Style, TextAnchor, Vector,
};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{ls, tx};

use crate::{
  erase_control::EraseControl,
  font_scale::{self, FontScale},
  graphics_settings::GraphicsSettings,
  select_control::SelectControl,
  settings_panel::SettingsPanel,
  toggle_control::ToggleControl,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Gameplay,
  Graphics,
}

/// Owns all state-only values required by the two source settings panels.
#[builder]
pub struct GameplayGraphicsHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for GameplayGraphicsHarness {
  fn render(&self) -> impl Render {
    let (specimen, set_specimen) = hooks::use_state(Specimen::Gameplay);
    let (font_scale, set_font_scale) = hooks::use_state(FontScale::Percent100);
    let (language, set_language) = hooks::use_state(String::from("English"));
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    let (increase_move_duration, set_increase_move_duration) = hooks::use_state(true);
    let (upload_crash_reports, set_upload_crash_reports) = hooks::use_state(true);
    let (resolution, set_resolution) = hooks::use_state(String::from("1920 × 1080"));
    let (max_framerate, set_max_framerate) = hooks::use_state(String::from("144 FPS"));
    let (display_mode, set_display_mode) = hooks::use_state(String::from("Borderless"));
    let (screenshake, set_screenshake) = hooks::use_state(true);
    let (vsync, set_vsync) = hooks::use_state(true);
    let (erase_requests, set_erase_requests) = hooks::use_state(0_u32);
    let (help_requests, set_help_requests) = hooks::use_state(0_u32);
    let (scrolled, set_scrolled) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);

    View::new()
      .name("gameplay-graphics-harness")
      .style(Style::new().width(887).margin_top(8))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(10.0)
          .style(Style::new().min_height(64).align_items(Align::Center))
          .child((
            self::harness_button(
              "GAMEPLAY",
              "settings-specimen-gameplay",
              specimen == Specimen::Gameplay,
              set_specimen
                .callback()
                .map_input(|_| Specimen::Gameplay)
                .then(set_scrolled.callback().map_input(|_| false)),
            ),
            self::harness_button(
              "GRAPHICS",
              "settings-specimen-graphics",
              specimen == Specimen::Graphics,
              set_specimen
                .callback()
                .map_input(|_| Specimen::Graphics)
                .then(set_scrolled.callback().map_input(|_| false)),
            ),
            self::harness_button(
              "RESET",
              "gameplay-graphics-reset",
              false,
              set_specimen
                .callback()
                .map_input(|_| Specimen::Gameplay)
                .then(
                  set_font_scale
                    .callback()
                    .map_input(|_| FontScale::Percent100),
                )
                .then(
                  set_language
                    .callback()
                    .map_input(|_| String::from("English")),
                )
                .then(set_reduce_motion.callback().map_input(|_| false))
                .then(set_increase_move_duration.callback().map_input(|_| true))
                .then(set_upload_crash_reports.callback().map_input(|_| true))
                .then(
                  set_resolution
                    .callback()
                    .map_input(|_| String::from("1920 × 1080")),
                )
                .then(
                  set_max_framerate
                    .callback()
                    .map_input(|_| String::from("144 FPS")),
                )
                .then(
                  set_display_mode
                    .callback()
                    .map_input(|_| String::from("Borderless")),
                )
                .then(set_screenshake.callback().map_input(|_| true))
                .then(set_vsync.callback().map_input(|_| true))
                .then(set_erase_requests.callback().map_input(|_| 0))
                .then(set_help_requests.callback().map_input(|_| 0))
                .then(set_scrolled.callback().map_input(|_| false))
                .then(set_reset_generation.update_callback(|value| value.wrapping_add(1))),
            ),
            control_behavior::static_label(ls(format!(
              "{} · Text {} · Help {help_requests} · Erase {erase_requests}",
              match specimen {
                Specimen::Gameplay => "Gameplay",
                Specimen::Graphics => "Graphics",
              },
              font_scale.label(),
            )))
            .name("gameplay-graphics-status")
            .style(Style::new().width(285).height(58).font_size(20)),
          )),
        font_scale::provider(
          font_scale,
          SettingsPanel::new().children(
            ScrollArea::new(
              Some(ls(match specimen {
                Specimen::Gameplay => "Gameplay settings controls",
                Specimen::Graphics => "Graphics settings controls",
              })),
              AccessibilityScrollAxis::Vertical,
              font_scale.factor() > 1.0 && !scrolled,
              scrolled,
            )
            .on_scroll({
              let set_scrolled = set_scrolled.clone();
              move |direction| set_scrolled.set(direction == AccessibilityScrollDirection::Forward)
            })
            .host_name("gameplay-graphics-scroll")
            .configure_host(|host| {
              host
                .scroll_offset(Vector::new(
                  0.0,
                  if scrolled {
                    self::content_height(specimen, font_scale)
                  } else {
                    0.0
                  },
                ))
                .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
                .vertical_scroller_visibility(if font_scale.factor() > 1.0 {
                  ScrollerVisibility::Auto
                } else {
                  ScrollerVisibility::Hidden
                })
            })
            .style(Style::new().width(839).height(971))
            .child(
              View::new()
                .name(match specimen {
                  Specimen::Gameplay => "gameplay-settings",
                  Specimen::Graphics => "graphics-settings-content",
                })
                .key((reset_generation, specimen as u8))
                .child((
                  (specimen == Specimen::Gameplay).then(|| {
                    (
                      SelectControl::new()
                        .label(control_behavior::name_source_text(tx(
                          "Language",
                          "Gameplay language setting label.",
                        )))
                        .value(language.clone())
                        .options(
                          ["English", "Español", "Français", "Deutsch"]
                            .map(String::from)
                            .to_vec(),
                        )
                        .overlay(self.overlay.clone())
                        .on_change(set_language.clone())
                        .first(true),
                      SelectControl::new()
                        .label(control_behavior::name_source_text(tx(
                          "Text Size",
                          "Gameplay text-size setting label.",
                        )))
                        .value(font_scale.label().to_owned())
                        .options(
                          FontScale::ALL
                            .map(|value| value.label().to_owned())
                            .to_vec(),
                        )
                        .overlay(self.overlay.clone())
                        .on_change(
                          set_font_scale
                            .callback()
                            .map_input(|label: String| match label.as_str() {
                              "150%" => FontScale::Percent150,
                              "200%" => FontScale::Percent200,
                              _ => FontScale::Percent100,
                            })
                            .then(set_scrolled.callback().map_input(|_| false)),
                        ),
                      ToggleControl::new()
                        .label(control_behavior::name_source_text(tx(
                          "Reduce Motion",
                          "Gameplay reduced-motion setting label.",
                        )))
                        .checked(reduce_motion)
                        .on_change(set_reduce_motion.clone()),
                      ToggleControl::new()
                        .label(
                          control_behavior::name_source_text(tx(
                            "Increase Move\nDuration",
                            "Two-line gameplay duration setting label.",
                          ))
                          .style(Style::new().height(112.24 * font_scale.factor())),
                        )
                        .aria_label(tx(
                          "Increase Move Duration",
                          "Gameplay duration checkbox accessibility label.",
                        ))
                        .row_height(self::multiline_row_height(font_scale))
                        .checked(increase_move_duration)
                        .on_change(set_increase_move_duration.clone()),
                      ToggleControl::new()
                        .label(
                          control_behavior::name_source_text(tx(
                            "Upload Crash\nReports",
                            "Two-line crash-report setting label.",
                          ))
                          .style(Style::new().height(112.24 * font_scale.factor())),
                        )
                        .aria_label(tx(
                          "Upload Crash Reports",
                          "Crash-report checkbox accessibility label.",
                        ))
                        .row_height(self::multiline_row_height(font_scale))
                        .checked(upload_crash_reports)
                        .with_info(true)
                        .on_info_click(set_help_requests.update_callback(|value| value + 1))
                        .on_change(set_upload_crash_reports.clone()),
                      EraseControl::new()
                        .on_click(set_erase_requests.update_callback(|value| value + 1)),
                    )
                  }),
                  (specimen == Specimen::Graphics).then(|| {
                    GraphicsSettings::new()
                      .resolution(resolution.clone())
                      .max_framerate(max_framerate.clone())
                      .display_mode(display_mode.clone())
                      .screenshake(screenshake)
                      .vsync(vsync)
                      .overlay(self.overlay.clone())
                      .on_resolution_change(set_resolution.clone())
                      .on_max_framerate_change(set_max_framerate.clone())
                      .on_display_mode_change(set_display_mode.clone())
                      .on_screenshake_change(set_screenshake.clone())
                      .on_vsync_change(set_vsync.clone())
                  }),
                )),
            ),
          ),
        ),
      ))
  }
}

fn harness_button(
  label: &'static str,
  name: &'static str,
  selected: bool,
  on_press: EventCallback<()>,
) -> impl Render {
  Button::new(ls(label))
    .host_name(name)
    .on_press(on_press)
    .style(
      Style::new()
        .width(160)
        .height(54)
        .padding((8, 12))
        .border_width(1)
        .border_color(Color::hex(if selected { 0x61f0e6 } else { 0x31455d }))
        .border_radius(6)
        .background_color(Color::hex(if selected { 0x163d43 } else { 0x101a28 }))
        .color(Color::hex(if selected { 0x7ffcf2 } else { 0xd4e4f1 }))
        .font_size(24)
        .unity_text_align(TextAnchor::MiddleCenter),
    )
}

fn multiline_row_height(scale: FontScale) -> f32 {
  match scale {
    FontScale::Percent100 => 159.0,
    FontScale::Percent150 => 227.0,
    FontScale::Percent200 => 211.0,
  }
}

fn content_height(specimen: Specimen, scale: FontScale) -> f32 {
  match specimen {
    Specimen::Gameplay => (4.0 * 159.0 + 2.0 * self::multiline_row_height(scale)) * scale.factor(),
    Specimen::Graphics => 5.0 * 159.0 * scale.factor(),
  }
}
