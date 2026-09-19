//! Safe, on-demand presentation diagnostics for development fixtures.

use battlement::{CameraTarget, Color, ObjectId, Style};
use reactant_core::{
  callback::Callback,
  component::Component,
  components::Button,
  element_ref::ElementRef,
  geometry::{
    Measurement, MeasurementStatus, PresentationWorkRef, WorldGeometry, WorldRef, use_geometry,
  },
  host::{Label, View},
  render::{Node, Render},
};
use trox::{ls, tx};

/// One explicitly safe object selection shown by the presentation inspector.
#[derive(Clone)]
pub enum InspectorObject {
  /// A Reactant UI host measured in viewport pixels.
  Ui {
    /// Stable game-facing identity to show in diagnostics.
    id: ObjectId,
    /// Committed UI host reference.
    element: ElementRef,
    /// Safe description of the declared visibility.
    visibility: String,
    /// Safe description of the latest Rust layout target.
    layout_target: String,
  },
  /// A world host projected through the selected camera.
  World {
    /// Stable native object identity.
    id: ObjectId,
    /// Camera used for the displayed-position observation.
    camera: CameraTarget,
    /// Safe description of the declared visibility.
    visibility: String,
    /// Safe description of the latest Rust layout target.
    layout_target: String,
  },
}

/// Hidden-by-default developer panel joining safe Rust labels with native playback state.
pub struct PresentationInspector {
  snapshot: String,
  prompt: Option<String>,
  worker: String,
  selection: Option<InspectorObject>,
}

struct InspectorPanel {
  snapshot: String,
  prompt: Option<String>,
  worker: String,
  selection: Option<InspectorObject>,
  close: Callback<()>,
}

struct UiPose {
  element: ElementRef,
}

struct WorldPose {
  object_id: ObjectId,
  camera: CameraTarget,
}

impl PresentationInspector {
  /// Creates an inspector from game-supplied labels that are safe to reveal.
  pub fn new(snapshot: impl Into<String>, worker: impl Into<String>) -> Self {
    Self {
      snapshot: snapshot.into(),
      prompt: None,
      worker: worker.into(),
      selection: None,
    }
  }

  /// Supplies a safe summary of the current prompt instead of serializing it.
  #[must_use]
  pub fn prompt(mut self, prompt: Option<impl Into<String>>) -> Self {
    self.prompt = prompt.map(Into::into);
    self
  }

  /// Selects the only object whose detailed pose should be sampled.
  #[must_use]
  pub fn selection(mut self, selection: Option<InspectorObject>) -> Self {
    self.selection = selection;
    self
  }
}

impl InspectorObject {
  /// Selects a UI host and its safe declared state.
  pub fn ui(
    id: ObjectId,
    element: ElementRef,
    visibility: impl Into<String>,
    layout_target: impl Into<String>,
  ) -> Self {
    Self::Ui {
      id,
      element,
      visibility: visibility.into(),
      layout_target: layout_target.into(),
    }
  }

  /// Selects a world host and its safe declared state.
  pub fn world(
    id: ObjectId,
    camera: CameraTarget,
    visibility: impl Into<String>,
    layout_target: impl Into<String>,
  ) -> Self {
    Self::World {
      id,
      camera,
      visibility: visibility.into(),
      layout_target: layout_target.into(),
    }
  }
}

impl Component for PresentationInspector {
  fn render(&self) -> impl Render {
    let (open, set_open) = reactant_core::hooks::use_state(false);
    if !open {
      return Node::new(
        Button::new(tx(
          "Open inspector",
          "Button that opens developer presentation diagnostics.",
        ))
        .host_name("presentation-inspector-open")
        .style(action_style())
        .on_press(set_open.callback().map_input(|()| true)),
      );
    }
    let close = set_open.callback().map_input(|()| false);
    Node::new(InspectorPanel {
      snapshot: self.snapshot.clone(),
      prompt: self.prompt.clone(),
      worker: self.worker.clone(),
      selection: self.selection.clone(),
      close,
    })
  }
}

impl Component for InspectorPanel {
  fn render(&self) -> impl Render {
    let work = use_geometry(PresentationWorkRef).measurements;
    let work = match work.status {
      MeasurementStatus::Waiting => "Native queue: waiting for first sample".to_owned(),
      MeasurementStatus::Unavailable(reason) => {
        format!("Native queue: unavailable ({reason:?})")
      }
      MeasurementStatus::Current => work.latest.map_or_else(
        || "Native queue: no sample".to_owned(),
        |value| {
          format!(
            "Native queue: {} queued batch(es) · {} blocking operation(s) · {} paused scope(s)",
            value.queued_batches, value.blocking_operations, value.paused_scopes
          )
        },
      ),
    };
    let selected = self.selection.as_ref().map_or_else(
      || {
        Node::new(Label::new(tx(
          "Selected object: none",
          "Inspector selection status.",
        )))
      },
      |selection| {
        let (id, visibility, layout_target, pose) = match selection {
          InspectorObject::Ui {
            id,
            element,
            visibility,
            layout_target,
          } => (
            id,
            visibility,
            layout_target,
            Node::new(UiPose {
              element: element.clone(),
            }),
          ),
          InspectorObject::World {
            id,
            camera,
            visibility,
            layout_target,
          } => (
            id,
            visibility,
            layout_target,
            Node::new(WorldPose {
              object_id: *id,
              camera: *camera,
            }),
          ),
        };
        Node::new(
          View::new()
            .child(Label::new(ls(format!("Selected UUID: {}", id.as_uuid()))))
            .child(Label::new(ls(format!("Declared visibility: {visibility}"))))
            .child(Label::new(ls(format!(
              "Latest layout target: {layout_target}"
            ))))
            .child(pose),
        )
      },
    );
    View::new()
      .name("presentation-inspector-panel")
      .style(panel_style())
      .child(
        Label::new(tx(
          "PRESENTATION INSPECTOR",
          "Developer presentation inspector heading.",
        ))
        .style(heading_style()),
      )
      .child(Label::new(ls(format!(
        "Latest Rust snapshot (may be ahead of Unity): {}",
        self.snapshot
      ))))
      .child(Label::new(ls(format!(
        "Safe prompt: {}",
        self
          .prompt
          .as_ref()
          .map_or_else(|| "none".to_owned(), Clone::clone)
      ))))
      .child(
        Label::new(ls(format!("Worker: {}", self.worker))).name("presentation-inspector-worker"),
      )
      .child(Label::new(ls(work)).name("presentation-inspector-work"))
      .child(selected)
      .child(
        View::new().style(actions_style()).child(
          Button::new(tx(
            "Close inspector",
            "Button that closes developer presentation diagnostics.",
          ))
          .host_name("presentation-inspector-close")
          .style(action_style())
          .on_press(self.close.clone()),
        ),
      )
  }
}

impl Component for UiPose {
  fn render(&self) -> impl Render {
    let pose = use_geometry(self.element.clone()).measurements;
    Label::new(ls(format_ui_pose(pose))).name("presentation-inspector-pose")
  }
}

impl Component for WorldPose {
  fn render(&self) -> impl Render {
    let pose = use_geometry(WorldRef::origin(self.object_id, self.camera)).measurements;
    Label::new(ls(format_world_pose(pose))).name("presentation-inspector-pose")
  }
}

fn format_ui_pose(pose: Measurement<battlement::ElementGeometry>) -> String {
  match pose.status {
    MeasurementStatus::Waiting | MeasurementStatus::Unavailable(_) => {
      format!("Displayed position: {}", status(pose.status))
    }
    MeasurementStatus::Current => pose.latest.map_or_else(
      || "Displayed position: unavailable (no current sample)".to_owned(),
      |value| {
        format!(
          "Displayed position: viewport ({:.1}, {:.1}) · native visible",
          value.viewport_bound.x, value.viewport_bound.y,
        )
      },
    ),
  }
}

fn format_world_pose(pose: Measurement<WorldGeometry>) -> String {
  match pose.status {
    MeasurementStatus::Waiting | MeasurementStatus::Unavailable(_) => {
      format!("Displayed position: {}", status(pose.status))
    }
    MeasurementStatus::Current => pose.latest.map_or_else(
      || "Displayed position: unavailable (no current sample)".to_owned(),
      |value| match value {
        WorldGeometry::Point(value) => format!(
          "Displayed position: viewport ({:.1}, {:.1}) depth {:.2} · native {}",
          value.point.x,
          value.point.y,
          value.depth,
          if value.is_inside_viewport {
            "visible"
          } else {
            "outside viewport"
          }
        ),
        _ => unreachable!("world inspector requests an origin"),
      },
    ),
  }
}

fn status(status: MeasurementStatus) -> String {
  match status {
    MeasurementStatus::Waiting => "waiting".to_owned(),
    MeasurementStatus::Current => "visible".to_owned(),
    MeasurementStatus::Unavailable(reason) => format!("unavailable ({reason:?})"),
  }
}

fn panel_style() -> Style {
  Style::new()
    .width(520.0)
    .padding(18.0)
    .background_color(Color::rgba(0.02, 0.035, 0.06, 0.97))
    .border_color(Color::rgb(0.24, 0.82, 0.92))
    .border_width(1.0)
    .color(Color::rgb(0.93, 0.97, 0.99))
}

fn heading_style() -> Style {
  Style::new()
    .font_size(20.0)
    .color(Color::rgb(0.24, 0.82, 0.92))
}

fn actions_style() -> Style {
  Style::new().flex_direction(battlement::FlexDirection::Row)
}

fn action_style() -> Style {
  Style::new()
    .padding((8, 12))
    .margin((8, 8, 0, 0))
    .background_color(Color::rgb(0.04, 0.11, 0.15))
    .border_color(Color::rgb(0.24, 0.82, 0.92))
    .border_width(1.0)
    .color(Color::WHITE)
}
