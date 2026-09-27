use battlement::{ParentScene, PickingMode, Prop, UiFontAddress};
use reactant::{
  Application, GameRoot, app_context, hooks,
  motion_config::{MotionConfig, ReducedMotion},
  prelude::*,
  world,
};
use trox::{SourceLocale, ls};

use crate::{
  assets,
  choreography::AnimatedTable,
  controller,
  domain::{HeartsState, IgnorePresentation, Intention, Phase, Seat, transition},
  layout_fixture,
  projection::PresentationSeed,
  reducer::HeartsAction,
  scene,
};

struct MotionFixture;
struct PauseControl(bool);

pub(crate) fn application() -> Application {
  Application::new(assets::hearts::CONTENT)
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(
      ContextProvider::new()
        .context(PresentationSeed(43))
        .child(MotionFixture),
    )
    .document(|mut document| {
      document.root_id = crate::app::ROOT;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| scene::camera().into_object(camera.object_id))
}

impl Component for MotionFixture {
  fn render(&self) -> impl Render {
    let (generation, reset) = hooks::use_state(0_u64);
    let (ending, set_ending) = hooks::use_state(false);
    let (paused, set_paused) = hooks::use_state(false);
    let (reduced, set_reduced) = hooks::use_state(false);
    let (rotated, set_rotated) = hooks::use_state(false);
    let game = controller::use_hearts(
      (generation, ending),
      move || self::initial(ending),
      Seat::South,
      true,
    );
    let viewport = app_context::use_viewport_size();
    let viewport_aspect = f64::from(viewport.width) / f64::from(viewport.height);
    let aspect = if rotated {
      if viewport_aspect > 1.0 {
        1.0 / viewport_aspect
      } else {
        viewport_aspect * 0.8
      }
    } else {
      viewport_aspect
    };
    let dispatch = game.game.clone();
    let version = dispatch.accepted().version;
    let next = self::next(&dispatch.accepted().state);
    let phase = match game.view.table.phase {
      Phase::Passing => "Passing",
      Phase::Playing { .. } => "Playing",
      Phase::HandOver => "Scored",
      Phase::MatchOver { .. } => "Match over",
    };
    (
      GameRoot::new(PauseControl(paused)),
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(18.px())
            .top(18.px())
            .width(320.px())
            .font_size(18.px())
            .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
        )
        .child((
          GameRoot::new(
            Heading::new(
              ls(format!("{phase}: {} cards in trick", game.view.trick.len())),
              2,
            )
            .host_name("motion-phase"),
          ),
          Label::new(ls(format!(
            "Presentation {:?}",
            game.game.presentation().status
          ))),
          Button::new(ls("Advance Hearts action")).on_press(move || {
            dispatch.dispatch(version, HeartsAction::Human(next.clone()));
          }),
          Button::new(ls("Replay deal")).on_press(reset.update_callback(|value| value + 1)),
          Button::new(ls("Load final trick")).on_press(set_ending.update_callback(|value| !value)),
          Button::new(ls("Toggle motion pause"))
            .on_press(set_paused.update_callback(|value| !value)),
          Button::new(ls("Retarget table layout"))
            .on_press(set_rotated.update_callback(|value| !value)),
          Button::new(ls("Toggle reduced motion"))
            .on_press(set_reduced.update_callback(|value| !value)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        scene::environment(viewport_aspect),
        GameRoot::new(
          MotionConfig::new(AnimatedTable {
            view: game.view,
            aspect,
            inspection: None,
            fresh: !ending,
            sound: true,
          })
          .reduced_motion(if reduced {
            ReducedMotion::Always
          } else {
            ReducedMotion::Never
          }),
        ),
      )),
    )
  }
}

fn next(state: &HeartsState) -> Intention {
  match state.phase() {
    Phase::Passing => Intention::SubmitPass {
      seat: Seat::South,
      cards: state.hand(Seat::South).iter().take(3).copied().collect(),
    },
    Phase::Playing { turn } => Intention::PlayCard {
      seat: turn,
      card: state.observe(turn).legal_plays[0],
    },
    Phase::HandOver | Phase::MatchOver { .. } => Intention::NextHand,
  }
}

fn initial(ending: bool) -> HeartsState {
  if ending {
    let mut state = layout_fixture::playing_state();
    while state.history.len() < 48 {
      let next = self::next(&state);
      transition::apply(&mut state, next, &mut IgnorePresentation).expect("fixture play");
    }
    return state;
  }
  let mut state = HeartsState::new(43);
  for seat in [Seat::West, Seat::North, Seat::East] {
    let cards = state.hand(seat).iter().take(3).copied().collect();
    transition::apply(
      &mut state,
      Intention::SubmitPass { seat, cards },
      &mut IgnorePresentation,
    )
    .expect("fixture pass");
  }
  state
}

impl Component for PauseControl {
  fn render(&self) -> impl Render {
    let presentation = reactant::use_game_presentation();
    let paused = self.0;
    hooks::use_effect(
      move || {
        if paused {
          presentation.pause();
        } else {
          presentation.resume();
        }
      },
      paused,
    );
  }
}
