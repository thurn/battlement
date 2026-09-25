use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{
  AudioClipAddress, FloatValue, ObjectId, ParentScene, ParticleSpawnLocation, PrefabAddress, Prop,
  StyleValue, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  animation_controls::{
    self, AnimationScope, AnimationSequence, MotionPositionRef, MotionSelector, SequencePosition,
  },
  prelude::*,
  testing::App,
  world,
};
use reactant_testing::{Display, temporal::Clock};

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000031");
const OUTSIDE: ObjectId = object_id!("321a0000-0000-4000-8000-000000000032");

#[derive(Clone, Default)]
struct Probe(
  Rc<RefCell<Option<(AnimationScope, AnimationScope)>>>,
  Rc<RefCell<Option<reactant::app_context::AppHandle>>>,
);
struct ScopedScene(Probe);

impl Component for ScopedScene {
  fn render(&self) -> impl Render {
    self.0.1.replace(Some(reactant::app_context::use_app()));
    let ui_scope = animation_controls::use_animation_scope();
    let world_scope = animation_controls::use_animation_scope();
    self
      .0
      .0
      .replace(Some((ui_scope.clone(), world_scope.clone())));
    (
      View::new()
        .motion(MotionProps::new().animation_scope(ui_scope))
        .child(
          View::new()
            .name("opacity")
            .style(Style::new().opacity(0.0))
            .motion(MotionProps::new().motion_name("subject")),
        ),
      View::new()
        .name("outside")
        .style(Style::new().opacity(0.0))
        .motion(MotionProps::new().motion_name("subject")),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .child(
            world::Group::new()
              .id(*WORLD.as_uuid())
              .motion(MotionProps::new().motion_name("subject")),
          )
          .motion(MotionProps::new().animation_scope(world_scope)),
        world::Group::new()
          .id(*OUTSIDE.as_uuid())
          .motion(MotionProps::new().motion_name("subject")),
      )),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId, Probe) {
  let probe = Probe::default();
  let app = App::new("motion/scene").ui(ScopedScene(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  assets.add_audio_clip("motion/chime");
  assets.add_particle_effect("motion/spark");
  (Display::connect(app, assets), root, probe)
}

#[test]
fn sequence_effect_occurrences_preserve_order_deduplicate_delivery_and_capture_positions() {
  let (mut display, _root, probe) = fixture();
  let (_, scope) = probe.0.borrow().as_ref().unwrap().clone();
  scope.start(
    AnimationSequence::new()
      .label_at(
        "beat",
        SequencePosition::Absolute(Duration::from_millis(501)),
      )
      .play_sound(AudioClipAddress::from("motion/chime"))
      .at(SequencePosition::Label("beat".to_owned(), 0.0))
      .play_sound(AudioClipAddress::from("motion/chime"))
      .at(SequencePosition::Label("beat".to_owned(), 0.0))
      .animate(
        MotionSelector::identified(WORLD),
        StyleTarget::new().local_position_x(10.0),
        Transition::tween().duration_secs(1.0).ease(Easing::Linear),
      )
      .at(SequencePosition::Absolute(Duration::ZERO))
      .particle_for(
        PrefabAddress::from("motion/spark"),
        MotionPositionRef::identified(WORLD).capture_at_start(),
        Duration::from_millis(250),
      )
      .at(SequencePosition::Label("beat".to_owned(), 0.0))
      .particle_for(
        PrefabAddress::from("motion/spark"),
        MotionPositionRef::identified(WORLD).follow(),
        Duration::from_millis(250),
      )
      .at(SequencePosition::Label("beat".to_owned(), 0.0)),
  );
  display.poll();
  display.poll();
  assert!(display.audio_occurrences().is_empty());
  Clock::advance(&mut display, Duration::from_millis(500));
  assert!(display.audio_occurrences().is_empty());
  Clock::advance(&mut display, Duration::from_millis(1));
  assert_eq!(display.audio_occurrences().len(), 2);
  assert_ne!(
    display.audio_occurrences()[0].command_id,
    display.audio_occurrences()[1].command_id
  );
  assert_eq!(display.particle_occurrences().len(), 2);
  let ParticleSpawnLocation::WorldPosition(captured) = display.particle_occurrences()[0].location
  else {
    panic!("captured particle position is not concrete")
  };
  let ParticleSpawnLocation::WorldPosition(following) = display.particle_occurrences()[1].location
  else {
    panic!("following particle position is not concrete")
  };
  assert_eq!(captured.x, 0.0);
  assert!((following.x - 5.0).abs() < 0.02);
  display.poll();
  assert_eq!(display.audio_occurrences().len(), 2);
  assert_eq!(display.particle_occurrences().len(), 2);
}

fn presented(display: &Display<App>, root: ObjectId, value: f32) {
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "opacity"))
      .style()
      .opacity,
    Prop::Set(StyleValue::Value(FloatValue(value)))
  );
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.x,
    f64::from(value)
  );
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "outside"))
      .style()
      .opacity,
    Prop::Set(StyleValue::Value(FloatValue(0.0)))
  );
  assert_eq!(
    display
      .object(OUTSIDE)
      .unwrap()
      .local_transform()
      .position
      .x,
    0.0
  );
}

fn sequence(target: StyleTarget) -> AnimationSequence {
  AnimationSequence::new().animate(
    MotionSelector::name("subject"),
    target,
    Transition::tween().duration_secs(1.0).ease(Easing::Linear),
  )
}

#[test]
fn scoped_ui_and_world_targets_select_only_their_own_descendants_and_share_playback_controls() {
  let (mut display, root, probe) = fixture();
  let (ui, world) = probe.0.borrow().as_ref().unwrap().clone();
  let ui_playback = ui.start(sequence(StyleTarget::new().opacity(1.0)));
  let world_playback = world.start(sequence(StyleTarget::new().local_position_x(1.0)));
  let completed = Rc::new(Cell::new(0));
  for playback in [&ui_playback, &world_playback] {
    let counter = completed.clone();
    playback.on_complete(move || counter.set(counter.get() + 1));
  }
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(250));
  presented(&display, root, 0.25);
  ui_playback.pause();
  world_playback.pause();
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(500));
  presented(&display, root, 0.25);
  ui_playback.play();
  world_playback.play();
  display.poll();
  display.settle();
  presented(&display, root, 1.0);
  assert_eq!(completed.get(), 2);
  assert_eq!(display.presentation_time(), Duration::from_millis(1500));
  assert_eq!(display.frame(), 0);
}

#[test]
fn scoped_set_and_stop_use_the_current_host_selection() {
  let (mut display, root, probe) = fixture();
  let (ui, world) = probe.0.borrow().as_ref().unwrap().clone();
  ui.set(MotionSelector::Children, StyleTarget::new().opacity(0.5));
  world.set(
    MotionSelector::Children,
    StyleTarget::new().local_position_x(0.5),
  );
  display.poll();
  presented(&display, root, 0.5);
  let first = ui.start(sequence(StyleTarget::new().opacity(1.0)));
  let second = world.start(sequence(StyleTarget::new().local_position_x(1.0)));
  let stopped = Rc::new(Cell::new(0));
  for playback in [&first, &second] {
    let counter = stopped.clone();
    playback.on_stop(move || counter.set(counter.get() + 1));
  }
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(500));
  presented(&display, root, 0.75);
  ui.stop(MotionSelector::Descendants);
  world.stop(MotionSelector::Descendants);
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(1));
  presented(&display, root, 0.75);
  assert_eq!(stopped.get(), 2);
}

#[test]
fn sequence_labels_follow_actual_completion_and_keep_declaration_order() {
  let (mut display, _root, probe) = fixture();
  let (scope, _) = probe.0.borrow().as_ref().unwrap().clone();
  let playback = scope.start(
    AnimationSequence::new()
      .animate(
        MotionSelector::name("subject"),
        StyleTarget::new().opacity(1.0),
        Transition::tween().duration_secs(0.4).ease(Easing::Linear),
      )
      .label("arrived")
      .then(
        MotionSelector::name("subject"),
        StyleTarget::new().x(20.0),
        Transition::tween().duration_secs(0.1).ease(Easing::Linear),
      )
      .at(SequencePosition::Absolute(Duration::from_millis(100)))
      .label_at(
        "same-time-a",
        SequencePosition::Absolute(Duration::from_millis(100)),
      )
      .label_at(
        "same-time-b",
        SequencePosition::Absolute(Duration::from_millis(100)),
      ),
  );
  let events = Rc::new(RefCell::new(Vec::new()));
  for label in ["same-time-a", "same-time-b", "arrived"] {
    let events = events.clone();
    playback.on_label(label, move || events.borrow_mut().push(label));
  }
  let terminal = events.clone();
  playback.on_complete(move || terminal.borrow_mut().push("complete"));
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(100));
  assert_eq!(&*events.borrow(), &["same-time-a", "same-time-b"]);

  Clock::advance(&mut display, Duration::from_millis(100));
  playback.pause();
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(500));
  assert_eq!(&*events.borrow(), &["same-time-a", "same-time-b"]);

  playback.play();
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(200));
  assert_eq!(
    &*events.borrow(),
    &["same-time-a", "same-time-b", "arrived", "complete"]
  );
}

#[test]
fn explicit_sequence_replacement_preserves_unrelated_property_motion() {
  let (mut display, root, probe) = fixture();
  let (scope, _) = probe.0.borrow().as_ref().unwrap().clone();
  let completed = Rc::new(Cell::new(false));
  let playback = scope.start(
    AnimationSequence::new()
      .animate(
        MotionSelector::name("subject"),
        StyleTarget::new().opacity(1.0).x(10.0),
        Transition::tween().duration_secs(1.0).ease(Easing::Linear),
      )
      .then(
        MotionSelector::name("subject"),
        StyleTarget::new().opacity(0.25),
        Transition::tween().duration_secs(0.1).ease(Easing::Linear),
      )
      .at(SequencePosition::Absolute(Duration::from_millis(200)))
      .replace(),
  );
  let terminal = completed.clone();
  playback.on_complete(move || terminal.set(true));
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(200));
  Clock::advance(&mut display, Duration::from_millis(100));
  let subject = display.ui_element(display.find_ui(root, "opacity"));
  assert_eq!(
    subject.style().opacity,
    Prop::Set(StyleValue::Value(FloatValue(0.25)))
  );
  let Prop::Set(StyleValue::Value(translate)) = subject.style().translate else {
    panic!("sequence x presentation is absent");
  };
  assert_eq!(translate.x, battlement::Length::Px(3.0));
  assert!(!completed.get());

  Clock::advance(&mut display, Duration::from_millis(700));
  let Prop::Set(StyleValue::Value(translate)) = display
    .ui_element(display.find_ui(root, "opacity"))
    .style()
    .translate
  else {
    panic!("sequence x presentation is absent");
  };
  assert_eq!(translate.x, battlement::Length::Px(10.0));
  assert!(completed.get());
}

#[test]
fn scheduled_sounds_use_their_bus_and_the_mix_when_they_start() {
  let (mut display, _, probe) = fixture();
  let (_, scope) = probe.0.borrow().as_ref().unwrap().clone();
  scope.start(
    AnimationSequence::new()
      .play_sound_with(
        "motion/chime",
        animation_controls::SequenceSoundOptions {
          bus: AudioBus::Music,
          ..Default::default()
        },
      )
      .at(SequencePosition::Absolute(Duration::from_millis(100)))
      .play_sound("motion/chime")
      .at(SequencePosition::Absolute(Duration::from_millis(100))),
  );
  display.poll();
  assert!(display.audio_occurrences().is_empty());
  probe
    .1
    .borrow()
    .as_ref()
    .unwrap()
    .send(reactant::audio::set_mix(AudioMix {
      master: 0.5,
      music: 0.25,
      effects: 0.8,
      muted: false,
    }));
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(100));
  let sounds = display.audio_occurrences();
  assert_eq!(sounds.len(), 2);
  assert_eq!(sounds[0].bus, AudioBus::Music);
  assert_eq!(sounds[0].mix_gain, 0.125);
  assert_eq!(sounds[1].bus, AudioBus::Effects);
  assert_eq!(sounds[1].mix_gain, 0.4);
}
