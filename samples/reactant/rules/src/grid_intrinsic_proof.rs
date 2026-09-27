use battlement::{Color, FlexDirection, GridTrack, WhiteSpace};
use reactant::{control_behavior, hooks, prelude::*};
use trox::ls;

struct GridIntrinsic;

pub(crate) fn app() -> crate::ReactantApplication {
  reactant::Application::new(crate::CONTENT_SCENE)
    .child(GridIntrinsic)
    .document(|mut document| {
      document.root_id = crate::ROOT_ID;
      document
    })
}

impl Component for GridIntrinsic {
  fn render(&self) -> impl Render {
    let (french, set_french) = hooks::use_state(false);
    let (scale, set_scale) = hooks::use_state(100_u32);
    let (width, set_width) = hooks::use_state(420_u32);
    let (revision, set_revision) = hooks::use_state(0_u32);
    let grid_ref = use_element_ref();
    let row_ref = use_element_ref();
    let second_ref = use_element_ref();
    let measured =
      use_geometry((grid_ref.clone(), row_ref.clone(), second_ref.clone())).measurements;
    let dimensions = format!(
      "Measured grid {:.0}, row {:.0}",
      measured.0.latest.map_or(0.0, |value| value.layout.width),
      measured.1.latest.map_or(0.0, |value| value.layout.width)
    );
    let separated = match (measured.0.latest, measured.1.latest, measured.2.latest) {
      (Some(grid), Some(first), Some(second)) => {
        match (first.bounds_in(&grid), second.bounds_in(&grid)) {
          (Some(first), Some(second)) => second.y >= first.y + first.height + 11.0,
          _ => false,
        }
      }
      _ => false,
    };
    let separation = if separated {
      "Rows separated"
    } else {
      "Rows overlap or unmeasured"
    };
    View::new()
      .style(
        Style::new()
          .width(960.px())
          .padding(24.px())
          .color(Color::WHITE)
          .background_color(Color::rgb(0.04, 0.09, 0.15)),
      )
      .child((
        Heading::new(ls("Intrinsic grid rows"), 1),
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(8.0)
          .child((
            Button::new(ls("English")).on_press(set_french.update_callback(|_| false)),
            Button::new(ls("French")).on_press(set_french.update_callback(|_| true)),
            [100, 150, 200].into_iter().map(|value| {
              Button::new(ls(format!("{value}%")))
                .on_press(set_scale.update_callback(move |_| value))
            }).collect::<Vec<_>>(),
            Button::new(ls("Narrow")).on_press(set_width.update_callback(|_| 200)),
            Button::new(ls("Wide")).on_press(set_width.update_callback(|_| 420)),
          )),
        Label::new(ls(format!("{} · {scale}% · {width}",
          if french { "French" } else { "English" })))
          .semantic(control_behavior::static_text_props(ls(format!(
            "{} · {scale}% · {width}", if french { "French" } else { "English" }
          )))),
        Label::new(ls(dimensions.clone()))
          .semantic(control_behavior::static_text_props(ls(dimensions))),
        Label::new(ls(separation))
          .semantic(control_behavior::static_text_props(ls(separation))),
        Grid::new()
          .element_ref(grid_ref)
          .columns([GridTrack::fr(1.0)])
          .row_gap(12.0)
          .style(Style::new().width((width as f32).px()))
          .child((0..2).map(|index| {
            Flex::new()
              .element_ref(if index == 0 { row_ref.clone() } else { second_ref.clone() })
              .direction(FlexDirection::Column)
              .gap(8.0)
              .style(Style::new().min_width(0.px()).padding(8.px())
                .background_color(Color::rgb(0.08, 0.18, 0.23)))
              .child((
                Flex::new().child(
                  Label::new(ls(if french {
                    "Afficher les coordonnées de l’échiquier pendant la révision des paramètres enregistrés"
                  } else {
                    "Keep the board coordinates visible while reviewing your saved game settings"
                  }))
                  .style(Style::new().font_size(16.0 * scale as f32 / 100.0)
                    .white_space(WhiteSpace::Normal)),
                ),
                Button::new(ls(format!("Setting {} · {revision}", index + 1)))
                  .on_press(set_revision.update_callback(|value| value + 1))
                  .style(Style::new().min_height(44.px())),
              ))
          }).collect::<Vec<_>>()),
      ))
  }
}
