use std::sync::Arc;

use battlement::{
  ControllerButton, ControllerInputSettings, ObjectId, ParentScene, PhysicalKey, PickingMode, Prop,
  UiFontAddress, Vector3, object_id,
};
use reactant::{
  Application, FilePersistenceBackend, PersistenceBackend, app_context, hooks,
  motion_config::{MotionConfig, ReducedMotion},
  overlay::OverlayHost,
  prelude::*,
  world,
};
use trox::{SourceLocale, ls};

use crate::{
  assets, audio, card_assets,
  card_controls::CardControls,
  card_input,
  choreography::AnimatedTable,
  controller,
  domain::{HeartsState, Phase, Seat, cards},
  match_ui,
  menus::{Menu, Menus},
  particles,
  saved_game::SavedSettings,
  scene,
  session_save::{self, SaveStatus},
  startup::{Saves, Startup},
};

pub(crate) const ROOT: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47000");

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Opponents {
  Disabled,
  Scripted,
  Rollout,
}

pub(crate) struct HeartsRoot {
  initial: HeartsState,
  gallery: bool,
  opponents: Opponents,
  fresh: bool,
  teaching: bool,
  new_match: fn() -> HeartsState,
}

/// Loads saved progress and settings before offering Continue or a new match.
pub fn application() -> Application {
  self::application_with_storage(Arc::new(FilePersistenceBackend))
}

/// Uses the same persistent application with an injected storage boundary.
pub fn application_with_storage(backend: Arc<dyn PersistenceBackend>) -> Application {
  self::configured_content(
    Startup {
      backend,
      opponents: Opponents::Rollout,
      new_match: self::new_match,
    },
    false,
  )
}

/// Mounts an already validated logical match without replaying its history.
pub fn application_from_state(initial: HeartsState) -> Application {
  self::configured(initial, false, Opponents::Rollout, false)
}

pub(crate) fn exported_application() -> Application {
  match std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() {
    Ok("persistence") => crate::persistence_fixture::application(),
    Ok("motion") => crate::motion_fixture::application(),
    Ok("particles") => crate::particle_fixture::application(),
    Ok("layout") => crate::layout_fixture::application(),
    Ok("cards") => self::configured(HeartsState::new(43), true, Opponents::Disabled, false),
    Ok("restored") => self::configured(HeartsState::new(73), false, Opponents::Disabled, false),
    Ok("shell") => self::configured(HeartsState::new(43), false, Opponents::Disabled, false),
    Ok("input") => self::configured(HeartsState::new(43), false, Opponents::Scripted, true),
    Ok("play") => self::configured(
      crate::layout_fixture::playing_state(),
      false,
      Opponents::Scripted,
      false,
    ),
    Ok(
      name @ ("teaching" | "screens" | "hand-results" | "moon-results" | "match-results"
      | "tied-results" | "hold"),
    ) => crate::screens_fixture::application(name),
    Ok("ai-lifecycle") => crate::ai_fixture::lifecycle_application(),
    Ok("ai") => crate::ai_fixture::application(),
    Err(_) => self::application(),
    Ok(name) => panic!("unknown Hearts fixture {name:?}"),
  }
}

pub(crate) fn screen_application(initial: HeartsState, teaching: bool) -> Application {
  self::configured_root(HeartsRoot {
    initial,
    gallery: false,
    opponents: Opponents::Scripted,
    fresh: false,
    teaching,
    new_match: self::fixture_match,
  })
}

fn configured(
  initial: HeartsState,
  gallery: bool,
  opponents: Opponents,
  fresh: bool,
) -> Application {
  self::configured_root(HeartsRoot {
    initial,
    gallery,
    opponents,
    fresh,
    teaching: fresh && opponents == Opponents::Rollout,
    new_match: self::fixture_match,
  })
}

fn configured_root(root: HeartsRoot) -> Application {
  let gallery = root.gallery;
  self::configured_content(root, gallery)
}

pub(crate) fn persistent_root(
  initial: HeartsState,
  fresh: bool,
  opponents: Opponents,
  new_match: fn() -> HeartsState,
) -> HeartsRoot {
  HeartsRoot {
    initial,
    gallery: false,
    opponents,
    fresh,
    teaching: fresh,
    new_match,
  }
}

fn new_match() -> HeartsState {
  HeartsState::new(fastrand::u64(..))
}

pub(crate) fn fixture_match() -> HeartsState {
  HeartsState::new(43)
}

pub(crate) fn configured_content(root: impl Component, gallery: bool) -> Application {
  Application::new(assets::hearts::CONTENT)
    .global_keys([
      PhysicalKey::ArrowLeft,
      PhysicalKey::ArrowRight,
      PhysicalKey::ArrowUp,
      PhysicalKey::ArrowDown,
      PhysicalKey::Enter,
      PhysicalKey::Escape,
      PhysicalKey::Tab,
    ])
    .controller_input(
      ControllerInputSettings::new().buttons([ControllerButton::South, ControllerButton::East]),
    )
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(root)
    .document(|mut document| {
      document.root_id = ROOT;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
        .name("hearts")
        .style(Style::new().color(Color::rgb(0.97, 0.94, 0.83)))
    })
    .camera(move |camera| {
      if !gallery {
        return scene::camera().into_object(camera.object_id);
      }
      world::Camera::new()
        .orthographic(4.8)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.12, 0.23, 0.15))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for HeartsRoot {
  fn render(&self) -> impl Render {
    let interactive = self.opponents != Opponents::Disabled;
    let overlay = reactant::use_portal_target();
    let saves = hooks::use_optional_context::<Saves>();
    let saved_settings = saves
      .as_ref()
      .and_then(|saves| saves.settings.value())
      .copied()
      .unwrap_or_default();
    let (mix, set_mix) = hooks::use_state(saved_settings.mix());
    let (preferences, set_preferences) = hooks::use_state(saved_settings.preferences);
    let (exiting, set_exiting) = hooks::use_state(false);
    let inactive = !reactant::application::use_application_state().is_active();
    let teaching = self.teaching;
    let (menu, set_menu) = hooks::use_state(teaching.then_some(Menu::PassingHelp));
    let (taught_play, set_taught_play) = hooks::use_state(false);
    let (generation, reset) = hooks::use_state(0_u64);
    let first = self.initial.clone();
    let new_match = self.new_match;
    let initial = hooks::use_memo(
      move || {
        if generation == 0 { first } else { new_match() }
      },
      generation,
    );
    let policy = if self.opponents == Opponents::Scripted {
      crate::layout_fixture::scripted_decision
    } else {
      crate::ai::search::decide
    };
    let suspended = inactive || exiting;
    let game = controller::use_hearts_with_policy(
      generation,
      move || initial,
      Seat::South,
      !interactive || menu.is_some() || suspended,
      policy,
    );
    session_save::use_autosave(&game, SavedSettings::capture(preferences, mix), exiting);
    let phase = game.view.table.phase;
    let show_help = set_menu.clone();
    hooks::use_effect(
      move || {
        let needs_help = teaching && !taught_play;
        if needs_help && matches!(phase, Phase::Playing { .. }) && menu.is_none() {
          set_taught_play.set(true);
          show_help.set(Some(Menu::PlayHelp));
        }
      },
      (phase, taught_play, menu),
    );
    let new_game = set_menu.clone();
    let immediate_reset = reset.clone();
    let new_game = EventCallback::new(move |()| {
      if interactive {
        new_game.set(Some(Menu::ConfirmNew));
      } else {
        immediate_reset.update(|value| value + 1);
      }
    });
    let open_menu = set_menu.clone();
    let open_menu = EventCallback::new(move |()| open_menu.set(Some(Menu::Pause)));
    let viewport = app_context::use_viewport_size();
    let aspect = f64::from(viewport.width) / f64::from(viewport.height);
    let input = card_input::use_card_input(game.clone(), viewport);
    let faces = if self.gallery {
      cards::deck()
    } else {
      Vec::new()
    };
    let surfaces: Vec<_> = faces
      .into_iter()
      .enumerate()
      .map(|(index, card)| {
        world::Sprite::new()
          .texture(card_assets::face(card))
          .size(0.95, 1.4)
          .position(Vector3::new(
            (index % 13) as f64 * 1.05 - 6.3,
            1.5 - (index / 13) as f64 * 1.65,
            0.0,
          ))
      })
      .collect();
    ContextProvider::new().context(preferences).child(
      Stack::new()
        .picking_mode(PickingMode::Ignore)
        .style(Style::new().width(100.pct()).height(100.pct()))
        .child((
          View::new()
            .picking_mode(PickingMode::Ignore)
            .style(Style::new().width(100.pct()).height(100.pct()))
            .child((
              View::new()
                .picking_mode(PickingMode::Ignore)
                .style(
                  Style::new()
                    .padding(18.px())
                    .color(if self.gallery {
                      Color::rgb(0.97, 0.94, 0.83)
                    } else {
                      Color::rgb(0.06, 0.12, 0.03)
                    })
                    .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
                )
                .child((
                  Heading::new(ls("Hearts"), 1).style(Style::new().font_size(32.px())),
                  Label::new(ls(format!("Hand {}", game.view.table.hand_index + 1)))
                    .style(Style::new().font_size(24.px())),
                  Button::new(ls("New game"))
                    .host_name("new-game")
                    .style(
                      Style::new()
                        .position(Position::Absolute)
                        .right(18.px())
                        .top(18.px())
                        .width(120.px())
                        .height(44.px())
                        .font_size(preferences.font().px())
                        .color(Color::rgb(0.08, 0.14, 0.06))
                        .background_color(Color::rgb(0.96, 0.92, 0.77))
                        .border_radius(6.px()),
                    )
                    .on_press(new_game.clone()),
                  interactive.then(|| {
                    Button::new(ls("Menu"))
                      .style(
                        Style::new()
                          .position(Position::Absolute)
                          .right(150.px())
                          .top(18.px())
                          .width(100.px())
                          .height(44.px())
                          .font_size(preferences.font().px())
                          .color(Color::rgb(0.08, 0.14, 0.06))
                          .background_color(Color::rgb(0.96, 0.92, 0.77))
                          .border_radius(6.px()),
                      )
                      .on_press(open_menu.clone())
                  }),
                )),
              world::SceneRoot::new(ParentScene::PrimaryScene).child((
                surfaces,
                (!self.gallery).then(|| {
                  let table = reactant::GameRoot::new(
                    MotionConfig::new(AnimatedTable {
                      view: game.view.clone(),
                      aspect,
                      inspection: input.inspection(),
                      fresh: self.fresh || generation > 0,
                      sound: interactive,
                    })
                    .reduced_motion(if preferences.reduced_motion {
                      ReducedMotion::Always
                    } else {
                      ReducedMotion::User
                    }),
                  );
                  let table = if interactive {
                    Node::new(ContextProvider::new().context(input.clone()).child(table))
                  } else {
                    Node::new(table)
                  };
                  (
                    scene::environment(aspect),
                    table,
                    reactant::GameRoot::new(particles::GameParticles { aspect }),
                  )
                }),
              )),
              interactive.then(|| {
                ContextProvider::new()
                  .context(input.clone())
                  .child(CardControls(overlay.clone()))
              }),
              (!self.gallery && input.inspection().is_none()).then(|| {
                View::new()
                  .picking_mode(PickingMode::Ignore)
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(0)
                      .top(0)
                      .width(100.pct())
                      .height(100.pct())
                      .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
                  )
                  .child(reactant::GameRoot::new(scene::seats(
                    &game.view,
                    aspect < 1.0,
                    preferences.larger_text,
                  )))
              }),
            )),
          interactive.then(|| {
            View::new()
              .picking_mode(PickingMode::Ignore)
              .style(
                Style::new()
                  .width(100.pct())
                  .height(100.pct())
                  .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
              )
              .child(audio::SoundControls {
                mix,
                set_mix: set_mix.clone(),
              })
          }),
          interactive.then(|| reactant::audio::AudioMixProvider {
            mix,
            children: Children::new(reactant::GameRoot::new(audio::GameAudio {
              phase: game.view.table.phase,
              selection: input.audio_occurrence(),
            })),
          }),
          interactive.then(|| {
            reactant::GameRoot::new((
              match_ui::PresentationPause(
                suspended
                  || matches!(
                    menu,
                    Some(Menu::Pause | Menu::Settings | Menu::Rules | Menu::ConfirmNew)
                  ),
              ),
              match_ui::Announcements,
              View::new()
                .picking_mode(PickingMode::Ignore)
                .style(Style::new().width(100.pct()).height(100.pct()))
                .child(match_ui::MatchStatus(game.clone())),
            ))
          }),
          interactive.then(|| {
            menu.map(|page| Menus {
              target: overlay.clone(),
              page,
              set_page: set_menu.clone(),
              preferences,
              set_preferences,
              mix,
              set_mix: set_mix.clone(),
              reset: EventCallback::new(move |()| reset.update(|value| value + 1)),
              exit: saves.as_ref().map(|_| {
                let exit = set_exiting.clone();
                EventCallback::new(move |()| exit.set(true))
              }),
            })
          }),
          (interactive
            && menu.is_none()
            && matches!(phase, Phase::HandOver | Phase::MatchOver { .. }))
          .then(|| match_ui::Results {
            target: overlay.clone(),
            game,
            new_game,
            menu: open_menu,
          }),
          saves.is_some().then(|| SaveStatus {
            target: overlay.clone(),
            exiting,
            set_exiting,
          }),
          crate::persistence_fixture::FixtureControls,
          OverlayHost::new(overlay),
        )),
    )
  }
}
