//! Inline presentation access without producing UI command journal entries.

use battlement_types::{Color, ObjectId};
use battlement_ui::{
  FloatValue, Length, LengthOrAuto, MotionProperty, MotionValue, Prop, Rotate, Scale, StyleValue,
  Translate, UiVisualElementProperties,
};

use crate::UiWorld;

impl UiWorld {
  /// Reads a sampled property represented by the fake's inline style adapter.
  #[must_use]
  pub fn motion_value(&self, host: ObjectId, property: MotionProperty) -> Option<MotionValue> {
    let style = self.elements.get(&host)?.style();
    Some(match property {
      MotionProperty::Opacity => MotionValue::Scalar(value(&style.opacity).map_or(1.0, |v| v.0)),
      MotionProperty::FlexGrow => MotionValue::Scalar(value(&style.flex_grow).map_or(0.0, |v| v.0)),
      MotionProperty::FlexShrink => {
        MotionValue::Scalar(value(&style.flex_shrink).map_or(1.0, |v| v.0))
      }
      MotionProperty::FontSize => {
        MotionValue::Length(value(&style.font_size).copied().unwrap_or(Length::Px(0.0)))
      }
      MotionProperty::LetterSpacing => MotionValue::Length(
        value(&style.letter_spacing)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::WordSpacing => MotionValue::Length(
        value(&style.word_spacing)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::PaddingBottom => MotionValue::Length(
        value(&style.padding_bottom)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::PaddingLeft => MotionValue::Length(
        value(&style.padding_left)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::PaddingRight => MotionValue::Length(
        value(&style.padding_right)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::PaddingTop => MotionValue::Length(
        value(&style.padding_top)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::BorderBottomLeftRadius => MotionValue::Length(
        value(&style.border_bottom_left_radius)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::BorderBottomRightRadius => MotionValue::Length(
        value(&style.border_bottom_right_radius)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::BorderTopLeftRadius => MotionValue::Length(
        value(&style.border_top_left_radius)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::BorderTopRightRadius => MotionValue::Length(
        value(&style.border_top_right_radius)
          .copied()
          .unwrap_or(Length::Px(0.0)),
      ),
      MotionProperty::Width => MotionValue::Length(read_auto(&style.width)),
      MotionProperty::Height => MotionValue::Length(read_auto(&style.height)),
      MotionProperty::MinWidth => MotionValue::Length(read_auto(&style.min_width)),
      MotionProperty::MinHeight => MotionValue::Length(read_auto(&style.min_height)),
      MotionProperty::MaxWidth => MotionValue::Length(read_auto(&style.max_width)),
      MotionProperty::MaxHeight => MotionValue::Length(read_auto(&style.max_height)),
      MotionProperty::Top => MotionValue::Length(read_auto(&style.top)),
      MotionProperty::Bottom => MotionValue::Length(read_auto(&style.bottom)),
      MotionProperty::Left => MotionValue::Length(read_auto(&style.left)),
      MotionProperty::Right => MotionValue::Length(read_auto(&style.right)),
      MotionProperty::MarginTop => MotionValue::Length(read_auto(&style.margin_top)),
      MotionProperty::MarginBottom => MotionValue::Length(read_auto(&style.margin_bottom)),
      MotionProperty::MarginLeft => MotionValue::Length(read_auto(&style.margin_left)),
      MotionProperty::MarginRight => MotionValue::Length(read_auto(&style.margin_right)),
      MotionProperty::FlexBasis => MotionValue::Length(read_auto(&style.flex_basis)),
      MotionProperty::BorderTopWidth => MotionValue::Length(Length::Px(
        value(&style.border_top_width).map_or(0.0, |v| v.0),
      )),
      MotionProperty::BorderBottomWidth => MotionValue::Length(Length::Px(
        value(&style.border_bottom_width).map_or(0.0, |v| v.0),
      )),
      MotionProperty::BorderLeftWidth => MotionValue::Length(Length::Px(
        value(&style.border_left_width).map_or(0.0, |v| v.0),
      )),
      MotionProperty::BorderRightWidth => MotionValue::Length(Length::Px(
        value(&style.border_right_width).map_or(0.0, |v| v.0),
      )),
      MotionProperty::Color => MotionValue::Color(
        value(&style.color)
          .copied()
          .unwrap_or(Color::rgba(1.0, 1.0, 1.0, 1.0)),
      ),
      MotionProperty::BackgroundColor => MotionValue::Color(
        value(&style.background_color)
          .copied()
          .unwrap_or(Color::rgba(0.0, 0.0, 0.0, 0.0)),
      ),
      MotionProperty::BorderTopColor => MotionValue::Color(
        value(&style.border_top_color)
          .copied()
          .unwrap_or(Color::rgba(0.0, 0.0, 0.0, 0.0)),
      ),
      MotionProperty::BorderBottomColor => MotionValue::Color(
        value(&style.border_bottom_color)
          .copied()
          .unwrap_or(Color::rgba(0.0, 0.0, 0.0, 0.0)),
      ),
      MotionProperty::BorderLeftColor => MotionValue::Color(
        value(&style.border_left_color)
          .copied()
          .unwrap_or(Color::rgba(0.0, 0.0, 0.0, 0.0)),
      ),
      MotionProperty::BorderRightColor => MotionValue::Color(
        value(&style.border_right_color)
          .copied()
          .unwrap_or(Color::rgba(0.0, 0.0, 0.0, 0.0)),
      ),
      MotionProperty::Scale => {
        let scale = value(&style.scale).copied().unwrap_or(Scale::uniform(1.0));
        MotionValue::Vector2([scale.x, scale.y])
      }
      MotionProperty::ScaleX => MotionValue::Scalar(value(&style.scale).map_or(1.0, |v| v.x)),
      MotionProperty::ScaleY => MotionValue::Scalar(value(&style.scale).map_or(1.0, |v| v.y)),
      MotionProperty::Rotate => MotionValue::Angle(value(&style.rotate).map_or(0.0, |v| v.degrees)),
      MotionProperty::X => {
        MotionValue::Length(value(&style.translate).map_or(Length::Px(0.0), |v| v.x))
      }
      MotionProperty::Y => {
        MotionValue::Length(value(&style.translate).map_or(Length::Px(0.0), |v| v.y))
      }
      MotionProperty::Z => {
        MotionValue::Length(Length::Px(value(&style.translate).map_or(0.0, |v| v.z)))
      }
      _ => return None,
    })
  }

  /// Applies a host-local sample without an authored update or Rust render.
  /// Returns false for properties outside the inline style adapter.
  pub fn apply_motion_value(
    &mut self,
    host: ObjectId,
    property: MotionProperty,
    sample: &MotionValue,
  ) -> bool {
    let Some(element) = self.elements.get_mut(&host) else {
      return false;
    };
    let style = &mut element.element.visual_element_mut().style;
    match (property, sample) {
      (MotionProperty::Opacity, MotionValue::Scalar(v)) => style.opacity = set(FloatValue(*v)),
      (MotionProperty::FlexGrow, MotionValue::Scalar(v)) => style.flex_grow = set(FloatValue(*v)),
      (MotionProperty::FlexShrink, MotionValue::Scalar(v)) => {
        style.flex_shrink = set(FloatValue(*v))
      }
      (MotionProperty::FontSize, MotionValue::Length(v)) => style.font_size = set(*v),
      (MotionProperty::LetterSpacing, MotionValue::Length(v)) => style.letter_spacing = set(*v),
      (MotionProperty::WordSpacing, MotionValue::Length(v)) => style.word_spacing = set(*v),
      (MotionProperty::PaddingBottom, MotionValue::Length(v)) => style.padding_bottom = set(*v),
      (MotionProperty::PaddingLeft, MotionValue::Length(v)) => style.padding_left = set(*v),
      (MotionProperty::PaddingRight, MotionValue::Length(v)) => style.padding_right = set(*v),
      (MotionProperty::PaddingTop, MotionValue::Length(v)) => style.padding_top = set(*v),
      (MotionProperty::BorderBottomLeftRadius, MotionValue::Length(v)) => {
        style.border_bottom_left_radius = set(*v)
      }
      (MotionProperty::BorderBottomRightRadius, MotionValue::Length(v)) => {
        style.border_bottom_right_radius = set(*v)
      }
      (MotionProperty::BorderTopLeftRadius, MotionValue::Length(v)) => {
        style.border_top_left_radius = set(*v)
      }
      (MotionProperty::BorderTopRightRadius, MotionValue::Length(v)) => {
        style.border_top_right_radius = set(*v)
      }
      (MotionProperty::Width, MotionValue::Length(v)) => style.width = set(write_auto(*v)),
      (MotionProperty::Height, MotionValue::Length(v)) => style.height = set(write_auto(*v)),
      (MotionProperty::MinWidth, MotionValue::Length(v)) => style.min_width = set(write_auto(*v)),
      (MotionProperty::MinHeight, MotionValue::Length(v)) => style.min_height = set(write_auto(*v)),
      (MotionProperty::MaxWidth, MotionValue::Length(v)) => style.max_width = set(write_auto(*v)),
      (MotionProperty::MaxHeight, MotionValue::Length(v)) => style.max_height = set(write_auto(*v)),
      (MotionProperty::Top, MotionValue::Length(v)) => style.top = set(write_auto(*v)),
      (MotionProperty::Bottom, MotionValue::Length(v)) => style.bottom = set(write_auto(*v)),
      (MotionProperty::Left, MotionValue::Length(v)) => style.left = set(write_auto(*v)),
      (MotionProperty::Right, MotionValue::Length(v)) => style.right = set(write_auto(*v)),
      (MotionProperty::MarginTop, MotionValue::Length(v)) => style.margin_top = set(write_auto(*v)),
      (MotionProperty::MarginBottom, MotionValue::Length(v)) => {
        style.margin_bottom = set(write_auto(*v))
      }
      (MotionProperty::MarginLeft, MotionValue::Length(v)) => {
        style.margin_left = set(write_auto(*v))
      }
      (MotionProperty::MarginRight, MotionValue::Length(v)) => {
        style.margin_right = set(write_auto(*v))
      }
      (MotionProperty::FlexBasis, MotionValue::Length(v)) => style.flex_basis = set(write_auto(*v)),
      (MotionProperty::BorderTopWidth, MotionValue::Length(v)) => {
        style.border_top_width = set(FloatValue(v.components()[0]))
      }
      (MotionProperty::BorderBottomWidth, MotionValue::Length(v)) => {
        style.border_bottom_width = set(FloatValue(v.components()[0]))
      }
      (MotionProperty::BorderLeftWidth, MotionValue::Length(v)) => {
        style.border_left_width = set(FloatValue(v.components()[0]))
      }
      (MotionProperty::BorderRightWidth, MotionValue::Length(v)) => {
        style.border_right_width = set(FloatValue(v.components()[0]))
      }
      (MotionProperty::Color, MotionValue::Color(v)) => style.color = set(*v),
      (MotionProperty::BackgroundColor, MotionValue::Color(v)) => style.background_color = set(*v),
      (MotionProperty::BorderTopColor, MotionValue::Color(v)) => style.border_top_color = set(*v),
      (MotionProperty::BorderBottomColor, MotionValue::Color(v)) => {
        style.border_bottom_color = set(*v)
      }
      (MotionProperty::BorderLeftColor, MotionValue::Color(v)) => style.border_left_color = set(*v),
      (MotionProperty::BorderRightColor, MotionValue::Color(v)) => {
        style.border_right_color = set(*v)
      }
      (MotionProperty::Scale, MotionValue::Vector2(v)) => style.scale = set(Scale::new(v[0], v[1])),
      (MotionProperty::ScaleX, MotionValue::Scalar(v)) => {
        let previous = value(&style.scale).copied().unwrap_or(Scale::uniform(1.0));
        style.scale = set(Scale::new(*v, previous.y));
      }
      (MotionProperty::ScaleY, MotionValue::Scalar(v)) => {
        let previous = value(&style.scale).copied().unwrap_or(Scale::uniform(1.0));
        style.scale = set(Scale::new(previous.x, *v));
      }
      (MotionProperty::Rotate, MotionValue::Angle(v)) => style.rotate = set(Rotate::degrees(*v)),
      (MotionProperty::X | MotionProperty::Y | MotionProperty::Z, MotionValue::Length(v)) => {
        let mut current = value(&style.translate).copied().unwrap_or(Translate::new(
          Length::Px(0.0),
          Length::Px(0.0),
          0.0,
        ));
        match property {
          MotionProperty::X => current.x = *v,
          MotionProperty::Y => current.y = *v,
          _ => current.z = v.components()[0],
        }
        style.translate = set(current);
      }
      _ => return false,
    }
    true
  }
}

fn value<T>(property: &Prop<StyleValue<T>>) -> Option<&T> {
  match property {
    Prop::Set(StyleValue::Value(value)) => Some(value),
    _ => None,
  }
}
fn set<T>(value: T) -> Prop<StyleValue<T>> {
  Prop::Set(StyleValue::Value(value))
}
fn read_auto(property: &Prop<StyleValue<LengthOrAuto>>) -> Length {
  match value(property) {
    Some(LengthOrAuto::Px(v)) => Length::Px(*v),
    Some(LengthOrAuto::Percent(v)) => Length::Percent(*v),
    _ => Length::Px(0.0),
  }
}
fn write_auto(value: Length) -> LengthOrAuto {
  match value {
    Length::Px(v) => LengthOrAuto::Px(v),
    Length::Percent(v) => LengthOrAuto::Percent(v),
    Length::Calc { px, .. } => LengthOrAuto::Px(px),
  }
}
