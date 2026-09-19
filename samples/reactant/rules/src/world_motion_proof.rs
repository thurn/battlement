use battlement::{ObjectId, ParentScene, PickingMode, Prop, Vector3, object_id};
use reactant::{animation_controls, hooks, prelude::*, world};
use trox::ls;

use crate::ROOT_ID;

const CARD: ObjectId = object_id!("38210000-0000-4000-8000-000000000001");

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Pose {
  Near,
  Far,
  SpringNear,
  SpringFar,
}
const HIT: ObjectId = object_id!("38210000-0000-4000-8000-000000000002");
struct MotionProof;

pub(crate) fn app() -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(MotionProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for MotionProof {
  fn render(&self) -> impl Render {
    let controls = animation_controls::use_animation_controls::<Pose>();
    let playback = hooks::use_ref(None::<AnimationPlayback>);
    let (reduced, reduce) = hooks::use_state(false);
    let buttons = [
      ("Tween far", Pose::Far),
      ("Tween near", Pose::Near),
      ("Spring far", Pose::SpringFar),
      ("Spring near", Pose::SpringNear),
    ]
    .map(|(name, pose)| {
      let controls = controls.clone();
      let playback = playback.clone();
      Button::new(ls(name)).on_press(move || {
        playback.replace(Some(
          controls.start(animation_controls::ControlTarget::Variant(pose)),
        ));
      })
    });
    let pause = playback.clone();
    let resume = playback.clone();
    let stop = playback.clone();
    let fast = playback.clone();
    MotionConfig::new((
      View::new()
        .style(
          Style::new()
            .width(300.px())
            .padding(20.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Shared Motion"), 1),
          Label::new(ls(
            "Opacity and world X share the same clock. Hover lifts the moving card.",
          )),
          buttons,
          Button::new(ls("Pause")).on_press(move || {
            if let Some(value) = pause.get() {
              value.pause();
            }
          }),
          Button::new(ls("Resume")).on_press(move || {
            if let Some(value) = resume.get() {
              value.play();
            }
          }),
          Button::new(ls("Double speed")).on_press(move || {
            if let Some(value) = fast.get() {
              value.set_speed(2.0);
            }
          }),
          Button::new(ls("Stop")).on_press(move || {
            if let Some(value) = stop.get() {
              value.stop();
            }
          }),
          Button::new(ls("Toggle reduced motion")).on_press(reduce.update_callback(|value| !value)),
          Label::new(ls(if reduced {
            "Reduced motion: always"
          } else {
            "Reduced motion: never"
          })),
          View::new()
            .style(
              Style::new()
                .height(64.px())
                .opacity(0.0)
                .background_color(Color::rgb(0.1, 0.8, 0.7)),
            )
            .animation_controls(controls.clone())
            .variants(variants(false)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*CARD.as_uuid())
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(1.5, 2.5),
            world::BoxHitRegion::new()
              .id(*HIT.as_uuid())
              .size(Vector3::new(3.0, 3.5, 0.2)),
          ))
          .motion(
            MotionProps::new()
              .animation_controls(controls)
              .variants(variants(true))
              .while_hover(
                MotionTarget::new(StyleTarget::new().local_offset_y(0.5))
                  .transition(Transition::tween().duration_secs(0.4).ease(Easing::Linear)),
              ),
          ),
      ),
    ))
    .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear))
    .reduced_motion(if reduced {
      ReducedMotion::Always
    } else {
      ReducedMotion::Never
    })
  }
}

fn variants(world: bool) -> Variants<Pose> {
  let target = |value| {
    if world {
      StyleTarget::new().local_position_x(value)
    } else {
      StyleTarget::new().opacity(value)
    }
  };
  let spring = |value| {
    MotionTarget::new(target(value)).transition(Transition::spring().stiffness(70.0).damping(10.0))
  };
  Variants::new()
    .target(Pose::Near, target(0.0))
    .target(Pose::Far, target(1.0))
    .target(Pose::SpringNear, spring(0.0))
    .target(Pose::SpringFar, spring(1.0))
}
