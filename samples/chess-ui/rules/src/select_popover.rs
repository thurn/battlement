//! Animated portal content for [`crate::select_control::SelectControl`].

use trox::tx;

use crate::{
  dropdown_motion,
  font_scale::{FontScale, FontScaleRole},
  select_control, select_navigation,
  select_option::SelectOption,
};
use battlement::{Color, Gradient, Length, PopoverPlacement, Scale, Style, TransformOrigin};
use battlement_reactant::{
  element_ref::ElementRef,
  hooks,
  overlay::Overlay,
  paint::{PaintLayer, PaintStyle},
  portal::PortalTarget,
  prelude::*,
  prelude::{PaintDropShadow, PaintFilterList},
  semantics::SemanticVisibility,
};

/// Owns the retained keyed value required by `AnimatePresence`.
#[builder]
pub(crate) struct SelectPopover {
  #[builder(required)]
  active_index: usize,
  #[builder(required)]
  anchor: ElementRef,
  #[builder(required)]
  font_scale: FontScale,
  #[builder(required)]
  on_change: EventCallback<String>,
  #[builder(required)]
  options: Vec<String>,
  open_generation: u32,
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  popover_scale: f32,
  reduced_motion: bool,
  #[builder(required)]
  set_active_index: hooks::StateSetter<usize>,
  #[builder(required)]
  set_open: hooks::StateSetter<bool>,
  #[builder(required)]
  set_restore_focus: hooks::StateSetter<bool>,
  #[builder(required)]
  value: String,
}

impl Component for SelectPopover {
  fn render(&self) -> impl Render {
    let is_present = use_is_present();
    Overlay::popover(self.overlay.clone(), self.anchor.clone())
      .host_name("select-popover")
      .placement(PopoverPlacement::bottom_start().offset(6.0))
      .style(
        Style::new()
          .width(self::width(self.font_scale) * self.popover_scale)
          .height(self::height(self.font_scale, self.options.len()) * self.popover_scale),
      )
      .child(
        View::new()
          .name("select-popover-motion")
          .style(
            Style::new()
              .width(100.pct())
              .height(100.pct())
              .transform_origin(TransformOrigin::two_dimensional(
                Length::Px(0.0),
                Length::Px(0.0),
              )),
          )
          .initial(dropdown_motion::menu_initial(self.reduced_motion))
          .animate(dropdown_motion::menu_visible())
          .exit(dropdown_motion::menu_exit(self.reduced_motion))
          .transition(dropdown_motion::menu_transition(self.reduced_motion))
          .child(
            ListBox::new(tx(
              "Display Mode options",
              "Display mode options interface label.",
            ))
            .host_name("select-listbox")
            .semantic_visibility(if is_present {
              SemanticVisibility::Exposed
            } else {
              SemanticVisibility::Hidden
            })
            .style(
              Style::new()
                .width(self::width(self.font_scale))
                .height(self::height(self.font_scale, self.options.len()))
                .padding_top(11)
                .padding_bottom(11)
                .padding_left(9)
                .padding_right(9)
                .scale(Scale::uniform(self.popover_scale))
                .transform_origin(TransformOrigin::two_dimensional(
                  Length::Px(0.0),
                  Length::Px(0.0),
                )),
            )
            .configure_host({
              let options = self.options.clone();
              let set_active_index = self.set_active_index.clone();
              let set_open = self.set_open.clone();
              let set_restore_focus = self.set_restore_focus.clone();
              let active_index = self.active_index;
              move |host| {
                host
                  .paint(self::popover_paint())
                  .on_key_down_event({
                    let options = options.clone();
                    let set_active_index = set_active_index.clone();
                    let set_open = set_open.clone();
                    let set_restore_focus = set_restore_focus.clone();
                    move |event| {
                      select_navigation::list_key(
                        event,
                        active_index,
                        &options,
                        set_active_index.clone(),
                        set_open.clone(),
                        set_restore_focus.clone(),
                      );
                    }
                  })
                  .on_navigation_move_event({
                    let set_active_index = set_active_index.clone();
                    move |event| {
                      select_navigation::list_navigation(
                        event,
                        active_index,
                        options.len(),
                        set_active_index.clone(),
                      );
                    }
                  })
                  .on_navigation_cancel(move || {
                    select_navigation::dismiss(set_open.clone(), set_restore_focus.clone());
                  })
              }
            })
            .child(
              self
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| self.option(index, option))
                .collect::<Vec<_>>(),
            ),
          ),
      )
  }
}

impl SelectPopover {
  fn option(&self, index: usize, option: &str) -> SelectOption {
    SelectOption::new()
      .active(index == self.active_index)
      .control_scale(self.font_scale.dynamic(FontScaleRole::Control))
      .font_scale(self.font_scale.factor())
      .index(index)
      .label(option)
      .focus_generation(self.open_generation)
      .selected(option == self.value)
      .on_press(
        self
          .on_change
          .clone()
          .map_input({
            let option = String::from(option);
            move |()| option.clone()
          })
          .then(self.set_active_index.callback().map_input(move |()| index))
          .then(EventCallback::new({
            let set_open = self.set_open.clone();
            let set_restore_focus = self.set_restore_focus.clone();
            move |()| {
              select_navigation::dismiss(set_open.clone(), set_restore_focus.clone());
            }
          })),
      )
  }
}

fn width(font_scale: FontScale) -> f32 {
  396.0 + (font_scale.factor() - 1.0) * 300.0
}

fn height(font_scale: FontScale, options: usize) -> f32 {
  22.0 + options as f32 * 76.0 * font_scale.dynamic(FontScaleRole::Control)
}

fn popover_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(145.0)
        .stop(0.0, Color::hex(0x5df5ff))
        .stop(0.48, Color::hex(0x718cff))
        .stop(1.0, Color::hex(0xff4bc9)),
    )
    .paint_filter(
      PaintFilterList::default()
        .drop_shadow(PaintDropShadow::new(
          0.0,
          10.0,
          14.0,
          0.0,
          Color::BLACK.with_alpha(0.72),
        ))
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          8.0,
          0.0,
          Color::hex(0x2b7eff).with_alpha(0.65),
        )),
    )
    .clip_polygon(select_control::clip(10.0))
    .layer(
      PaintLayer::new(
        Gradient::linear(180.0)
          .stop(0.0, Color::hex(0x07152e))
          .stop(1.0, Color::hex(0x020611)),
      )
      .bounds_inset(3.0)
      .clip_polygon(select_control::clip(7.0)),
    )
}
