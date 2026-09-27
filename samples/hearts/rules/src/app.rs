use battlement::{
  AudioMix, ControllerButton, ControllerInputSettings, ObjectId, ParentScene, PhysicalKey,
  PickingMode, Prop, UiFontAddress, Vector3, object_id,
};
use reactant::{
  Application, app_context, hooks,
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
  particles, scene,
  settings::Preferences,
};

pub(crate) const ROOT: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47000");

#[derive(Clone, Copy, Eq, PartialEq)]
enum Opponents {
  Disabled,
  Scripted,
  Rollout,
}

struct HeartsRoot {
  initial: HeartsState,
  gallery: bool,
  opponents: Opponents,
  fresh: bool,
  teaching: bool,
}

/// Opens a new match through the same root used for restored state.
pub fn application() -> Application {
  self::configured(HeartsState::new(43), false, Opponents::Rollout, true)
}

/// Mounts an already validated logical match without replaying its history.
pub fn application_from_state(initial: HeartsState) -> Application {
  self::configured(initial, false, Opponents::Rollout, false)
}

pub(crate) fn exported_application() -> Application {
  match std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() {
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
  })
}

fn configured_root(root: HeartsRoot) -> Application {
  let gallery = root.gallery;
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
    let (mix, set_mix) = hooks::use_state(AudioMix {
      music: 0.22,
      effects: 0.65,
      ..AudioMix::default()
    });
    let (preferences, set_preferences) = hooks::use_state(Preferences::default());
    let teaching = self.teaching;
    let (menu, set_menu) = hooks::use_state(teaching.then_some(Menu::PassingHelp));
    let (taught_play, set_taught_play) = hooks::use_state(false);
    let (generation, reset) = hooks::use_state(0_u64);
    let initial = if generation == 0 {
      self.initial.clone()
    } else {
      HeartsState::new(43)
    };
    let policy = if self.opponents == Opponents::Scripted {
      crate::layout_fixture::scripted_decision
    } else {
      crate::ai::search::decide
    };
    let game = controller::use_hearts_with_policy(
      generation,
      move || initial,
      Seat::South,
      !interactive || menu.is_some(),
      policy,
    );
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
              match_ui::PresentationPause(matches!(
                menu,
                Some(Menu::Pause | Menu::Settings | Menu::Rules | Menu::ConfirmNew)
              )),
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
          OverlayHost::new(overlay),
        )),
    )
  }
}
