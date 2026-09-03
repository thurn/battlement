use serde::{Deserialize, Serialize};

/// Runtime value shape accepted by an animation property.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionValueKind {
  /// One finite floating-point channel.
  Scalar,
  /// One finite pixel/percentage/calc length.
  Length,
  /// Four linear RGBA channels.
  Color,
  /// A two-dimensional vector.
  Vector2,
  /// A three-dimensional vector.
  Vector3,
  /// A finite angle in degrees.
  Angle,
  /// An ordered transform operation list.
  TransformList,
  /// A compatible filter operation list.
  FilterList,
  /// A compatible shadow list.
  ShadowList,
  /// A compatible gradient.
  Gradient,
  /// A rectangular inset clip.
  ClipInset,
  /// A polygon clip with a stable vertex count.
  ClipPolygon,
  /// A discrete enum, asset, text, or opaque protocol value.
  Discrete,
}

/// Canonical interpolation behavior for one property.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum InterpolationCategory {
  /// Interpolates each finite numeric channel linearly after easing.
  Numeric,
  /// Interpolates pixel and percentage components independently.
  Length,
  /// Uses Motion 13.1.1's square-root RGB mixer and linear alpha.
  Color,
  /// Interpolates compatible structured shapes and falls back to discrete segments.
  Structured,
  /// Switches at the property-specific discrete boundary.
  Discrete,
}

/// Additive composition available to a property.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AdditiveRule {
  /// Additive animation is invalid.
  None,
  /// Scalar/vector channels add with zero as identity.
  Sum,
  /// Scale channels multiply with one as identity.
  Multiply,
  /// Compatible transform operations compose component-by-component.
  Transform,
}

/// Reference box used to resolve percentage channels.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PercentageReference {
  /// The value has no percentage representation.
  None,
  /// Resolves against the containing block's inline axis.
  ContainingWidth,
  /// Resolves against the containing block's block axis.
  ContainingHeight,
  /// Resolves against the animated host's border-box width.
  SelfWidth,
  /// Resolves against the animated host's border-box height.
  SelfHeight,
}

/// Complete generated metadata for one animation property.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct MotionPropertyMetadata {
  /// Stable lower-snake-case wire identity.
  pub wire_name: &'static str,
  /// Runtime protocol value shape.
  pub value_kind: MotionValueKind,
  /// Canonical unit used for scalar channels.
  pub canonical_unit: &'static str,
  /// Canonical initial value used beneath authored layers.
  pub initial_value: &'static str,
  /// Interpolation algorithm.
  pub interpolation: InterpolationCategory,
  /// Percentage reference box.
  pub percentage_reference: PercentageReference,
  /// Additive composition behavior.
  pub additive: AdditiveRule,
  /// Unity writer identity generated for this property.
  pub unity_writer: &'static str,
}

macro_rules! properties {
  ($($variant:ident => ($wire:literal, $kind:ident, $unit:literal, $initial:literal, $mix:ident, $reference:ident, $add:ident)),+ $(,)?) => {
    /// Exhaustive animation-property catalog used by Rust validation and Unity writers.
    #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
    pub enum MotionProperty {
      $(
        #[doc = concat!("The `", $wire, "` animation property.")]
        $variant,
      )+
    }

    impl MotionProperty {
      /// Every catalog entry in stable wire order.
      pub const ALL: &'static [Self] = &[$(Self::$variant),+];

      /// Returns the canonical metadata used to validate and lower this property.
      #[must_use]
      pub const fn metadata(self) -> MotionPropertyMetadata {
        match self {
          $(Self::$variant => MotionPropertyMetadata {
            wire_name: $wire,
            value_kind: MotionValueKind::$kind,
            canonical_unit: $unit,
            initial_value: $initial,
            interpolation: InterpolationCategory::$mix,
            percentage_reference: PercentageReference::$reference,
            additive: AdditiveRule::$add,
            unity_writer: concat!("Write", stringify!($variant)),
          }),+
        }
      }
    }
  };
}

properties! {
  AlignContent => ("align_content", Discrete, "keyword", "stretch", Discrete, None, None),
  AlignItems => ("align_items", Discrete, "keyword", "stretch", Discrete, None, None),
  AlignSelf => ("align_self", Discrete, "keyword", "auto", Discrete, None, None),
  AspectRatio => ("aspect_ratio", Scalar, "ratio", "auto", Numeric, None, None),
  BackgroundColor => ("background_color", Color, "linear-rgba", "transparent", Color, None, None),
  BackgroundImage => ("background_image", Discrete, "asset", "none", Discrete, None, None),
  BackgroundPositionX => ("background_position_x", Length, "calc", "0%", Length, SelfWidth, Sum),
  BackgroundPositionY => ("background_position_y", Length, "calc", "0%", Length, SelfHeight, Sum),
  BackgroundRepeat => ("background_repeat", Discrete, "keyword", "repeat", Discrete, None, None),
  BackgroundSize => ("background_size", Vector2, "calc", "auto", Structured, SelfWidth, None),
  BorderBottomColor => ("border_bottom_color", Color, "linear-rgba", "transparent", Color, None, None),
  BorderBottomLeftRadius => ("border_bottom_left_radius", Length, "calc", "0px", Length, SelfWidth, Sum),
  BorderBottomRightRadius => ("border_bottom_right_radius", Length, "calc", "0px", Length, SelfWidth, Sum),
  BorderBottomWidth => ("border_bottom_width", Length, "calc", "0px", Length, ContainingWidth, Sum),
  BorderLeftColor => ("border_left_color", Color, "linear-rgba", "transparent", Color, None, None),
  BorderLeftWidth => ("border_left_width", Length, "calc", "0px", Length, ContainingWidth, Sum),
  BorderRightColor => ("border_right_color", Color, "linear-rgba", "transparent", Color, None, None),
  BorderRightWidth => ("border_right_width", Length, "calc", "0px", Length, ContainingWidth, Sum),
  BorderTopColor => ("border_top_color", Color, "linear-rgba", "transparent", Color, None, None),
  BorderTopLeftRadius => ("border_top_left_radius", Length, "calc", "0px", Length, SelfWidth, Sum),
  BorderTopRightRadius => ("border_top_right_radius", Length, "calc", "0px", Length, SelfWidth, Sum),
  BorderTopWidth => ("border_top_width", Length, "calc", "0px", Length, ContainingWidth, Sum),
  Bottom => ("bottom", Length, "calc", "auto", Length, ContainingHeight, Sum),
  Color => ("color", Color, "linear-rgba", "white", Color, None, None),
  Cursor => ("cursor", Discrete, "cursor", "auto", Discrete, None, None),
  Display => ("display", Discrete, "keyword", "flex", Discrete, None, None),
  Filter => ("filter", FilterList, "filter-list", "none", Structured, None, None),
  FlexBasis => ("flex_basis", Length, "calc", "auto", Length, ContainingWidth, Sum),
  FlexDirection => ("flex_direction", Discrete, "keyword", "column", Discrete, None, None),
  FlexGrow => ("flex_grow", Scalar, "number", "0", Numeric, None, Sum),
  FlexShrink => ("flex_shrink", Scalar, "number", "1", Numeric, None, Sum),
  FlexWrap => ("flex_wrap", Discrete, "keyword", "nowrap", Discrete, None, None),
  FontSize => ("font_size", Length, "calc", "0px", Length, ContainingWidth, Sum),
  Height => ("height", Length, "calc", "auto", Length, ContainingHeight, Sum),
  JustifyContent => ("justify_content", Discrete, "keyword", "flex-start", Discrete, None, None),
  Left => ("left", Length, "calc", "auto", Length, ContainingWidth, Sum),
  LetterSpacing => ("letter_spacing", Length, "calc", "0px", Length, ContainingWidth, Sum),
  MarginBottom => ("margin_bottom", Length, "calc", "0px", Length, ContainingWidth, Sum),
  MarginLeft => ("margin_left", Length, "calc", "0px", Length, ContainingWidth, Sum),
  MarginRight => ("margin_right", Length, "calc", "0px", Length, ContainingWidth, Sum),
  MarginTop => ("margin_top", Length, "calc", "0px", Length, ContainingWidth, Sum),
  MaxHeight => ("max_height", Length, "calc", "none", Length, ContainingHeight, Sum),
  MaxWidth => ("max_width", Length, "calc", "none", Length, ContainingWidth, Sum),
  MinHeight => ("min_height", Length, "calc", "auto", Length, ContainingHeight, Sum),
  MinWidth => ("min_width", Length, "calc", "auto", Length, ContainingWidth, Sum),
  Opacity => ("opacity", Scalar, "number", "1", Numeric, None, Sum),
  Overflow => ("overflow", Discrete, "keyword", "visible", Discrete, None, None),
  PaddingBottom => ("padding_bottom", Length, "calc", "0px", Length, ContainingWidth, Sum),
  PaddingLeft => ("padding_left", Length, "calc", "0px", Length, ContainingWidth, Sum),
  PaddingRight => ("padding_right", Length, "calc", "0px", Length, ContainingWidth, Sum),
  PaddingTop => ("padding_top", Length, "calc", "0px", Length, ContainingWidth, Sum),
  Position => ("position", Discrete, "keyword", "relative", Discrete, None, None),
  Right => ("right", Length, "calc", "auto", Length, ContainingWidth, Sum),
  Rotate => ("rotate", Angle, "degrees", "0deg", Numeric, None, Sum),
  Scale => ("scale", Vector2, "number", "1 1", Numeric, None, Multiply),
  TextOverflow => ("text_overflow", Discrete, "keyword", "clip", Discrete, None, None),
  TextShadow => ("text_shadow", ShadowList, "shadow-list", "none", Structured, None, None),
  Top => ("top", Length, "calc", "auto", Length, ContainingHeight, Sum),
  TransformOrigin => ("transform_origin", Vector3, "calc", "50% 50% 0px", Structured, SelfWidth, Sum),
  Translate => ("translate", Vector3, "calc", "0px 0px 0px", Structured, SelfWidth, Sum),
  UnityBackgroundImageTintColor => ("unity_background_image_tint_color", Color, "linear-rgba", "white", Color, None, None),
  UnityEditorTextRenderingMode => ("unity_editor_text_rendering_mode", Discrete, "keyword", "default", Discrete, None, None),
  UnityFontDefinition => ("unity_font_definition", Discrete, "asset", "none", Discrete, None, None),
  UnityFontStyleAndWeight => ("unity_font_style_and_weight", Discrete, "keyword", "normal", Discrete, None, None),
  UnityMaterial => ("unity_material", Discrete, "asset", "none", Discrete, None, None),
  UnityOverflowClipBox => ("unity_overflow_clip_box", Discrete, "keyword", "padding-box", Discrete, None, None),
  UnityParagraphSpacing => ("unity_paragraph_spacing", Length, "calc", "0px", Length, ContainingWidth, Sum),
  UnitySliceBottom => ("unity_slice_bottom", Scalar, "pixel", "0", Numeric, None, Sum),
  UnitySliceLeft => ("unity_slice_left", Scalar, "pixel", "0", Numeric, None, Sum),
  UnitySliceRight => ("unity_slice_right", Scalar, "pixel", "0", Numeric, None, Sum),
  UnitySliceScale => ("unity_slice_scale", Scalar, "number", "1", Numeric, None, Sum),
  UnitySliceTop => ("unity_slice_top", Scalar, "pixel", "0", Numeric, None, Sum),
  UnitySliceType => ("unity_slice_type", Discrete, "keyword", "sliced", Discrete, None, None),
  UnityTextAlign => ("unity_text_align", Discrete, "keyword", "upper-left", Discrete, None, None),
  UnityTextAutoSize => ("unity_text_auto_size", Discrete, "keyword", "none", Discrete, None, None),
  UnityTextGenerator => ("unity_text_generator", Discrete, "keyword", "standard", Discrete, None, None),
  UnityTextOutlineColor => ("unity_text_outline_color", Color, "linear-rgba", "transparent", Color, None, None),
  UnityTextOutlineWidth => ("unity_text_outline_width", Scalar, "pixel", "0", Numeric, None, Sum),
  UnityTextOverflowPosition => ("unity_text_overflow_position", Discrete, "keyword", "end", Discrete, None, None),
  Visibility => ("visibility", Discrete, "keyword", "visible", Discrete, None, None),
  WhiteSpace => ("white_space", Discrete, "keyword", "normal", Discrete, None, None),
  Width => ("width", Length, "calc", "auto", Length, ContainingWidth, Sum),
  WordSpacing => ("word_spacing", Length, "calc", "0px", Length, ContainingWidth, Sum),
  X => ("x", Length, "calc", "0px", Length, SelfWidth, Sum),
  Y => ("y", Length, "calc", "0px", Length, SelfHeight, Sum),
  Z => ("z", Length, "pixel", "0px", Length, None, Sum),
  RotateX => ("rotate_x", Angle, "degrees", "0deg", Numeric, None, Sum),
  RotateY => ("rotate_y", Angle, "degrees", "0deg", Numeric, None, Sum),
  ScaleX => ("scale_x", Scalar, "number", "1", Numeric, None, Multiply),
  ScaleY => ("scale_y", Scalar, "number", "1", Numeric, None, Multiply),
  SkewX => ("skew_x", Angle, "degrees", "0deg", Numeric, None, Sum),
  SkewY => ("skew_y", Angle, "degrees", "0deg", Numeric, None, Sum),
  TransformList => ("transform_list", TransformList, "transform-list", "none", Structured, SelfWidth, Transform),
  BackgroundGradient => ("background_gradient", Gradient, "gradient", "none", Structured, SelfWidth, None),
  BoxShadow => ("box_shadow", ShadowList, "shadow-list", "none", Structured, None, None),
  ClipInset => ("clip_inset", ClipInset, "calc", "0px", Structured, SelfWidth, None),
  ClipPolygon => ("clip_polygon", ClipPolygon, "calc", "none", Structured, SelfWidth, None),
  Mask => ("mask", Discrete, "asset", "none", Discrete, None, None),
  Layout => ("layout", Vector3, "projection", "identity", Numeric, None, None),
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::MotionProperty;

  #[test]
  fn wire_and_writer_identities_are_exhaustive_and_unique() {
    let mut wire_names = HashSet::new();
    let mut writers = HashSet::new();
    for property in MotionProperty::ALL {
      let metadata = property.metadata();
      assert!(wire_names.insert(metadata.wire_name));
      assert!(writers.insert(metadata.unity_writer));
      assert!(!metadata.canonical_unit.is_empty());
      assert!(!metadata.initial_value.is_empty());
    }
  }
}
