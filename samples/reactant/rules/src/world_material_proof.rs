use crate::{CONTENT_SCENE, Game, ROOT_ID, model};
use battlement::{
  AudioClipAddress, MaterialInstance, MaterialParameter, MaterialVector, ParentScene, Vector3,
  object_id,
};
use reactant::{app::App, hooks, prelude::*, world};
use trox::ls;

const AUDIO_PLAYBACK_ID: battlement::ObjectId = object_id!("38220000-0000-4000-8000-000000000001");
const AUDIO_CLIP: AudioClipAddress = AudioClipAddress::from_static("reactant/assets/clock-pulse");
const CLIP: MaterialParameter<f64> = MaterialParameter::new("_Clip");
const ACCENT: MaterialParameter<Color> = MaterialParameter::new("_Accent");
const WARP: MaterialParameter<MaterialVector> = MaterialParameter::new("_Warp");
const MATERIAL: &str = "reactant/world/card-material";

struct MaterialProof;
pub(crate) fn app() -> App<Game> {
  App::with_model(CONTENT_SCENE, model::new())
    .ui(MaterialProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
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
impl Component for MaterialProof {
  fn render(&self) -> impl Render {
    let (changed, change) = hooks::use_state(false);
    let (cleared, clear) = hooks::use_state(false);
    let (faded, fade) = hooks::use_state(false);
    let (audio_ready, set_audio_ready) = hooks::use_state(false);
    let (parameter_motion, set_parameter_motion) = hooks::use_state(false);
    let app = reactant::app_context::use_app();
    let audio = AudioPlayback::new(AUDIO_PLAYBACK_ID);
    let transition = Transition::tween().duration_secs(1.0).ease(Easing::Linear);
    let mut left = world::Sprite::new()
      .texture("reactant/assets/texture")
      .size(2.3, 3.4)
      .opacity(if faded { 0.4 } else { 1.0 })
      .layer(0);
    if !cleared {
      left = left.material(
        MaterialInstance::new(MATERIAL)
          .parameter(CLIP, if changed { 0.7 } else { 0.12 })
          .parameter(
            ACCENT,
            if changed {
              Color::rgb(1.0, 0.45, 0.15)
            } else {
              Color::rgb(0.35, 0.7, 1.0)
            },
          )
          .parameter(WARP, MaterialVector([0.0; 4])),
      );
    }
    let motion_card = world::Sprite::new()
      .texture("reactant/assets/texture")
      .size(1.4, 1.8)
      .layer(1)
      .position(Vector3::new(5.3, -2.2, 0.0))
      .material(MaterialInstance::new(MATERIAL).parameter(CLIP, 0.15));
    let motion_card = if parameter_motion {
      Node::new(
        motion_card
          .animate(StyleTarget::new().material_scalar(CLIP, 0.55))
          .transition(transition.clone()),
      )
    } else {
      Node::new(motion_card)
    };
    let light = world::Light::new().intensity(0.35);
    let light = if parameter_motion {
      Node::new(
        light
          .animate(StyleTarget::new().light_intensity(2.0))
          .transition(transition.clone()),
      )
    } else {
      Node::new(light)
    };
    let audio_host = world::Group::new();
    let audio_host = if parameter_motion && audio_ready {
      Node::new(
        audio_host
          .animate(StyleTarget::new().audio_volume(audio, 0.25))
          .transition(transition),
      )
    } else {
      Node::new(audio_host)
    };
    let parameter_status = match (audio_ready, parameter_motion) {
      (false, false) => "Parameter motion: ready",
      (false, true) => "Parameter motion: waiting for audio",
      (true, false) => "Parameter motion: audio ready",
      (true, true) => "Parameter motion: running",
    };
    (
      View::new()
        .style(
          Style::new()
            .width(300.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Independent card materials"), 1),
          Label::new(ls(
            "Three cards share one material. Only the left card changes.",
          )),
          Button::new(ls("Change left card")).on_press(change.update_callback(|v| !v)),
          Button::new(ls("Fade artwork")).on_press(fade.update_callback(|v| !v)),
          Button::new(ls("Clear left overrides")).on_press(clear.update_callback(|v| !v)),
          Button::new(ls("Play parameter audio")).on_press({
            let app = app.clone();
            move |_: &mut Game| {
              app.send(audio.play_command(AUDIO_CLIP, AudioPlaybackOptions::new().looping(true)));
              set_audio_ready.set(true);
            }
          }),
          Button::new(ls("Animate parameters"))
            .on_press(move |_: &mut Game| set_parameter_motion.set(true)),
          Heading::new(ls(parameter_status), 2),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .position(Vector3::new(-2.3, 0.3, 0.0))
          .child((left, self::label("LEFT", -2.1))),
        world::Group::new()
          .position(Vector3::new(0.8, 0.3, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(2.3, 3.4)
              .layer(0)
              .material(
                MaterialInstance::new(MATERIAL)
                  .parameter(CLIP, 0.35)
                  .parameter(ACCENT, Color::rgb(0.3, 1.0, 0.6)),
              ),
            self::label("RIGHT", -2.1),
          )),
        world::Group::new()
          .position(Vector3::new(3.9, 0.3, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(2.3, 3.4)
              .layer(0)
              .material(MaterialInstance::new(MATERIAL)),
            self::label("SOURCE", -2.1),
          )),
        motion_card,
        light,
        audio_host,
      )),
    )
  }
}
fn label(text: &str, y: f64) -> world::Text {
  world::Text::new()
    .font("reactant/world/font")
    .text(text)
    .size(4.5)
    .position(Vector3::new(0.0, y, 0.0))
    .layer(1)
}
