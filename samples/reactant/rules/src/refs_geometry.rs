use battlement::{
  CameraTarget, DisplayId, Label, ScrollViewMode, ScrollerVisibility, TextField, ViewportRect,
  VisualElement,
};
use battlement_reactant::prelude::*;

use crate::{
  Control, GEOMETRY_TARGET_ID, Game, Interaction, MISSING_GEOMETRY_TARGET_ID, design_system,
};

pub(crate) struct RefsGeometry {
  pub(crate) active: bool,
  pub(crate) effect_runs: u32,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for RefsGeometry {
  fn render(&self) -> impl Render {
    let field_ref = use_element_ref();
    let action_ref = field_ref.clone();
    let action_button_ref = use_element_ref();
    let restore_ref = action_button_ref.clone();
    let active = self.active;
    let world_object = if active {
      MISSING_GEOMETRY_TARGET_ID
    } else {
      GEOMETRY_TARGET_ID
    };
    let targets = (
      field_ref.clone(),
      ViewportRef::display(DisplayId(0)),
      WorldRef::origin(world_object, CameraTarget::Input),
      WorldRef::rendered_bounds(world_object, CameraTarget::Input),
    );
    let geometry = use_geometry(targets.clone());
    use_geometry_effect(
      |game: &mut Game, _| game.geometry_effect_runs += 1,
      targets,
      active,
    );
    let (field, viewport, point, bounds) = geometry.measurements;
    ScrollView::new()
      .name("refs-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .vertical_scroller_style(design_system::effects_scroller())
      .vertical_low_button_style(design_system::effects_scroll_button())
      .vertical_high_button_style(design_system::effects_scroll_button())
      .vertical_track_style(design_system::effects_scroll_track())
      .vertical_dragger_style(design_system::effects_scroll_dragger())
      .vertical_dragger_border_style(design_system::effects_scroll_dragger())
      .style(design_system::canvas(self.compact))
      .child(
        VisualElement::new()
          .name("refs-content")
          .style(design_system::refs_content())
          .child((!self.compact).then(|| {
            Label::new("REFS & GEOMETRY").style(design_system::resources_eyebrow(self.compact))
          }))
          .child(
            Label::new("Measure committed hosts")
              .name("refs-title")
              .style(design_system::effects_title(self.compact)),
          )
          .child(
            VisualElement::new()
              .name("refs-card")
              .style(design_system::refs_card(self.compact))
              .child(
                Label::new(self::overall_status(point.status, bounds.status))
                  .name("refs-status")
                  .style(design_system::refs_status(active, self.compact)),
              )
              .child(
                Label::new(format!("Effect runs · {}", self.effect_runs))
                  .name("geometry-effect-runs")
                  .style(design_system::geometry_effect_status()),
              )
              .child(
                VisualElement::new()
                  .name("refs-control-row")
                  .style(design_system::refs_control_row(self.compact))
                  .child(
                    TextField::new()
                      .name("refs-field")
                      .value("Stable reference")
                      .style(design_system::refs_field(self.compact))
                      .input_style(design_system::refs_field_input())
                      .text_element_style(design_system::refs_field_text())
                      .element_ref(field_ref),
                  )
                  .child(
                    super::interactive_button(
                      if active {
                        "RESTORE TARGET"
                      } else {
                        "SHOW UNAVAILABLE"
                      },
                      "refs-action",
                      design_system::boundary_action(
                        super::control_state(self.interaction, Control::RefsAction),
                        !active,
                        self.compact,
                      ),
                      Control::RefsAction,
                      move |game: &mut Game| {
                        if active {
                          restore_ref.focus();
                          action_ref.select_text(0, 0);
                        } else {
                          action_ref.focus();
                          action_ref.select_text(16, 0);
                        }
                        game.refs_active = !active;
                      },
                    )
                    .element_ref(action_button_ref),
                  ),
              ),
          )
          .child(
            VisualElement::new()
              .name("geometry-grid")
              .style(design_system::geometry_grid(self.compact))
              .child(self::specimen(
                "SCREEN SPACE",
                "geometry-screen",
                self::screen_value(field, viewport.latest.map(|value| value.viewport)),
                false,
                self.compact,
              ))
              .child(self::specimen(
                "WORLD ORIGIN",
                "geometry-point",
                self::point_value(point),
                active,
                self.compact,
              ))
              .child(self::specimen(
                "WORLD BOUNDS",
                "geometry-bounds",
                self::bounds_value(bounds),
                active,
                self.compact,
              )),
          ),
      )
  }
}

fn specimen(
  heading: &'static str,
  name: &'static str,
  value: String,
  unavailable: bool,
  compact: bool,
) -> impl HostRender {
  VisualElement::new()
    .name(name)
    .style(design_system::geometry_specimen(compact))
    .child(Label::new(heading).style(design_system::geometry_heading(unavailable)))
    .child(Label::new(value).style(design_system::geometry_value()))
}

fn overall_status(point: MeasurementStatus, bounds: MeasurementStatus) -> &'static str {
  if point == MeasurementStatus::Current && bounds == MeasurementStatus::Current {
    "GEOMETRY CURRENT"
  } else if matches!(point, MeasurementStatus::Unavailable(_)) {
    "TARGET UNAVAILABLE"
  } else {
    "MEASURING"
  }
}

fn screen_value(
  field: Measurement<battlement::ElementGeometry>,
  viewport: Option<ViewportRect>,
) -> String {
  match (field.latest, viewport) {
    (Some(field), Some(viewport)) => format!(
      "x {:>4.0}  y {:>4.0}\n{:>4.0} × {:>4.0} px",
      field.viewport_bound.x, field.viewport_bound.y, viewport.width, viewport.height,
    ),
    _ => self::status_text(field.status).to_owned(),
  }
}

fn point_value(point: Measurement<WorldGeometry>) -> String {
  match point.latest {
    Some(WorldGeometry::Point(value)) if point.status == MeasurementStatus::Current => format!(
      "x {:>4.0}  y {:>4.0}\ndepth {:>5.1}",
      value.point.x, value.point.y, value.depth
    ),
    _ => self::status_text(point.status).to_owned(),
  }
}

fn bounds_value(bounds: Measurement<WorldGeometry>) -> String {
  match bounds.latest {
    Some(WorldGeometry::Bounds(value)) if bounds.status == MeasurementStatus::Current => format!(
      "{:>4.0} × {:>4.0} px\ndepth {:>4.1}–{:>4.1}",
      value.bound.width, value.bound.height, value.nearest_depth, value.farthest_depth
    ),
    _ => self::status_text(bounds.status).to_owned(),
  }
}

fn status_text(status: MeasurementStatus) -> &'static str {
  match status {
    MeasurementStatus::Waiting => "WAITING FOR FRAME",
    MeasurementStatus::Current => "CURRENT",
    MeasurementStatus::Unavailable(_) => "UNAVAILABLE",
  }
}
