use battlement::{ParentScene, PickingMode, Prop, UiFontAddress};
use reactant::{Application, app_context, hooks, prelude::*, world};
use trox::{SourceLocale, ls};

use crate::{
  assets,
  card_table::CardTable,
  domain::{Checkpoint, HeartsState, IgnorePresentation, Intention, Phase, Seat, transition},
  projection::{HumanView, Projection},
  scene,
};

struct LayoutFixture;

pub(crate) fn application() -> Application {
  Application::new(assets::hearts::CONTENT)
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(LayoutFixture)
    .document(|mut document| {
      document.root_id = crate::app::ROOT;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| scene::camera().into_object(camera.object_id))
}

impl Component for LayoutFixture {
  fn render(&self) -> impl Render {
    let views = hooks::use_memo(self::views, ());
    let (index, set_index) = hooks::use_state(0_usize);
    let (inspecting, inspect) = hooks::use_state(false);
    let viewport = app_context::use_viewport_size();
    let aspect = f64::from(viewport.width) / f64::from(viewport.height);
    let last = views.len() - 1;
    let view = &views[index];
    (
      View::new()
        .picking_mode(PickingMode::Ignore)
        .style(
          Style::new()
            .padding(18.px())
            .flex_direction(FlexDirection::Row)
            .align_items(Align::Center)
            .color(Color::rgb(0.06, 0.12, 0.03))
            .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
        )
        .child((
          Heading::new(
            ls(format!("Card layout {} / {}", index + 1, views.len())),
            1,
          ),
          Button::new(ls("Next layout checkpoint"))
            .on_press(set_index.update_callback(move |index| (index + 1).min(last))),
          Button::new(ls("Inspect visible card")).on_press(inspect.update_callback(|value| !value)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        scene::environment(aspect),
        CardTable::new(view, aspect)
          .inspect(inspecting.then(|| view.hands[Seat::South.index()][0])),
      )),
    )
  }
}

pub(crate) fn views() -> Vec<HumanView> {
  let projection = Projection::default();
  let mut state = HeartsState::new(43);
  let mut states = vec![state.clone()];
  for seat in Seat::ALL {
    let cards = state.hand(seat).iter().copied().take(3).collect();
    transition::apply(
      &mut state,
      Intention::SubmitPass { seat, cards },
      &mut IgnorePresentation,
    )
    .expect("fixture pass");
  }
  states.push(state.clone());
  let mut checkpoints: Vec<Checkpoint> = Vec::new();
  for _ in 0..4 {
    let Phase::Playing { turn } = state.phase() else {
      panic!("fixture playing phase")
    };
    let card = state.observe(turn).legal_plays[0];
    transition::apply(
      &mut state,
      Intention::PlayCard { seat: turn, card },
      &mut checkpoints,
    )
    .expect("fixture legal play");
  }
  states.extend(checkpoints.into_iter().map(|checkpoint| checkpoint.state));
  states
    .iter()
    .map(|state| projection.view(state, Seat::South))
    .collect()
}
