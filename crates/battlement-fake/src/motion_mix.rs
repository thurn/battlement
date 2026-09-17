//! Typed Motion interpolation with the native host's compatibility rules.

use battlement::{
  Color, FilterFunction, FilterList, Gradient, GradientStop, Length, MotionValue, Shadow,
  TransformOperation,
};

pub(crate) fn mix(left: &MotionValue, right: &MotionValue, progress: f64) -> MotionValue {
  let p = progress.clamp(0.0, 1.0);
  let compatible = match (left, right) {
    (MotionValue::Scalar(a), MotionValue::Scalar(b)) => {
      Some(MotionValue::Scalar(number(*a, *b, p)))
    }
    (MotionValue::Angle(a), MotionValue::Angle(b)) => Some(MotionValue::Angle(number(*a, *b, p))),
    (MotionValue::Length(a), MotionValue::Length(b)) => {
      Some(MotionValue::Length(length(*a, *b, p)))
    }
    (MotionValue::Color(a), MotionValue::Color(b)) => Some(MotionValue::Color(color(*a, *b, p))),
    (MotionValue::Vector2(a), MotionValue::Vector2(b)) => {
      Some(MotionValue::Vector2(numbers(*a, *b, p)))
    }
    (MotionValue::Vector3(a), MotionValue::Vector3(b)) => {
      Some(MotionValue::Vector3(numbers(*a, *b, p)))
    }
    (MotionValue::TransformList(a), MotionValue::TransformList(b)) if a.len() == b.len() => a
      .iter()
      .zip(b)
      .map(|(a, b)| operation(a, b, p))
      .collect::<Option<Vec<_>>>()
      .map(MotionValue::TransformList),
    (MotionValue::FilterList(a), MotionValue::FilterList(b)) if !b.as_slice().is_empty() => b
      .as_slice()
      .iter()
      .enumerate()
      .map(|(i, b)| filter(a.as_slice().get(i), b, p))
      .collect::<Option<Vec<_>>>()
      .map(|items| MotionValue::FilterList(FilterList::new(items))),
    (MotionValue::ShadowList(a), MotionValue::ShadowList(b)) if a.len() == b.len() => a
      .iter()
      .zip(b)
      .map(|(a, b)| (a.inset == b.inset).then(|| shadow(*a, *b, p)))
      .collect::<Option<Vec<_>>>()
      .map(MotionValue::ShadowList),
    (MotionValue::Gradient(a), MotionValue::Gradient(b)) => {
      gradient(a, b, p).map(MotionValue::Gradient)
    }
    (MotionValue::ClipInset(a), MotionValue::ClipInset(b)) => {
      Some(MotionValue::ClipInset(std::array::from_fn(|i| {
        length(a[i], b[i], p)
      })))
    }
    (MotionValue::ClipPolygon(a), MotionValue::ClipPolygon(b)) if a.len() == b.len() => {
      Some(MotionValue::ClipPolygon(
        a.iter()
          .zip(b)
          .map(|(a, b)| std::array::from_fn(|i| length(a[i], b[i], p)))
          .collect(),
      ))
    }
    _ => None,
  };
  compatible.unwrap_or_else(|| if p < 0.5 { left.clone() } else { right.clone() })
}

fn lerp(a: f64, b: f64, p: f64) -> f64 {
  a + (b - a) * p
}
fn number(a: f32, b: f32, p: f64) -> f32 {
  lerp(f64::from(a), f64::from(b), p) as f32
}
fn numbers<const N: usize>(a: [f32; N], b: [f32; N], p: f64) -> [f32; N] {
  std::array::from_fn(|i| number(a[i], b[i], p))
}
fn length(a: Length, b: Length, p: f64) -> Length {
  let a = a.components();
  let b = b.components();
  Length::calc(number(a[0], b[0], p), number(a[1], b[1], p))
}
fn color(a: Color, b: Color, p: f64) -> Color {
  Color::rgba(
    lerp(a.r * a.r, b.r * b.r, p).max(0.0).sqrt(),
    lerp(a.g * a.g, b.g * b.g, p).max(0.0).sqrt(),
    lerp(a.b * a.b, b.b * b.b, p).max(0.0).sqrt(),
    lerp(a.a, b.a, p),
  )
}
fn operation(a: &TransformOperation, b: &TransformOperation, p: f64) -> Option<TransformOperation> {
  Some(match (a, b) {
    (TransformOperation::Translate(a), TransformOperation::Translate(b)) => {
      TransformOperation::Translate(std::array::from_fn(|i| length(a[i], b[i], p)))
    }
    (TransformOperation::Rotate(a), TransformOperation::Rotate(b)) => {
      TransformOperation::Rotate(numbers(*a, *b, p))
    }
    (TransformOperation::Scale(a), TransformOperation::Scale(b)) => {
      TransformOperation::Scale(numbers(*a, *b, p))
    }
    (TransformOperation::Skew(a), TransformOperation::Skew(b)) => {
      TransformOperation::Skew(numbers(*a, *b, p))
    }
    _ => return None,
  })
}
fn shadow(a: Shadow, b: Shadow, p: f64) -> Shadow {
  Shadow {
    x: number(a.x, b.x, p),
    y: number(a.y, b.y, p),
    blur: number(a.blur, b.blur, p),
    spread: number(a.spread, b.spread, p),
    color: color(a.color, b.color, p),
    inset: b.inset,
  }
}
fn filter(a: Option<&FilterFunction>, b: &FilterFunction, p: f64) -> Option<FilterFunction> {
  Some(match (a, b) {
    (Some(FilterFunction::Brightness(a)), FilterFunction::Brightness(b)) => {
      FilterFunction::Brightness(number(*a, *b, p))
    }
    (None, FilterFunction::Brightness(b)) => FilterFunction::Brightness(number(0.0, *b, p)),
    (Some(FilterFunction::DropShadow(a)), FilterFunction::DropShadow(b)) => {
      FilterFunction::DropShadow(shadow(*a, *b, p))
    }
    _ => return None,
  })
}
fn stops(a: &[GradientStop], b: &[GradientStop], p: f64) -> Option<Vec<GradientStop>> {
  (a.len() == b.len()).then(|| {
    a.iter()
      .zip(b)
      .map(|(a, b)| GradientStop {
        color: color(a.color, b.color, p),
        position: number(a.position, b.position, p),
      })
      .collect()
  })
}
fn gradient(a: &Gradient, b: &Gradient, p: f64) -> Option<Gradient> {
  match (a, b) {
    (
      Gradient::Linear {
        angle: a,
        stops: sa,
      },
      Gradient::Linear {
        angle: b,
        stops: sb,
      },
    ) => Some(Gradient::Linear {
      angle: number(*a, *b, p),
      stops: stops(sa, sb, p)?,
    }),
    (
      Gradient::Radial {
        center: a,
        radius: ra,
        stops: sa,
      },
      Gradient::Radial {
        center: b,
        radius: rb,
        stops: sb,
      },
    ) => Some(Gradient::Radial {
      center: numbers(*a, *b, p),
      radius: numbers(*ra, *rb, p),
      stops: stops(sa, sb, p)?,
    }),
    _ => None,
  }
}
