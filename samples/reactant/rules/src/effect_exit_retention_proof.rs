use std::time::Duration;

use battlement::{MaterialInstance, MaterialParameter, ParentScene, Vector3};
use reactant::{animation_controls, hooks, native_host, prelude::*, world};
use trox::ls;

use crate::ROOT_ID;

const CLIP: MaterialParameter<f64> = MaterialParameter::new("_Clip");
const MATERIAL: &str = "reactant/world/card-material";

struct EffectExitRetentionProof;

struct Card {
  scope: AnimationScope,
  reference: native_host::ObjectRef,
  reverse: bool,
  set_status: StateSetter<u8>,
}

pub(crate) fn app() -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(EffectExitRetentionProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.5)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.025, 0.04, 0.075))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for EffectExitRetentionProof {
  fn render(&self) -> impl Render {
    let scope = animation_controls::use_animation_scope();
    let target = native_host::use_object_ref();
    let projectile = native_host::use_object_ref();
    let retained = hooks::use_ref(None::<AnimationPlayback>);
    let (open, set_open) = hooks::use_state(true);
    let (reverse, set_reverse) = hooks::use_state(false);
    let (status, set_status) = hooks::use_state(0_u8);

    let exit_scope = scope.clone();
    let exit_target = target.clone();
    let exit_projectile = projectile.clone();
    let exit_retained = retained.clone();
    let exit_status = set_status.clone();
    let close = set_open.clone();
    let launch = move || {
      if !open {
        return;
      }
      let sequence = AnimationSequence::new()
        .animate(
          MotionSelector::name("dissolve-surface"),
          StyleTarget::new().material_scalar(CLIP, 1.0),
          Transition::tween().duration_secs(1.0).ease(Easing::Linear),
        )
        .at(SequencePosition::Absolute(Duration::ZERO))
        .animate(
          MotionSelector::name("dissolve-label"),
          StyleTarget::new().opacity(0.0),
          Transition::tween().duration_secs(0.65).ease(Easing::Linear),
        )
        .at(SequencePosition::Absolute(Duration::ZERO))
        .animate(
          MotionSelector::object(exit_projectile.clone()),
          SequenceTarget::new(StyleTarget::new()).position(
            exit_target
              .local_point(Vector3::new(0.9, 0.45, 0.0))
              .follow(),
          ),
          Transition::tween().duration_secs(1.4).ease(Easing::Linear),
        )
        .at(SequencePosition::Absolute(Duration::ZERO));
      let playback = exit_scope.start(sequence);
      playback.on_complete({
        let exit_status = exit_status.clone();
        move || exit_status.set(2)
      });
      exit_retained.replace(Some(playback));
      exit_status.set(1);
      close.set(false);
    };
    let restore_status = set_status.clone();
    let restore = move || {
      if open {
        return;
      }
      set_reverse.set(true);
      restore_status.set(3);
      set_open.set(true);
    };
    let status = match status {
      0 => "Retained exit: ready",
      1 => "Retained exit: playing",
      2 => "Retained exit: complete",
      3 => "Reverse dissolve: playing",
      _ => "Reverse dissolve: complete",
    };
    (
      View::new()
        .style(
          Style::new()
            .width(360.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.05, 0.08, 0.14)),
        )
        .child((
          Heading::new(ls("Retained effects"), 1),
          Heading::new(ls(status), 2),
          Button::new(ls("Launch retained exit")).on_press(launch),
          Button::new(ls("Restore with reverse dissolve")).on_press(restore),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((
            world::Group::new()
              .position(Vector3::new(-2.8, -0.8, 0.0))
              .reference(projectile)
              .child(
                world::Sprite::new()
                  .texture("reactant/assets/texture")
                  .size(0.42, 0.42),
              )
              .with_motion(MotionProps::new().motion_name("projectile")),
            AnimatePresence::new().child(open.then(|| {
              Card {
                scope: scope.clone(),
                reference: target.clone(),
                reverse,
                set_status: set_status.clone(),
              }
              .key("retained-card")
            })),
          ))
          .with_motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}

impl Component for Card {
  fn render(&self) -> impl Render {
    let retained = hooks::use_ref(None::<AnimationPlayback>);
    let reverse = self.reverse;
    let scope = self.scope.clone();
    let set_status = self.set_status.clone();
    hooks::use_effect(
      move || {
        if reverse {
          let sequence = AnimationSequence::new()
            .animate(
              MotionSelector::name("dissolve-surface"),
              StyleTarget::new().material_scalar(CLIP, 0.0),
              Transition::tween().duration_secs(0.8).ease(Easing::Linear),
            )
            .at(SequencePosition::Absolute(Duration::ZERO))
            .animate(
              MotionSelector::name("dissolve-label"),
              StyleTarget::new().opacity(1.0),
              Transition::tween().duration_secs(0.6).ease(Easing::Linear),
            )
            .at(SequencePosition::Absolute(Duration::ZERO));
          let playback = scope.start(sequence);
          playback.on_complete(move || set_status.set(4));
          retained.replace(Some(playback));
        }
        || {}
      },
      reverse,
    );
    let clip = if self.reverse { 1.0 } else { 0.0 };
    let opacity = if self.reverse { 0.0 } else { 1.0 };
    world::Group::new()
      .position(Vector3::new(1.7, 0.0, 0.0))
      .reference(self.reference.clone())
      .child((
        surface(Vector3::new(-0.35, 0.0, 0.0), clip),
        surface(Vector3::new(0.35, 0.0, 0.0), clip),
        world::Text::new()
          .font("reactant/world/font")
          .text("RETAINED")
          .size(4.4)
          .opacity(opacity)
          .position(Vector3::new(0.0, -1.35, 0.0))
          .layer(2)
          .with_motion(MotionProps::new().motion_name("dissolve-label")),
      ))
      .with_motion(MotionProps::new().motion_name("dissolve-anchor"))
  }
}

fn surface(position: Vector3, clip: f64) -> impl Render {
  world::Sprite::new()
    .texture("reactant/assets/texture")
    .size(1.35, 2.1)
    .position(position)
    .material(MaterialInstance::new(MATERIAL).parameter(CLIP, clip))
    .with_motion(
      MotionProps::new()
        .animate(StyleTarget::new().material_scalar(CLIP, clip as f32))
        .motion_name("dissolve-surface"),
    )
}
