use crate::{Game, design_system};
use battlement::{
  Align, AudioClipAddress, Color, FilterFunction, FilterList, FlexDirection, FlexWrap, Length,
  LengthUnits, ObjectId, ScrollViewMode, ScrollerVisibility, Style, TransformOperation, object_id,
};
use battlement_reactant::prelude::*;
use std::time::Duration;

pub(crate) const AUDIO_CLIP: AudioClipAddress =
  AudioClipAddress::from_static("reactant/assets/clock-pulse");

const AUDIO_PLAYBACK_ID: ObjectId = object_id!("35d40000-0000-4000-8000-000000000001");

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ControlVariant {
  Rest,
  Active,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ValuesTimeControlsState {
  pub(crate) source_high: bool,
  pub(crate) audio_playing: bool,
  pub(crate) audio_paused: bool,
  pub(crate) buffering: bool,
  pub(crate) looping: bool,
  pub(crate) replacements: u32,
  pub(crate) trace: Vec<&'static str>,
}

impl Default for ValuesTimeControlsState {
  fn default() -> Self {
    Self {
      source_high: false,
      audio_playing: false,
      audio_paused: false,
      buffering: false,
      looping: true,
      replacements: 0,
      trace: vec!["graph mounted · zero frame traffic"],
    }
  }
}

#[builder]
pub(crate) struct ValuesTimeControls {
  pub(crate) state: ValuesTimeControlsState,
  pub(crate) compact: bool,
}

impl Component for ValuesTimeControls {
  fn render(&self) -> impl Render {
    let source = use_motion_value(0.0_f32);
    let checkpoint = use_state(0.0_f32).1;
    use_motion_value_event(source.clone(), MotionValueEvent::Change, move |value| {
      checkpoint.set(value);
    });
    let length = use_transform(
      source.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([Length::px(-42.0), Length::px(42.0)]),
    );
    let color = use_transform(
      source.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([
        Color::rgba(0.12, 0.78, 0.95, 1.0),
        Color::rgba(0.95, 0.3, 0.55, 1.0),
      ]),
    );
    let filters = use_transform(
      source.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([
        FilterList::new([FilterFunction::Blur(0.0), FilterFunction::Contrast(0.7)]),
        FilterList::new([FilterFunction::Blur(3.0), FilterFunction::Contrast(1.4)]),
      ]),
    );
    let transforms = use_transform(
      source.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([
        vec![TransformOperation::Rotate([0.0, 0.0, -8.0])],
        vec![TransformOperation::Rotate([0.0, 0.0, 8.0])],
      ]),
    );
    let spring = use_spring(
      source.clone(),
      SpringOptions::new().stiffness(150.0).damping(17.0),
    );
    let velocity = use_velocity(source.clone());
    let controlled_clock = use_controlled_motion_clock();
    let controlled_time = use_motion_time(MotionTimeSource::Controlled(controlled_clock.clone()));
    let controlled_x = use_transform(
      controlled_time,
      InputRange::new([
        Duration::ZERO,
        Duration::from_millis(500),
        Duration::from_secs(1),
      ]),
      OutputRange::new([-60.0, 60.0, -60.0]),
    );
    let audio = AudioPlayback::new(AUDIO_PLAYBACK_ID);
    let audio_time = use_motion_time(MotionTimeSource::Audio(audio));
    let audio_scale = use_transform(
      audio_time,
      InputRange::new([
        Duration::ZERO,
        Duration::from_millis(250),
        Duration::from_millis(500),
      ]),
      OutputRange::new([0.8, 1.3, 0.8]),
    );
    let controls = use_animation_controls::<ControlVariant>();
    let scope = use_animation_scope();

    let source_action = source.clone();
    let clock_action = controlled_clock.clone();
    let control_action = controls.clone();
    let scope_action = scope.clone();
    ScrollView::new()
      .name("values-time-controls-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(Label::new("UNITY-LOCAL VALUES · EXPLICIT CHECKPOINTS").style(eyebrow()))
      .child(
        Label::new("Values, Time & Controls")
          .name("page-title")
          .style(title()),
      )
      .child(
        Label::new(format!(
          "SOURCE {} · AUDIO {} · BUFFER {} · LOOP {} · REPLACEMENTS {}",
          if self.state.source_high { "1.0" } else { "0.0" },
          if self.state.audio_playing {
            if self.state.audio_paused {
              "PAUSED"
            } else {
              "PLAYING"
            }
          } else {
            "STOPPED"
          },
          if self.state.buffering {
            "FROZEN"
          } else {
            "READY"
          },
          if self.state.looping { "ON" } else { "OFF" },
          self.state.replacements,
        ))
        .name("values-transport-status")
        .style(status()),
      )
      .child(
        View::new()
          .style(control_row())
          .child(action("SOURCE", "values-source", move |game| {
            game.values_time_controls.source_high = !game.values_time_controls.source_high;
            source_action.set(if game.values_time_controls.source_high {
              1.0
            } else {
              0.0
            });
            game.values_time_controls.trace.push("source retargeted");
          }))
          .child(action("TIME +250", "values-time", move |game| {
            clock_action.advance(Duration::from_millis(250));
            game.values_time_controls.trace.push("controlled +250ms");
          }))
          .child(action("CONTROLS", "values-controls", move |game| {
            control_action.start(ControlVariant::Active);
            game
              .values_time_controls
              .trace
              .push("controls snapshot started");
          }))
          .child(action("SEQUENCE", "values-sequence", move |game| {
            scope_action.start(
              AnimationSequence::new()
                .animate(
                  MotionSelector::Children,
                  StyleTarget::new().opacity(1.0).x(28.0),
                  Transition::tween().duration_secs(0.24),
                )
                .then(
                  MotionSelector::name("sequence-b"),
                  StyleTarget::new().opacity(0.45).x(-18.0),
                  Transition::spring().stiffness(170.0).damping(18.0),
                )
                .at(SequencePosition::WithPrevious(0.08)),
            );
            game
              .values_time_controls
              .trace
              .push("scope selector snapshot");
          }))
          .child(audio_controls(audio)),
      )
      .child(
        View::new()
          .name("values-shared-graph")
          .style(gallery())
          .child(probe(
            "RANGE · LENGTH",
            StyleTarget::new().x_length_value(length),
          ))
          .child(probe(
            "COLOR",
            StyleTarget::new().background_color_value(color),
          ))
          .child(probe("FILTER", StyleTarget::new().filter_value(filters)))
          .child(probe(
            "TRANSFORM",
            StyleTarget::new().transform_list_value(transforms),
          ))
          .child(probe("SPRING", StyleTarget::new().scale_value(spring)))
          .child(probe("VELOCITY", StyleTarget::new().x_value(velocity))),
      )
      .child(
        View::new()
          .style(gallery())
          .child(probe(
            "CONTROLLED TIME",
            StyleTarget::new().x_value(controlled_x),
          ))
          .child(probe(
            "AUDIO PLAYHEAD",
            StyleTarget::new().scale_value(audio_scale),
          ))
          .child(
            View::new()
              .style(probe_style())
              .animation_controls(controls)
              .variants(
                Variants::<ControlVariant, ()>::new()
                  .target(
                    ControlVariant::Rest,
                    StyleTarget::new().opacity(0.45).x(0.0),
                  )
                  .target(
                    ControlVariant::Active,
                    StyleTarget::new().opacity(1.0).x(34.0),
                  ),
              )
              .animate_variant(ControlVariant::Rest)
              .child(Label::new("TYPED CONTROLS").style(probe_text())),
          ),
      )
      .child(
        View::new()
          .name("values-sequence-scope")
          .animation_scope(scope)
          .style(gallery())
          .child(sequence_probe("sequence-a", "SCOPE A"))
          .child(sequence_probe("sequence-b", "SCOPE B")),
      )
      .child(
        Label::new(format!("TRACE  {}", self.state.trace.join("  ›  ")))
          .name("values-trace")
          .style(trace()),
      )
      .child(action("GESTURES & DRAG", "gestures-navigation", |game| {
        game.screen = crate::Screen::GesturesDrag;
      }))
  }
}

fn audio_controls(audio: AudioPlayback) -> View {
  let app = use_app();
  View::new()
    .style(control_row())
    .child(action("PLAY", "audio-play", {
      let app = app.clone();
      move |game| {
        app.send(audio.play_command(
          AUDIO_CLIP,
          AudioPlaybackOptions::new().looping(game.values_time_controls.looping),
        ));
        game.values_time_controls.audio_playing = true;
        game.values_time_controls.audio_paused = false;
        game.values_time_controls.trace.push("audio play");
      }
    }))
    .child(action("PAUSE", "audio-pause", {
      let app = app.clone();
      move |game| {
        app.send(audio.pause());
        game.values_time_controls.audio_paused = true;
        game.values_time_controls.trace.push("audio pause");
      }
    }))
    .child(action("RESUME", "audio-resume", {
      let app = app.clone();
      move |game| {
        app.send(audio.resume());
        game.values_time_controls.audio_paused = false;
        game.values_time_controls.trace.push("audio resume");
      }
    }))
    .child(action("BUFFER", "audio-buffer", {
      let app = app.clone();
      move |game| {
        game.values_time_controls.buffering = !game.values_time_controls.buffering;
        app.send(audio.set_buffering(game.values_time_controls.buffering));
        game.values_time_controls.trace.push("buffer boundary");
      }
    }))
    .child(action("SEEK", "audio-seek", {
      let app = app.clone();
      move |game| {
        app.send(audio.seek(Duration::from_millis(350)));
        game.values_time_controls.trace.push("seek discontinuity");
      }
    }))
    .child(action("LOOP", "audio-loop", {
      let app = app.clone();
      move |game| {
        game.values_time_controls.looping = !game.values_time_controls.looping;
        app.send(audio.replace(AUDIO_CLIP));
        game
          .values_time_controls
          .trace
          .push("loop / replace discontinuity");
      }
    }))
    .child(action("REPLACE", "audio-replace", {
      let app = app.clone();
      move |game| {
        app.send(audio.replace(AUDIO_CLIP));
        game.values_time_controls.replacements += 1;
        game.values_time_controls.trace.push("clip replaced");
      }
    }))
    .child(action("STOP", "audio-stop", {
      let app = app.clone();
      move |game| {
        app.send(audio.stop(Duration::ZERO));
        game.values_time_controls.audio_playing = false;
        game.values_time_controls.audio_paused = false;
        game.values_time_controls.trace.push("audio stopped");
      }
    }))
}

fn probe(label: &'static str, motion: StyleTarget) -> View {
  View::new()
    .style(probe_style())
    .animate(motion)
    .child(Label::new(label).style(probe_text()))
}

fn sequence_probe(name: &'static str, label: &'static str) -> View {
  View::new()
    .name(name)
    .motion_name(name)
    .style(probe_style())
    .child(Label::new(label).style(probe_text()))
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(text)
    .name(name)
    .style(action_style())
    .on_click(callback)
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(28.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.98, 0.4, 0.16))
}

fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .margin((6, 0, 10, 0))
}

fn status() -> Style {
  Style::new()
    .font_size(15.0)
    .color(Color::rgb(0.55, 0.82, 0.78))
}

fn control_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((10, 0))
}

fn action_style() -> Style {
  Style::new()
    .height(36.0)
    .min_width(88.0)
    .margin((0, 6, 6, 0))
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(1.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((8, 0))
}

fn probe_style() -> Style {
  Style::new()
    .width(180.0)
    .height(72.0)
    .margin((0, 8, 8, 0))
    .align_items(Align::Center)
    .background_color(Color::rgb(0.04, 0.11, 0.14))
    .border_color(Color::rgb(0.18, 0.38, 0.42))
    .border_width(1.0)
}

fn probe_text() -> Style {
  Style::new()
    .font_size(13.0)
    .color(Color::rgb(0.9, 0.96, 0.98))
}

fn trace() -> Style {
  Style::new()
    .font_size(13.0)
    .color(Color::rgb(0.64, 0.72, 0.75))
    .margin((8, 0))
}
