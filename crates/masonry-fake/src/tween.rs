//! Instant tween endpoint selection.

use masonry::{Tween, TweenRepeat};

pub(crate) fn final_factor(tween: Tween) -> f64 {
    match tween.repeat {
        TweenRepeat::Once | TweenRepeat::Forever(_) => 1.0,
        TweenRepeat::Count {
            additional_traversals,
            mode,
        } => match mode {
            masonry::RepeatMode::Restart => 1.0,
            masonry::RepeatMode::PingPong => {
                if additional_traversals % 2 == 1 {
                    0.0
                } else {
                    1.0
                }
            }
        },
    }
}

pub(crate) fn scalar(start: f64, target: f64, tween: Tween) -> f64 {
    start + (target - start) * final_factor(tween)
}

pub(crate) fn vector(
    start: masonry::Vector3,
    target: masonry::Vector3,
    tween: Tween,
) -> masonry::Vector3 {
    masonry::Vector3::new(
        scalar(start.x, target.x, tween),
        scalar(start.y, target.y, tween),
        scalar(start.z, target.z, tween),
    )
}

pub(crate) fn rgb(
    start: masonry::RgbColor,
    target: masonry::RgbColor,
    tween: Tween,
) -> masonry::RgbColor {
    masonry::RgbColor {
        r: scalar(start.r, target.r, tween),
        g: scalar(start.g, target.g, tween),
        b: scalar(start.b, target.b, tween),
    }
}

pub(crate) fn color(start: masonry::Color, target: masonry::Color, tween: Tween) -> masonry::Color {
    masonry::Color {
        r: scalar(start.r, target.r, tween),
        g: scalar(start.g, target.g, tween),
        b: scalar(start.b, target.b, tween),
        a: scalar(start.a, target.a, tween),
    }
}
