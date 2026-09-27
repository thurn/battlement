use battlement::{
  CommandBody, GameObjectKind, ParentScene, ParticlePlayPayload,
  application::ReducedMotionPreference,
};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{Application, hooks, prelude::*, world};
use reactant_testing::Display;
use trox::ls;

struct Fixture;

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let (seed, set_seed) = hooks::use_state(7_u32);
    let (enabled, enable) = hooks::use_state(true);
    let (alternate, replace) = hooks::use_state(false);
    let (revision, rerender) = hooks::use_state(0_u32);
    (
      Button::new(ls("Seed")).on_press(set_seed.update_callback(|v| v + 1)),
      Button::new(ls("Enable")).on_press(enable.update_callback(|v| !v)),
      Button::new(ls("Replace")).on_press(replace.update_callback(|v| !v)),
      Button::new(ls("Rerender")).on_press(rerender.update_callback(|v| v + 1)),
      Label::new(ls(format!("Revision {revision}"))),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::ParticleEmitter::new(
          if alternate {
            "particles/b"
          } else {
            "particles/a"
          },
          seed,
        )
        .enabled(enabled),
      ),
    )
  }
}

#[test]
fn emitter_owns_restart_replacement_disablement_and_motion_preference() {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("particles/scene");
  for address in ["particles/a", "particles/b"] {
    catalog.add_prefab(address, FakePrefab::new().with_particle_systems());
  }
  let mut display = Display::mount(
    || Application::new("particles/scene").child(Fixture),
    catalog,
  );
  display.flush();
  assert_eq!(self::plays(&display).len(), 1);
  let first = self::plays(&display)[0];
  assert_eq!(first.seed, 7);
  display.activate_accessible("Rerender");
  display.flush();
  assert_eq!(self::plays(&display).len(), 1);
  display.activate_accessible("Seed");
  display.flush();
  let next = *self::plays(&display).last().unwrap();
  assert_eq!(next.object_id, first.object_id);
  assert_eq!(next.seed, 8);
  display.activate_accessible("Replace");
  display.flush();
  let replacement = *self::plays(&display).last().unwrap();
  assert_eq!(self::prefabs(&display), 1);
  assert!(
    matches!(display.world().object(replacement.object_id).unwrap().kind(), GameObjectKind::Prefab { address, .. } if address.as_str() == "particles/b")
  );
  for _ in 0..2 {
    display.activate_accessible("Enable");
    display.flush();
    assert_eq!(self::prefabs(&display), 0);
    display.activate_accessible("Enable");
    display.flush();
    assert_eq!(self::prefabs(&display), 1);
    display.set_reduced_motion_preference(ReducedMotionPreference::Reduce);
    display.flush();
    assert_eq!(self::prefabs(&display), 0);
    display.set_reduced_motion_preference(ReducedMotionPreference::NoPreference);
    display.flush();
    assert_eq!(self::prefabs(&display), 1);
  }
}

fn plays(display: &Display) -> Vec<ParticlePlayPayload> {
  display
    .commands()
    .iter()
    .filter_map(|entry| match entry.command.body {
      CommandBody::ParticlePlay(value) => Some(value),
      _ => None,
    })
    .collect()
}

fn prefabs(display: &Display) -> usize {
  display
    .world()
    .objects()
    .filter(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
    .count()
}
