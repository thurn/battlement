use std::time::Duration;

use battlement::{ObjectId, ParentScene, PickingMode, Prop, Vector3, object_id};
use reactant::{animation_controls, hooks, prelude::*, world};
use trox::ls;

use crate::ROOT_ID;

const CARD: ObjectId = object_id!("384a0000-0000-4000-8000-000000000001");
const REVEAL: ObjectId = object_id!("384a0000-0000-4000-8000-000000000002");

struct DrawReflowProof {
  destination: world::LayoutDestination,
}

pub(crate) fn app() -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(DrawReflowProof {
      destination: world::LayoutDestination::new(*CARD.as_uuid()),
    })
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.5)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for DrawReflowProof {
  fn render(&self) -> impl Render {
    let scope = animation_controls::use_animation_scope();
    let (wide, set_wide) = hooks::use_state(false);
    let (complete, set_complete) = hooks::use_state(false);
    let destination = self.destination.clone();
    let sequence_destination = destination.clone();
    let sequence_scope = scope.clone();
    let run = Button::new(ls("Run draw reflow")).on_press(move || {
      set_wide.set(false);
      set_complete.set(false);
      let playback = sequence_scope.start(
        AnimationSequence::new()
          .animate(
            sequence_destination.selector(),
            SequenceTarget::new(StyleTarget::new())
              .position(MotionPositionRef::identified(REVEAL).follow()),
            Transition::tween()
              .duration_secs(0.8)
              .ease(Easing::EaseInOut),
          )
          .then(
            sequence_destination.selector(),
            SequenceTarget::new(StyleTarget::new()).position(sequence_destination.clone()),
            Transition::spring().stiffness(360.0).damping(28.0),
          )
          .label_at(
            "reflow",
            SequencePosition::Absolute(Duration::from_millis(300)),
          ),
      );
      let label_wide = set_wide.clone();
      let completion = set_complete.clone();
      playback.on_label("reflow", move || label_wide.set(true));
      playback.on_complete(move || completion.set(true));
    });
    let hand = world::Flex::new()
      .plane(world::LayoutPlane::xy(Vector3::new(-0.5, -1.0, 0.0)))
      .extent((if wide { 7.0 } else { 4.0 }, 2.0))
      .child(world::LayoutChild::new(
        destination,
        world::LayoutBox::new(1.2, 1.8),
        world::Group::new()
          .child(
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(1.2, 1.8),
          )
          .while_hover(StyleTarget::new().local_offset_y(0.3)),
      ));
    (
      View::new()
        .style(
          Style::new()
            .width(360.px())
            .padding(22.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Draw reflow"), 1),
          Label::new(ls(
            "The reveal keeps its anchor while the hand changes underneath it.",
          )),
          run,
          Heading::new(
            ls(if complete {
              "Draw complete at latest hand target"
            } else if wide {
              "Hand reflowed while draw is running"
            } else {
              "Draw ready"
            }),
            2,
          ),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((
            world::Group::new()
              .id(*REVEAL.as_uuid())
              .position(Vector3::new(-2.5, 1.4, 0.0))
              .animate(StyleTarget::new()),
            hand,
          ))
          .motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}
