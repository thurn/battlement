use battlement::{ParentScene, PickingMode, Prop, UiFontAddress};
use reactant::{
  Application, GameRoot, animation_controls, app_context, hooks,
  motion_config::{MotionConfig, ReducedMotion},
  native_host,
  prelude::*,
  world,
};
use trox::{SourceLocale, ls};

use crate::{
  assets,
  card_table::CardTable,
  controller,
  domain::{HeartsState, Seat},
  particles, scene,
};

struct ParticleFixture;
struct Probe {
  aspect: f64,
}

pub(crate) fn application() -> Application {
  Application::new(assets::hearts::CONTENT)
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(ParticleFixture)
    .document(|mut document| {
      document.root_id = crate::app::ROOT;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| scene::camera().into_object(camera.object_id))
}

impl Component for ParticleFixture {
  fn render(&self) -> impl Render {
    let (generation, reset) = hooks::use_state(0_u64);
    let game = controller::use_hearts(generation, || HeartsState::new(43), Seat::South, true);
    let viewport = app_context::use_viewport_size();
    let aspect = f64::from(viewport.width) / f64::from(viewport.height);
    (
      Button::new(ls("Replace particle session"))
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(18.px())
            .top(18.px())
            .height(40.px()),
        )
        .on_press(reset.update_callback(|value| value + 1)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        scene::environment(aspect),
        CardTable::new(&game.view, aspect),
      )),
      GameRoot::new(Probe { aspect }),
    )
  }
}

impl Component for Probe {
  fn render(&self) -> impl Render {
    let (enabled, set_enabled) = hooks::use_state(true);
    let (reduced, set_reduced) = hooks::use_state(false);
    let (seed, set_seed) = hooks::use_state(43_u32);
    let (paused, set_paused) = hooks::use_state(false);
    let (trigger, fire) = hooks::use_state(0_u64);
    let presentation = reactant::use_game_presentation();
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
    (
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        MotionConfig::new(Effects {
          aspect: self.aspect,
          seed,
          enabled,
          trigger,
        })
        .reduced_motion(if reduced {
          ReducedMotion::Always
        } else {
          ReducedMotion::Never
        }),
      ),
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(18.px())
            .top(70.px())
            .width(210.px())
            .font_size(18.px())
            .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
        )
        .child((
          Button::new(ls(if enabled {
            "Remove ambience"
          } else {
            "Mount ambience"
          }))
          .on_press(set_enabled.update_callback(|value| !value)),
          Button::new(ls("Toggle particle pause"))
            .on_press(set_paused.update_callback(|value| !value)),
          Button::new(ls(if reduced {
            "Full motion"
          } else {
            "Reduced motion"
          }))
          .on_press(set_reduced.update_callback(|value| !value)),
          Button::new(ls("Change particle seed"))
            .on_press(set_seed.update_callback(|value| if value == 43 { 97 } else { 43 })),
          Button::new(ls("Particle accent")).on_press(fire.update_callback(|value| value + 1)),
        )),
    )
  }
}

struct Effects {
  aspect: f64,
  seed: u32,
  enabled: bool,
  trigger: u64,
}

impl Component for Effects {
  fn render(&self) -> impl Render {
    let anchor = native_host::use_object_ref();
    let scope = animation_controls::use_animation_scope();
    let reduced = reactant::motion_config::use_reduced_motion();
    let trigger = self.trigger;
    hooks::use_effect(
      {
        let anchor = anchor.clone();
        let scope = scope.clone();
        move || {
          if trigger > 0 && !reduced {
            scope.start(particles::burst(anchor, 71));
          }
        }
      },
      trigger,
    );
    (
      self
        .enabled
        .then(|| particles::ambient(self.aspect, self.seed)),
      particles::anchor(self.aspect)
        .reference(anchor)
        .motion(MotionProps::new().animation_scope(scope)),
    )
  }
}
