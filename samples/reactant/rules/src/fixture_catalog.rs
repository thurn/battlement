use crate::{CONTENT_SCENE, ROOT_ID, ReactantApplication, Screen};
use reactant::{Application, prelude::*};
use trox::ls;

type Builder = fn() -> ReactantApplication;

#[derive(Clone, Copy)]
pub(crate) enum FixtureStatus {
  Available(Builder),
  Unavailable { owner: &'static str },
}

/// One stable laboratory selector and its implementation status.
#[derive(Clone, Copy)]
pub(crate) struct Fixture {
  pub(crate) selector: &'static str,
  pub(crate) status: FixtureStatus,
}

macro_rules! available {
  ($selector:literal, $builder:expr) => {
    Fixture {
      selector: $selector,
      status: FixtureStatus::Available($builder),
    }
  };
}

macro_rules! unavailable {
  ($selector:literal, $owner:literal) => {
    Fixture {
      selector: $selector,
      status: FixtureStatus::Unavailable { owner: $owner },
    }
  };
}

pub(crate) const FIXTURES: &[Fixture] = &[
  available!("delivery-diagnostics", crate::delivery_proof::app),
  available!("motion-ui", motion_ui),
  available!("motion-reduced", motion_reduced),
  available!("motion-sequence", motion_sequence),
  available!("shared-motion", crate::world_motion_proof::app),
  available!("effect-occurrences", crate::effect_occurrence_proof::app),
  available!(
    "effect-exit-retention",
    crate::effect_exit_retention_proof::app
  ),
  available!("gameplay-pause", crate::gameplay_pause_proof::app),
  available!("world-navigation", crate::navigation_proof::app),
  available!("pointer-routing", crate::pointer_proof::app),
  available!("world-hits", crate::world_hit_proof::app),
  available!("world-layout", crate::world_layout_proof::app),
  available!("draw-reflow", crate::draw_reflow_proof::app),
  available!("world-materials", crate::world_material_proof::app),
  available!("world-text", crate::world_text_proof::app),
  available!("world-primitives", crate::world_proof::app),
  available!("stable-selectors", stable_selectors),
  available!("stable-props", stable_props),
  available!("destruction-queue", crate::destruction_queue_proof::app),
  available!("presentation-lifetime", crate::lifetime_proof::app),
  available!("presentation-identity", crate::identity_proof::app),
  available!(
    "presentation-identity-perspective",
    crate::identity_proof::perspective_app
  ),
  available!("mixed-tree", crate::mixed_proof::app),
  available!("rules-prompts", crate::prompt_proof::app),
  available!("rules-session", crate::session_proof::app),
  available!("rules-batches", crate::batch_proof::app),
  available!("rules-batches-auto", crate::batch_proof::automatic_app),
  available!("presentation-inspector", crate::batch_proof::inspector_app),
  unavailable!("identity-transfer", "44a"),
  unavailable!("duplicate-identity", "44a"),
  unavailable!("visibility-transition", "44a"),
  unavailable!("ui-world-transfer", "44a"),
  unavailable!("mixed-input", "44a"),
  unavailable!("composed-card", "44a"),
  unavailable!("contained-layout", "44a"),
  unavailable!("stores", "44a"),
  unavailable!("card-browser", "44b"),
  unavailable!("card-selection-scenes", "44b"),
  unavailable!("simulation-preview", "44b"),
  unavailable!("motion-equivalence", "45b"),
  unavailable!("material-effects", "45b"),
  unavailable!("attached-effects", "45b"),
  unavailable!("occurrence-delivery", "45b"),
  unavailable!("prompt-cycle", "45b"),
  unavailable!("cancellation", "45a"),
  unavailable!("asset-loading", "45a"),
  unavailable!("snapshot-queue", "45a"),
  unavailable!("save-failure", "45a"),
];

pub(crate) fn build(selector: &str) -> ReactantApplication {
  let fixture = FIXTURES
    .iter()
    .find(|fixture| fixture.selector == selector)
    .unwrap_or_else(|| panic!("unknown Reactant laboratory fixture `{selector}`"));
  match fixture.status {
    FixtureStatus::Available(builder) => builder(),
    FixtureStatus::Unavailable { owner } => unavailable(selector, owner),
  }
}

fn unavailable(selector: &str, owner: &str) -> ReactantApplication {
  Application::new(CONTENT_SCENE)
    .child(
      View::new()
        .style(
          Style::new()
            .width(100.0_f32.pct())
            .padding(32.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.04, 0.07, 0.12)),
        )
        .child((
          Heading::new(ls("Fixture unavailable"), 1),
          Text::new(ls(format!(
            "{selector} is owned by task {owner} and is not implemented yet"
          ))),
        )),
    )
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.name("battlement-reactant-unavailable")
    })
}

fn motion_ui() -> ReactantApplication {
  crate::app_setup::application_with_screen(Some(Screen::TargetsTimelines))
}

fn motion_reduced() -> ReactantApplication {
  crate::app_setup::application_with_screen(Some(Screen::ComposedEffects))
}

fn motion_sequence() -> ReactantApplication {
  crate::app_setup::application_with_screen(Some(Screen::ValuesTimeControls))
}

fn stable_selectors() -> ReactantApplication {
  crate::selector_proof::app(false)
}

fn stable_props() -> ReactantApplication {
  crate::selector_proof::app(true)
}

#[cfg(test)]
mod tests {
  use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
  };

  use battlement_fake::assets::FakeAssetCatalog;
  use reactant_testing::Display;

  use super::*;

  #[test]
  fn selectors_are_unique_and_ditto_uses_available_fixtures() {
    let selectors = FIXTURES
      .iter()
      .map(|fixture| fixture.selector)
      .collect::<BTreeSet<_>>();
    assert_eq!(selectors.len(), FIXTURES.len());

    for line in include_str!("../../ditto.toml").lines() {
      let Some(selector) = line.trim().strip_prefix("fixture = \"") else {
        continue;
      };
      let selector = selector.trim_end_matches('"');
      let fixture = FIXTURES
        .iter()
        .find(|fixture| fixture.selector == selector)
        .unwrap_or_else(|| panic!("Ditto fixture `{selector}` is not catalogued"));
      assert!(
        matches!(fixture.status, FixtureStatus::Available(_)),
        "Ditto fixture `{selector}` is unavailable"
      );
    }
  }

  #[test]
  fn later_scene_selectors_are_explicitly_unavailable() {
    for selector in [
      "identity-transfer",
      "simulation-preview",
      "cancellation",
      "save-failure",
    ] {
      let fixture = FIXTURES
        .iter()
        .find(|fixture| fixture.selector == selector)
        .expect("planned fixture is catalogued");
      assert!(matches!(fixture.status, FixtureStatus::Unavailable { .. }));
    }

    let mut display = Display::mount(|| build("identity-transfer"), catalog());
    display.poll();
    assert!(has_label(&display, "Fixture unavailable"));
    assert!(has_label(
      &display,
      "identity-transfer is owned by task 44a and is not implemented yet"
    ));
  }

  #[test]
  fn available_scenes_reset_through_fresh_engine_entrypoints() {
    let mut display = Display::mount(|| build("stable-selectors"), catalog());
    display.poll();
    activate(&mut display, "Change settings");
    assert!(has_label(&display, "Settings: 1"));
    drop(display);

    let mut display = Display::mount(|| build("draw-reflow"), catalog());
    display.poll();
    display.settle();
    display.poll();
    drop(display);

    let mut display = Display::mount(|| build("rules-session"), catalog());
    display.poll();
    wait_for_label(&mut display, "Choose number");
    activate(&mut display, "Choose number");
    wait_for_label(&mut display, "Answer five");
    drop(display);

    let mut display = Display::mount(|| build("stable-selectors"), catalog());
    display.poll();
    assert!(has_label(&display, "Settings: 0"));
    assert!(has_label(&display, "Score 1 / local 0"));
  }

  fn activate(display: &mut Display, label: &str) {
    let object_id = display
      .accessibility()
      .nodes
      .iter()
      .find(|node| node.label.as_deref() == Some(label))
      .unwrap_or_else(|| panic!("missing accessible control {label}"))
      .object_id;
    display.ui().deliver_event(battlement::UiEvent {
      target_id: object_id,
      cancelable: true,
      default_prevented: false,
      body: battlement::UiEventBody::AccessibilityAction(battlement::UiAccessibilityActionEvent {
        backend_generation: 1,
        action: battlement::UiAccessibilityAction::Activate,
      }),
    });
    display.poll();
  }

  fn has_label(display: &Display, label: &str) -> bool {
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some(label))
  }

  fn wait_for_label(display: &mut Display, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !has_label(display, label) {
      assert!(Instant::now() < deadline, "timed out waiting for {label}");
      std::thread::yield_now();
      display.poll();
    }
  }

  fn catalog() -> FakeAssetCatalog {
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene(crate::CONTENT_SCENE);
    catalog.add_textures(crate::generated_asset_addresses());
    catalog.add_texture("reactant/assets/texture");
    catalog.add_material(crate::MOTION_MATERIAL);
    catalog.add_material_with_parameters(
      "reactant/world/card-material",
      [
        battlement::MaterialParameter::<battlement::Color>::new("_Accent")
          .value(battlement::Color::WHITE),
        battlement::MaterialParameter::<f64>::new("_Clip").value(0.0),
        battlement::MaterialParameter::<battlement::MaterialVector>::new("_Warp")
          .value(battlement::MaterialVector([0.0; 4])),
      ],
    );
    catalog.add_text_mesh_pro_font("reactant/world/font");
    catalog.add_audio_clip("reactant/assets/clock-pulse");
    catalog.add_particle_effect("reactant/effect-burst");
    catalog
  }
}
